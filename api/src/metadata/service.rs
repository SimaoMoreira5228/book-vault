use sea_orm::{ColumnTrait, EntityTrait, ExprTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::AppError;
use crate::db::entities::prelude::*;
use crate::db::entities::{book_metadata, books, metadata_cache};
use crate::metadata::amazon::AmazonProvider;
use crate::metadata::goodreads::GoodReadsProvider;
use crate::metadata::google_books::GoogleBooksProvider;
use crate::metadata::openlibrary::OpenLibraryProvider;
use crate::metadata::{MetadataField, MetadataProvider, MetadataQuery, ProspectiveMetadata};

const CACHE_TTL_HOURS: i64 = 24;

pub struct MetadataService {
	providers: Vec<Box<dyn MetadataProvider>>,
}

impl MetadataService {
	pub fn new() -> Self {
		Self {
			providers: vec![
				Box::new(AmazonProvider::new()),
				Box::new(OpenLibraryProvider::new()),
				Box::new(GoogleBooksProvider::new()),
				Box::new(GoodReadsProvider::new()),
			],
		}
	}

	pub async fn get_metadata(
		&self,
		db: &sea_orm::DatabaseConnection,
		book_id: Uuid,
	) -> Result<serde_json::Value, AppError> {
		let book = Books::find_by_id(book_id)
			.one(db)
			.await?
			.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

		let meta = BookMetadata::find()
			.filter(book_metadata::Column::BookId.eq(book_id))
			.one(db)
			.await?;

		Ok(serde_json::json!({
			"book_id": book_id,
			"title": book.title,
			"author": book.author,
			"isbn": book.isbn,
			"publisher": book.publisher,
			"page_count": book.page_count,
			"provider_ids": meta.as_ref().map(|m| &m.provider_ids).unwrap_or(&serde_json::Value::Null),
			"locked_fields": meta.as_ref().map(|m| &m.locked_fields).unwrap_or(&serde_json::Value::Null),
			"cached_metadata": meta.as_ref().map(|m| &m.cached_metadata).unwrap_or(&serde_json::Value::Null),
			"last_refreshed_at": meta.as_ref().and_then(|m| m.last_refreshed_at).map(|t| t.to_string()),
		}))
	}

	pub async fn search_candidates(
		&self,
		db: &sea_orm::DatabaseConnection,
		_book_id: Uuid,
		query: &MetadataQuery,
	) -> Result<Vec<ProspectiveMetadata>, AppError> {
		let mut all_results = Vec::new();
		let mut futures = Vec::new();

		for provider in &self.providers {
			let cached = check_cache(db, provider.id(), query).await?;
			if let Some(results) = cached {
				all_results.extend(results);
			} else {
				futures.push(provider.search(query));
			}
		}

		let live_results: Vec<Vec<ProspectiveMetadata>> = futures::future::join_all(futures)
			.await
			.into_iter()
			.filter_map(|r| r.ok())
			.collect();

		for results in &live_results {
			if !results.is_empty() {
				let provider_id = results[0].provider.clone();
				update_cache(db, &provider_id, query, results).await?;
			}
			all_results.extend(results.iter().cloned());
		}

		Ok(all_results)
	}

	pub async fn confirm_match(
		&self,
		db: &sea_orm::DatabaseConnection,
		book_id: Uuid,
		candidate: &ProspectiveMetadata,
	) -> Result<serde_json::Value, AppError> {
		let book = Books::find_by_id(book_id)
			.one(db)
			.await?
			.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

		let existing_meta = BookMetadata::find()
			.filter(book_metadata::Column::BookId.eq(book_id))
			.one(db)
			.await?;

		let locked_fields: Vec<String> = existing_meta
			.as_ref()
			.and_then(|m| m.locked_fields.as_array())
			.map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
			.unwrap_or_default();

		let mut provider_ids = existing_meta
			.as_ref()
			.and_then(|m| m.provider_ids.as_object())
			.cloned()
			.unwrap_or_default();

		provider_ids.insert(
			candidate.provider.clone(),
			serde_json::Value::String(candidate.provider_id.clone()),
		);

		let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
		let mut book_active: books::ActiveModel = book.into();

		if !locked_fields.contains(&"title".to_string()) {
			if let Some(ref title) = candidate.title {
				book_active.title = Set(title.clone());
			}
		}
		if !locked_fields.contains(&"author".to_string()) && !candidate.authors.is_empty() {
			book_active.author = Set(Some(candidate.authors.join(", ")));
		}
		if !locked_fields.contains(&"isbn".to_string()) {
			if let Some(ref isbn13) = candidate.isbn13 {
				book_active.isbn = Set(Some(isbn13.clone()));
			} else if let Some(ref isbn10) = candidate.isbn10 {
				book_active.isbn = Set(Some(isbn10.clone()));
			}
		}
		if !locked_fields.contains(&"publisher".to_string()) {
			book_active.publisher = Set(candidate.publisher.clone());
		}
		if !locked_fields.contains(&"page_count".to_string()) {
			book_active.page_count = Set(candidate.page_count.map(|c| c as i64));
		}

		book_active.updated_at = Set(now);
		Books::update(book_active).exec(db).await?;

		let meta_payload =
			serde_json::to_value(candidate).map_err(|e| AppError::Internal(format!("Serialization error: {e}")))?;

		if let Some(existing) = existing_meta {
			let mut active: book_metadata::ActiveModel = existing.into();
			active.provider_ids = Set(serde_json::Value::Object(provider_ids));
			active.cached_metadata = Set(meta_payload);
			active.last_refreshed_at = Set(Some(now));
			active.updated_at = Set(now);
			BookMetadata::update(active).exec(db).await?;
		} else {
			BookMetadata::insert(book_metadata::ActiveModel {
				id: Set(Uuid::now_v7()),
				book_id: Set(book_id),
				provider_ids: Set(serde_json::Value::Object(provider_ids)),
				locked_fields: Set(serde_json::Value::Array(vec![])),
				cached_metadata: Set(meta_payload),
				last_refreshed_at: Set(Some(now)),
				created_at: Set(now),
				updated_at: Set(now),
			})
			.exec(db)
			.await?;
		}

		self.get_metadata(db, book_id).await
	}

	pub async fn refresh_metadata(
		&self,
		db: &sea_orm::DatabaseConnection,
		book_id: Uuid,
	) -> Result<serde_json::Value, AppError> {
		let book = Books::find_by_id(book_id)
			.one(db)
			.await?
			.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

		let query = MetadataQuery {
			title: Some(book.title.clone()),
			author: book.author.clone(),
			isbn: book.isbn.clone(),
		};

		let mut all_results = Vec::new();
		for provider in &self.providers {
			if let Ok(results) = provider.search(&query).await {
				all_results.extend(results);
			}
		}

		if all_results.is_empty() {
			return Err(AppError::NotFound("No metadata found from any provider".into()));
		}

		let merged = merge_candidates(&all_results);

		let existing_meta = BookMetadata::find()
			.filter(book_metadata::Column::BookId.eq(book_id))
			.one(db)
			.await?;

		let locked_fields: Vec<String> = existing_meta
			.as_ref()
			.and_then(|m| m.locked_fields.as_array())
			.map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
			.unwrap_or_default();

		let mut provider_ids = existing_meta
			.as_ref()
			.and_then(|m| m.provider_ids.as_object())
			.cloned()
			.unwrap_or_default();

		for result in &all_results {
			if !result.provider_id.is_empty() {
				provider_ids.insert(result.provider.clone(), serde_json::Value::String(result.provider_id.clone()));
			}
		}

		let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
		let mut book_active: books::ActiveModel = book.into();

		if !locked_fields.contains(&"title".to_string()) {
			if let Some(ref title) = merged.title {
				book_active.title = Set(title.clone());
			}
		}
		if !locked_fields.contains(&"author".to_string()) && !merged.authors.is_empty() {
			book_active.author = Set(Some(merged.authors.join(", ")));
		}
		if !locked_fields.contains(&"isbn".to_string()) {
			if let Some(ref isbn13) = merged.isbn13 {
				book_active.isbn = Set(Some(isbn13.clone()));
			} else if let Some(ref isbn10) = merged.isbn10 {
				book_active.isbn = Set(Some(isbn10.clone()));
			}
		}
		if !locked_fields.contains(&"publisher".to_string()) {
			book_active.publisher = Set(merged.publisher.clone());
		}
		if !locked_fields.contains(&"page_count".to_string()) {
			book_active.page_count = Set(merged.page_count.map(|c| c as i64));
		}

		book_active.updated_at = Set(now);
		Books::update(book_active).exec(db).await?;

		let meta_payload =
			serde_json::to_value(&merged).map_err(|e| AppError::Internal(format!("Serialization error: {e}")))?;

		if let Some(existing) = existing_meta {
			let mut active: book_metadata::ActiveModel = existing.into();
			active.provider_ids = Set(serde_json::Value::Object(provider_ids));
			active.cached_metadata = Set(meta_payload);
			active.last_refreshed_at = Set(Some(now));
			active.updated_at = Set(now);
			BookMetadata::update(active).exec(db).await?;
		} else {
			BookMetadata::insert(book_metadata::ActiveModel {
				id: Set(Uuid::now_v7()),
				book_id: Set(book_id),
				provider_ids: Set(serde_json::Value::Object(provider_ids)),
				locked_fields: Set(serde_json::Value::Array(vec![])),
				cached_metadata: Set(meta_payload),
				last_refreshed_at: Set(Some(now)),
				created_at: Set(now),
				updated_at: Set(now),
			})
			.exec(db)
			.await?;
		}

		self.get_metadata(db, book_id).await
	}

	pub async fn lock_field(
		&self,
		db: &sea_orm::DatabaseConnection,
		book_id: Uuid,
		field: MetadataField,
	) -> Result<serde_json::Value, AppError> {
		self.toggle_lock(db, book_id, field, true).await
	}

	pub async fn unlock_field(
		&self,
		db: &sea_orm::DatabaseConnection,
		book_id: Uuid,
		field: MetadataField,
	) -> Result<serde_json::Value, AppError> {
		self.toggle_lock(db, book_id, field, false).await
	}

	async fn toggle_lock(
		&self,
		db: &sea_orm::DatabaseConnection,
		book_id: Uuid,
		field: MetadataField,
		lock: bool,
	) -> Result<serde_json::Value, AppError> {
		let field_str = field.to_string();
		let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

		let existing = BookMetadata::find()
			.filter(book_metadata::Column::BookId.eq(book_id))
			.one(db)
			.await?;

		if let Some(meta) = existing {
			let mut locked: Vec<String> = meta
				.locked_fields
				.as_array()
				.map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
				.unwrap_or_default();

			if lock {
				if !locked.contains(&field_str) {
					locked.push(field_str);
				}
			} else {
				locked.retain(|f| f != &field_str);
			}

			let mut active: book_metadata::ActiveModel = meta.into();
			active.locked_fields = Set(serde_json::Value::Array(
				locked.into_iter().map(serde_json::Value::String).collect(),
			));
			active.updated_at = Set(now);
			BookMetadata::update(active).exec(db).await?;
		} else {
			let locked = if lock {
				vec![serde_json::Value::String(field_str)]
			} else {
				vec![]
			};

			BookMetadata::insert(book_metadata::ActiveModel {
				id: Set(Uuid::now_v7()),
				book_id: Set(book_id),
				provider_ids: Set(serde_json::Value::Object(serde_json::Map::new())),
				locked_fields: Set(serde_json::Value::Array(locked)),
				cached_metadata: Set(serde_json::Value::Null),
				last_refreshed_at: Set(None),
				created_at: Set(now),
				updated_at: Set(now),
			})
			.exec(db)
			.await?;
		}

		self.get_metadata(db, book_id).await
	}
}

fn merge_candidates(candidates: &[ProspectiveMetadata]) -> ProspectiveMetadata {
	let mut merged = candidates[0].clone();

	for c in &candidates[1..] {
		if merged.title.is_none() && c.title.is_some() {
			merged.title = c.title.clone();
		}
		if merged.description.is_none() && c.description.is_some() {
			merged.description = c.description.clone();
		}
		if merged.isbn13.is_none() && c.isbn13.is_some() {
			merged.isbn13 = c.isbn13.clone();
		}
		if merged.isbn10.is_none() && c.isbn10.is_some() {
			merged.isbn10 = c.isbn10.clone();
		}
		if merged.page_count.is_none() && c.page_count.is_some() {
			merged.page_count = c.page_count;
		}
		if merged.cover_url.is_none() && c.cover_url.is_some() {
			merged.cover_url = c.cover_url.clone();
		}
		if merged.published_date.is_none() && c.published_date.is_some() {
			merged.published_date = c.published_date.clone();
		}
		if merged.publisher.is_none() && c.publisher.is_some() {
			merged.publisher = c.publisher.clone();
		}
		if merged.rating.is_none() && c.rating.is_some() {
			merged.rating = c.rating;
		}
		if merged.subtitle.is_none() && c.subtitle.is_some() {
			merged.subtitle = c.subtitle.clone();
		}

		for g in &c.genres {
			if !merged.genres.contains(g) {
				merged.genres.push(g.clone());
			}
		}
		for a in &c.authors {
			if !merged.authors.contains(a) {
				merged.authors.push(a.clone());
			}
		}
	}

	merged
}

fn query_cache_key(provider: &str, query: &MetadataQuery) -> String {
	let input = serde_json::json!({
		"provider": provider,
		"title": query.title,
		"author": query.author,
		"isbn": query.isbn,
	});
	blake3::hash(input.to_string().as_bytes()).to_hex().to_string()
}

async fn check_cache(
	db: &sea_orm::DatabaseConnection,
	provider: &str,
	query: &MetadataQuery,
) -> Result<Option<Vec<ProspectiveMetadata>>, AppError> {
	let hash = query_cache_key(provider, query);

	let entry = MetadataCache::find()
		.filter(
			metadata_cache::Column::Provider
				.eq(provider)
				.and(metadata_cache::Column::QueryHash.eq(hash)),
		)
		.one(db)
		.await?;

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	match entry {
		Some(cache) if cache.expires_at > now => {
			let results: Vec<ProspectiveMetadata> = serde_json::from_value(cache.response)
				.map_err(|e| AppError::Internal(format!("Cache deserialize error: {e}")))?;
			Ok(Some(results))
		}
		_ => Ok(None),
	}
}

async fn update_cache(
	db: &sea_orm::DatabaseConnection,
	provider: &str,
	query: &MetadataQuery,
	results: &[ProspectiveMetadata],
) -> Result<(), AppError> {
	let hash = query_cache_key(provider, query);
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let expires = now + chrono::Duration::hours(CACHE_TTL_HOURS);

	let payload = serde_json::to_value(results).map_err(|e| AppError::Internal(format!("Cache serialize error: {e}")))?;

	MetadataCache::insert(metadata_cache::ActiveModel {
		id: Set(Uuid::now_v7()),
		provider: Set(provider.to_string()),
		query_hash: Set(hash),
		response: Set(payload),
		expires_at: Set(expires),
		created_at: Set(now),
	})
	.exec(db)
	.await?;

	Ok(())
}

impl Default for MetadataService {
	fn default() -> Self {
		Self::new()
	}
}
