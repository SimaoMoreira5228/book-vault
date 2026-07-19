pub mod dictionary;
pub mod segment;
pub mod wiktionary_parser;

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post, put};
use sea_orm::ModelTrait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::book_ir;
use crate::db::entities::prelude::*;
use crate::ir::block::Block;
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct LinguisticAnnotationResponse {
	pub id: Uuid,
	pub book_id: Uuid,
	pub language: String,
	pub section_id: Uuid,
	pub block_index: i64,
	pub char_start: i64,
	pub char_end: i64,
	pub surface_form: String,
	pub lemma: String,
	pub reading: Option<String>,
	pub pos: Option<String>,
	pub frequency_rank: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct VocabularyEntryResponse {
	pub id: Uuid,
	pub user_id: Uuid,
	pub language: String,
	pub lemma: String,
	pub sense_label: Option<String>,
	pub sense_id: Option<String>,
	pub definition: Option<String>,
	pub state: String,
	pub first_seen_at: String,
	pub last_reviewed_at: Option<String>,
	pub srs_due_at: Option<String>,
	pub srs_interval_days: Option<i64>,
	pub srs_ease_factor: Option<f64>,
	pub sentence_snippet: Option<String>,
	pub context_sentence: Option<String>,
	pub source: Option<String>,
	pub frequency_rank_sense: Option<i64>,
}

#[derive(Deserialize)]
pub struct AnnotateRequest {
	pub language: String,
}

#[derive(Deserialize)]
pub struct AddVocabRequest {
	pub language: String,
	pub lemma: String,
	pub sense_label: Option<String>,
	pub sense_id: Option<String>,
	pub definition: Option<String>,
	pub surface_form: Option<String>,
	pub sentence_snippet: Option<String>,
	pub context_sentence: Option<String>,
	pub source: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateVocabRequest {
	pub state: Option<String>,
	pub sense_label: Option<String>,
	pub sense_id: Option<String>,
	pub definition: Option<String>,
	pub srs_interval_days: Option<i64>,
	pub srs_ease_factor: Option<f64>,
}

#[derive(Deserialize)]
pub struct ListVocabQuery {
	pub language: Option<String>,
	pub state: Option<String>,
	pub due: Option<bool>,
}

#[derive(Deserialize)]
pub struct LookupRequest {
	pub word: String,
	pub context: String,
	pub language: String,
	pub definition_language: Option<String>,
}

#[derive(Serialize)]
pub struct LookupResponse {
	pub entries: Vec<dictionary::DictionaryEntry>,
	pub cached: bool,
	pub translation: Option<String>,
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/{id}/language/annotate", post(annotate_book_language))
		.route("/{id}/language/annotations", get(get_annotations))
}

pub fn vocab_routes() -> Router<SharedState> {
	Router::new()
		.route("/", get(list_vocabulary))
		.route("/", post(add_vocabulary))
		.route("/lookup", post(lookup_word))
		.route("/review", get(get_review_cards))
		.route("/{id}", put(update_vocabulary))
		.route("/{id}/review", post(submit_review))
		.route("/{id}/sentences", get(list_sentences))
		.route("/{id}/sentences", post(add_sentence))
		.route("/{id}", delete(delete_vocabulary))
}

async fn lookup_word(
	State(state): State<SharedState>,
	_auth: AuthenticatedUser,
	Json(req): Json<LookupRequest>,
) -> Result<Json<LookupResponse>, AppError> {
	let word = req.word.trim().to_lowercase();
	if word.is_empty() {
		return Err(AppError::BadRequest("Empty word".into()));
	}

	let definition_language = req.definition_language.clone().unwrap_or_else(|| req.language.clone());
	let word_lang = &req.language[..req.language.len().min(2)];
	let def_lang = &definition_language[..definition_language.len().min(2)];

	let context_hash = blake3::hash(req.context.as_bytes()).to_hex().to_string();
	let cache_key = format!("{}:{}:{}:{}", word, word_lang, def_lang, &context_hash[..16]);

	let existing = crate::db::entities::dictionary_cache::Entity::find()
		.filter(
			sea_orm::Condition::all()
				.add(crate::db::entities::dictionary_cache::Column::Word.eq(&word))
				.add(crate::db::entities::dictionary_cache::Column::Language.eq(&req.language))
				.add(crate::db::entities::dictionary_cache::Column::ContextHash.eq(&cache_key)),
		)
		.one(&state.db)
		.await?;

	if let Some(cached) = existing {
		let _now_utc: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
		if cached.expires_at > _now_utc {
			let entries: Vec<dictionary::DictionaryEntry> = serde_json::from_str(&cached.response_json).unwrap_or_default();
			return Ok(Json(LookupResponse { entries, cached: true, translation: None }));
		}
	}

	let query = dictionary::DictionaryQuery {
		word: word.clone(),
		context: req.context.clone(),
		language: req.language.clone(),
		definition_language: definition_language.clone(),
	};

	let entries = if let Some(provider) = state.dictionary_service.find_provider(word_lang) {
		provider.lookup(&query).await.unwrap_or_default()
	} else {
		vec![dictionary::DictionaryEntry {
			word: word.clone(),
			lemma: segment::lemmatize(&word, word_lang),
			sense_label: None,
			sense_id: None,
			part_of_speech: None,
			definition: format!("No dictionary provider configured for \"{}\".", word),
			example_sentences: vec![req.context.clone()],
			pronunciation: None,
			frequency_rank: None,
		}]
	};

	let translation = if let Some(ref translator) = state.dictionary_service.translator {
		if def_lang != "en" {
			translator.translate(&req.context, word_lang, def_lang).await.unwrap_or(None)
		} else {
			None
		}
	} else {
		None
	};

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let expires = now + chrono::Duration::hours(48);
	let json = serde_json::to_string(&entries).unwrap_or_default();

	let _ = crate::db::entities::dictionary_cache::Entity::insert(crate::db::entities::dictionary_cache::ActiveModel {
		id: Set(Uuid::now_v7()),
		word: Set(word.clone()),
		language: Set(req.language.clone()),
		context_hash: Set(cache_key),
		response_json: Set(json),
		created_at: Set(now),
		expires_at: Set(expires),
	})
	.exec(&state.db)
	.await;

	Ok(Json(LookupResponse { entries, cached: false, translation }))
}

async fn get_review_cards(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Query(query): Query<ListVocabQuery>,
) -> Result<Json<Vec<VocabularyEntryResponse>>, AppError> {
	let _now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let mut find = crate::db::entities::vocabulary_entries::Entity::find()
		.filter(
			sea_orm::Condition::all()
				.add(crate::db::entities::vocabulary_entries::Column::UserId.eq(auth.user_id))
				.add(crate::db::entities::vocabulary_entries::Column::State.eq("learning")),
		)
		.order_by_asc(crate::db::entities::vocabulary_entries::Column::SrsDueAt);

	if query.due.unwrap_or(false) {
		find = find.filter(
			sea_orm::Condition::any()
				.add(crate::db::entities::vocabulary_entries::Column::SrsDueAt.is_null())
				.add(crate::db::entities::vocabulary_entries::Column::SrsDueAt.lte(_now)),
		);
	}

	if let Some(ref lang) = query.language {
		find = find.filter(crate::db::entities::vocabulary_entries::Column::Language.eq(lang));
	}

	let entries = find.all(&state.db).await?;
	Ok(Json(entries.into_iter().map(entry_to_response).collect()))
}

async fn submit_review(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(entry_id): Path<Uuid>,
	Json(req): Json<SubmitReviewRequest>,
) -> Result<Json<VocabularyEntryResponse>, AppError> {
	let existing = crate::db::entities::vocabulary_entries::Entity::find_by_id(entry_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Vocabulary entry not found".into()))?;

	if existing.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let current_ease = existing.srs_ease_factor.unwrap_or(2.5);
	let current_interval = existing.srs_interval_days.unwrap_or(0);
	let mut active: crate::db::entities::vocabulary_entries::ActiveModel = existing.into();

	let q = req.quality as f64;

	let new_ease = (current_ease + 0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02)).max(1.3);
	let new_interval = if q < 3.0 {
		0
	} else if current_interval == 0 {
		1
	} else if current_interval == 1 {
		6
	} else {
		(current_interval as f64 * new_ease).round() as i64
	};

	let next_due = now + chrono::Duration::days(new_interval);

	active.srs_ease_factor = Set(Some(new_ease));
	active.srs_interval_days = Set(Some(new_interval));
	active.srs_due_at = Set(Some(next_due));
	active.last_reviewed_at = Set(Some(now));

	if q >= 4.0 {
		active.state = Set("known".to_string());
	} else if q >= 2.0 {
		active.state = Set("learning".to_string());
	}

	let entry = active.update(&state.db).await?;
	Ok(Json(entry_to_response(entry)))
}

async fn list_sentences(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(entry_id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
	let entry = crate::db::entities::vocabulary_entries::Entity::find_by_id(entry_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Entry not found".into()))?;
	if entry.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let sentences = crate::db::entities::vocab_example_sentences::Entity::find()
		.filter(crate::db::entities::vocab_example_sentences::Column::VocabularyEntryId.eq(entry_id))
		.all(&state.db)
		.await?;

	Ok(Json(
		sentences
			.into_iter()
			.map(|s| {
				serde_json::json!({
					"id": s.id,
					"sentence": s.sentence,
					"source": s.source,
					"book_id": s.book_id,
					"book_title": s.book_title,
					"created_at": s.created_at,
				})
			})
			.collect(),
	))
}

async fn add_sentence(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(entry_id): Path<Uuid>,
	Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
	let entry = crate::db::entities::vocabulary_entries::Entity::find_by_id(entry_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Entry not found".into()))?;
	if entry.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let sentence = req["sentence"].as_str().unwrap_or("").to_string();
	if sentence.is_empty() {
		return Err(AppError::BadRequest("Empty sentence".into()));
	}

	let source = req["source"].as_str().unwrap_or("user").to_string();

	let model = crate::db::entities::vocab_example_sentences::ActiveModel {
		id: Set(Uuid::now_v7()),
		vocabulary_entry_id: Set(entry_id),
		sentence: Set(sentence),
		source: Set(source),
		book_id: Set(req["book_id"].as_str().and_then(|s| Uuid::try_parse(s).ok())),
		book_title: Set(req["book_title"].as_str().map(|s| s.to_string())),
		created_at: Set(now),
	};

	let result = model.insert(&state.db).await?;
	Ok(Json(serde_json::json!({ "id": result.id, "sentence": result.sentence })))
}

#[derive(Deserialize)]
pub struct SubmitReviewRequest {
	pub quality: u8,
}

async fn annotate_book_language(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
	Json(req): Json<AnnotateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let ir_row = BookIr::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	let ir: crate::ir::BookIr = crate::ingest::deserialize_ir(&ir_row.payload)?;

	for section in &ir.spine {
		for (block_idx, block) in section.blocks.iter().enumerate() {
			if let Some(text) = extract_text(block) {
				let tokens = segment::tokenize(&text, &req.language);
				for token in &tokens {
					let existing = crate::db::entities::linguistic_annotations::Entity::find()
						.filter(
							sea_orm::Condition::all()
								.add(crate::db::entities::linguistic_annotations::Column::BookId.eq(book_id))
								.add(crate::db::entities::linguistic_annotations::Column::SectionId.eq(section.id))
								.add(crate::db::entities::linguistic_annotations::Column::BlockIndex.eq(block_idx as i64))
								.add(
									crate::db::entities::linguistic_annotations::Column::CharStart
										.eq(token.char_start as i64),
								),
						)
						.one(&state.db)
						.await?;
					if existing.is_some() {
						continue;
					}
					crate::db::entities::linguistic_annotations::Entity::insert(
						crate::db::entities::linguistic_annotations::ActiveModel {
							id: Set(Uuid::now_v7()),
							book_id: Set(book_id),
							language: Set(req.language.clone()),
							section_id: Set(section.id),
							block_index: Set(block_idx as i64),
							char_start: Set(token.char_start as i64),
							char_end: Set(token.char_end as i64),
							surface_form: Set(token.surface_form.clone()),
							lemma: Set(token.lemma.clone()),
							reading: Set(token.reading.clone()),
							pos: Set(token.pos.clone()),
							frequency_rank: Set(token.frequency_rank.map(|r| r as i64)),
						},
					)
					.exec(&state.db)
					.await?;
				}
			}
		}
	}

	Ok(Json(serde_json::json!({ "message": "annotations created" })))
}

fn extract_text(block: &Block) -> Option<String> {
	match block {
		Block::Paragraph(spans) => Some(spans.iter().map(|s| s.text.as_str()).collect::<String>()),
		Block::Heading { spans, .. } => Some(spans.iter().map(|s| s.text.as_str()).collect::<String>()),
		_ => None,
	}
}

async fn get_annotations(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
	Query(query): Query<AnnotateRequest>,
) -> Result<Json<Vec<LinguisticAnnotationResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let anns = crate::db::entities::linguistic_annotations::Entity::find()
		.filter(
			sea_orm::Condition::all()
				.add(crate::db::entities::linguistic_annotations::Column::BookId.eq(book_id))
				.add(crate::db::entities::linguistic_annotations::Column::Language.eq(&query.language)),
		)
		.all(&state.db)
		.await?;

	Ok(Json(
		anns.into_iter()
			.map(|a| LinguisticAnnotationResponse {
				id: a.id,
				book_id: a.book_id,
				language: a.language,
				section_id: a.section_id,
				block_index: a.block_index,
				char_start: a.char_start,
				char_end: a.char_end,
				surface_form: a.surface_form,
				lemma: a.lemma,
				reading: a.reading,
				pos: a.pos,
				frequency_rank: a.frequency_rank,
			})
			.collect(),
	))
}

async fn list_vocabulary(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Query(query): Query<ListVocabQuery>,
) -> Result<Json<Vec<VocabularyEntryResponse>>, AppError> {
	let mut find = crate::db::entities::vocabulary_entries::Entity::find()
		.filter(crate::db::entities::vocabulary_entries::Column::UserId.eq(auth.user_id))
		.order_by_desc(crate::db::entities::vocabulary_entries::Column::FirstSeenAt);

	if let Some(ref lang) = query.language {
		find = find.filter(crate::db::entities::vocabulary_entries::Column::Language.eq(lang));
	}
	if let Some(ref state) = query.state {
		find = find.filter(crate::db::entities::vocabulary_entries::Column::State.eq(state));
	}

	let entries = find.all(&state.db).await?;
	Ok(Json(entries.into_iter().map(entry_to_response).collect()))
}

async fn add_vocabulary(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Json(req): Json<AddVocabRequest>,
) -> Result<Json<VocabularyEntryResponse>, AppError> {
	let existing = crate::db::entities::vocabulary_entries::Entity::find()
		.filter(
			sea_orm::Condition::all()
				.add(crate::db::entities::vocabulary_entries::Column::UserId.eq(auth.user_id))
				.add(crate::db::entities::vocabulary_entries::Column::Language.eq(&req.language))
				.add(crate::db::entities::vocabulary_entries::Column::Lemma.eq(&req.lemma)),
		)
		.one(&state.db)
		.await?;

	if let Some(existing) = existing {
		return Ok(Json(entry_to_response(existing)));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let entry = crate::db::entities::vocabulary_entries::ActiveModel {
		id: Set(Uuid::now_v7()),
		user_id: Set(auth.user_id),
		language: Set(req.language),
		lemma: Set(req.lemma),
		sense_label: Set(req.sense_label),
		sense_id: Set(req.sense_id),
		definition: Set(req.definition),
		state: Set("unknown".to_string()),
		first_seen_at: Set(now),
		last_reviewed_at: Set(None),
		srs_due_at: Set(None),
		srs_interval_days: Set(None),
		srs_ease_factor: Set(None),
		sentence_snippet: Set(req.sentence_snippet),
		context_sentence: Set(req.context_sentence),
		source: Set(req.source),
		frequency_rank_sense: Set(None),
	};

	let entry = entry.insert(&state.db).await?;
	Ok(Json(entry_to_response(entry)))
}

async fn update_vocabulary(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(entry_id): Path<Uuid>,
	Json(req): Json<UpdateVocabRequest>,
) -> Result<Json<VocabularyEntryResponse>, AppError> {
	let existing = crate::db::entities::vocabulary_entries::Entity::find_by_id(entry_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Vocabulary entry not found".into()))?;

	if existing.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let mut active: crate::db::entities::vocabulary_entries::ActiveModel = existing.into();

	if let Some(ref state) = req.state {
		active.state = Set(state.clone());
		active.last_reviewed_at = Set(Some(now));
	}
	if let Some(days) = req.srs_interval_days {
		active.srs_interval_days = Set(Some(days));
	}
	if let Some(factor) = req.srs_ease_factor {
		active.srs_ease_factor = Set(Some(factor));
	}

	let entry = active.update(&state.db).await?;
	Ok(Json(entry_to_response(entry)))
}

async fn delete_vocabulary(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(entry_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	let existing = crate::db::entities::vocabulary_entries::Entity::find_by_id(entry_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Vocabulary entry not found".into()))?;

	if existing.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	existing.delete(&state.db).await?;
	Ok(Json(serde_json::json!({ "message": "vocabulary entry deleted" })))
}

fn entry_to_response(e: crate::db::entities::vocabulary_entries::Model) -> VocabularyEntryResponse {
	VocabularyEntryResponse {
		id: e.id,
		user_id: e.user_id,
		language: e.language,
		lemma: e.lemma,
		sense_label: e.sense_label,
		sense_id: e.sense_id,
		definition: e.definition,
		state: e.state,
		first_seen_at: e.first_seen_at.to_string(),
		last_reviewed_at: e.last_reviewed_at.map(|d| d.to_string()),
		srs_due_at: e.srs_due_at.map(|d| d.to_string()),
		srs_interval_days: e.srs_interval_days,
		srs_ease_factor: e.srs_ease_factor,
		sentence_snippet: e.sentence_snippet,
		context_sentence: e.context_sentence,
		source: e.source,
		frequency_rank_sense: e.frequency_rank_sense,
	}
}
