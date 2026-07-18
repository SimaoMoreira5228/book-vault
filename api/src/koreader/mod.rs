use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::entities::prelude::*;
use crate::db::entities::{books, reading_progress};
use crate::{AppError, SharedState};

use super::auth::middleware::AuthenticatedUser;

#[derive(Serialize, Deserialize, Debug)]
pub struct KoreaderProgressRequest {
	#[serde(default)]
	pub document: String,
	#[serde(default)]
	pub progress: f64,
	#[serde(default)]
	pub device_id: String,
	#[serde(default)]
	pub status: String,
	#[serde(default)]
	pub current_page: u32,
	#[serde(default)]
	pub total_pages: u32,
	#[serde(default)]
	pub timestamp: u64,
}

#[derive(Serialize, Debug)]
pub struct KoreaderProgressResponse {
	pub document: String,
	pub progress: f64,
	pub device_id: String,
	pub status: String,
	pub current_page: u32,
	pub total_pages: u32,
	pub timestamp: u64,
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/koreader/progress", put(save_koreader_progress))
		.route("/koreader/progress/{book_id}", get(get_koreader_progress))
}

async fn save_koreader_progress(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Json(req): Json<KoreaderProgressRequest>,
) -> Result<(StatusCode, Json<KoreaderProgressResponse>), AppError> {
	let book_id = resolve_book_id(&state, &req.document, auth.user_id).await?;

	let now = chrono::Utc::now().into();
	let percentage = req.progress.clamp(0.0, 100.0);

	let existing = ReadingProgress::find()
		.filter(reading_progress::Column::UserId.eq(auth.user_id))
		.filter(reading_progress::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?;

	if let Some(progress) = existing {
		let mut active: reading_progress::ActiveModel = progress.into();
		active.percentage = Set(percentage);
		active.updated_at = Set(now);
		ReadingProgress::update(active).exec(&state.db).await?;
	} else {
		ReadingProgress::insert(reading_progress::ActiveModel {
			id: Set(Uuid::now_v7()),
			user_id: Set(auth.user_id),
			book_id: Set(book_id),
			section_id: Set(Uuid::nil()),
			block_index: Set(0),
			char_offset: Set(0),
			percentage: Set(percentage),
			updated_at: Set(now),
		})
		.exec(&state.db)
		.await?;
	}

	Ok((
		StatusCode::OK,
		Json(KoreaderProgressResponse {
			document: req.document,
			progress: percentage,
			device_id: req.device_id,
			status: req.status,
			current_page: req.current_page,
			total_pages: req.total_pages,
			timestamp: req.timestamp,
		}),
	))
}

async fn get_koreader_progress(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Vec<KoreaderProgressResponse>>, AppError> {
	let progress = ReadingProgress::find()
		.filter(reading_progress::Column::UserId.eq(auth.user_id))
		.filter(reading_progress::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?;

	if let Some(p) = progress {
		let book = Books::find_by_id(book_id)
			.one(&state.db)
			.await?
			.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

		Ok(Json(vec![KoreaderProgressResponse {
			document: book.source_hash.unwrap_or_else(|| book.id.to_string()),
			progress: p.percentage,
			device_id: String::new(),
			status: infer_koreader_status(&book.read_status),
			current_page: 0,
			total_pages: book.page_count.unwrap_or(0) as u32,
			timestamp: p.updated_at.timestamp() as u64,
		}]))
	} else {
		Ok(Json(vec![]))
	}
}

async fn resolve_book_id(state: &SharedState, document: &str, user_id: Uuid) -> Result<Uuid, AppError> {
	if let Ok(uid) = Uuid::try_parse(document) {
		return Ok(uid);
	}

	let books = Books::find()
		.filter(books::Column::SourceHash.eq(Some(document.to_string())))
		.all(&state.db)
		.await?;

	if let Some(b) = books.first() {
		return Ok(b.id);
	}

	let title_clean = document.rsplit('/').next().unwrap_or(document);
	let pattern = format!("%{}%", title_clean);
	let fuzzy = Books::find()
		.filter(sea_orm::Condition::any().add(books::Column::Title.like(&pattern)))
		.filter(books::Column::LibraryId.is_in(
			crate::routes::books::get_user_library_ids(&state.db, user_id)
				.await?,
		))
		.one(&state.db)
		.await?;

	if let Some(b) = fuzzy {
		return Ok(b.id);
	}

	Err(AppError::NotFound(format!("Book not found: {}", document)))
}

fn infer_koreader_status(read_status: &str) -> String {
	match read_status {
		"reading" => "reading".to_string(),
		"finished" => "complete".to_string(),
		_ => "paused".to_string(),
	}
}
