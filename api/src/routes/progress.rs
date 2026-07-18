use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::{annotations, books, reading_progress};
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct ProgressResponse {
	pub section_id: Uuid,
	pub block_index: i64,
	pub char_offset: i64,
	pub percentage: f64,
	pub updated_at: String,
}

#[derive(Deserialize)]
pub struct SaveProgressRequest {
	pub section_id: Uuid,
	pub block_index: i64,
	pub char_offset: i64,
	pub percentage: f64,
}

#[derive(Serialize)]
pub struct AnnotationResponse {
	pub id: Uuid,
	pub book_id: Uuid,
	pub section_id: Uuid,
	pub block_index: i64,
	pub start_offset: i64,
	pub end_offset: i64,
	pub color: Option<String>,
	pub note: Option<String>,
	pub created_at: String,
	pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateAnnotationRequest {
	pub section_id: Uuid,
	pub block_index: i64,
	pub start_offset: i64,
	pub end_offset: i64,
	pub color: Option<String>,
	pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateAnnotationRequest {
	pub color: Option<String>,
	pub note: Option<String>,
}

pub fn book_routes() -> Router<SharedState> {
	Router::new()
		.route("/{id}/progress", get(get_progress))
		.route("/{id}/progress", put(save_progress))
		.route("/{id}/annotations", get(list_annotations))
		.route("/{id}/annotations", post(create_annotation))
}

pub fn annotation_routes() -> Router<SharedState> {
	Router::new()
		.route("/all", get(list_all_annotations))
		.route("/{id}", put(update_annotation))
		.route("/{id}", delete(delete_annotation))
}

async fn list_all_annotations(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
) -> Result<Json<Vec<AnnotationResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book_ids: Vec<Uuid> = Books::find()
		.filter(books::Column::LibraryId.is_in(library_ids))
		.all(&state.db)
		.await?
		.into_iter()
		.map(|b| b.id)
		.collect();

	let annotations = Annotations::find()
		.filter(annotations::Column::UserId.eq(auth.user_id))
		.filter(annotations::Column::BookId.is_in(book_ids))
		.order_by_desc(annotations::Column::CreatedAt)
		.all(&state.db)
		.await?;

	Ok(Json(
		annotations
			.into_iter()
			.map(|a| AnnotationResponse {
				id: a.id,
				book_id: a.book_id,
				section_id: a.section_id,
				block_index: a.block_index,
				start_offset: a.start_offset,
				end_offset: a.end_offset,
				color: a.color,
				note: a.note,
				created_at: a.created_at.to_string(),
				updated_at: a.updated_at.to_string(),
			})
			.collect(),
	))
}

async fn get_progress(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Option<ProgressResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let progress = ReadingProgress::find()
		.filter(reading_progress::Column::UserId.eq(auth.user_id))
		.filter(reading_progress::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?;

	Ok(Json(progress.map(|p| ProgressResponse {
		section_id: p.section_id,
		block_index: p.block_index,
		char_offset: p.char_offset,
		percentage: p.percentage,
		updated_at: p.updated_at.to_string(),
	})))
}

async fn save_progress(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
	Json(req): Json<SaveProgressRequest>,
) -> Result<Json<ProgressResponse>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let existing = ReadingProgress::find()
		.filter(reading_progress::Column::UserId.eq(auth.user_id))
		.filter(reading_progress::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?;

	let progress = if let Some(p) = existing {
		let mut active: reading_progress::ActiveModel = p.into();
		active.section_id = Set(req.section_id);
		active.block_index = Set(req.block_index);
		active.char_offset = Set(req.char_offset);
		active.percentage = Set(req.percentage);
		active.updated_at = Set(now);
		ReadingProgress::update(active).exec(&state.db).await?
	} else {
		ReadingProgress::insert(reading_progress::ActiveModel {
			id: Set(Uuid::now_v7()),
			user_id: Set(auth.user_id),
			book_id: Set(book_id),
			section_id: Set(req.section_id),
			block_index: Set(req.block_index),
			char_offset: Set(req.char_offset),
			percentage: Set(req.percentage),
			updated_at: Set(now),
		})
		.exec_with_returning(&state.db)
		.await?
	};

	if req.percentage > 0.0 && req.percentage < 100.0 {
		let mut bm: books::ActiveModel = book.into();
		bm.read_status = Set("reading".to_string());
		bm.updated_at = Set(now);
		Books::update(bm).exec(&state.db).await?;
	} else if req.percentage >= 99.9 {
		let mut bm: books::ActiveModel = book.into();
		bm.read_status = Set("finished".to_string());
		bm.updated_at = Set(now);
		Books::update(bm).exec(&state.db).await?;
	}

	Ok(Json(ProgressResponse {
		section_id: progress.section_id,
		block_index: progress.block_index,
		char_offset: progress.char_offset,
		percentage: progress.percentage,
		updated_at: progress.updated_at.to_string(),
	}))
}

async fn list_annotations(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Vec<AnnotationResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let annotations = Annotations::find()
		.filter(annotations::Column::BookId.eq(book_id))
		.filter(annotations::Column::UserId.eq(auth.user_id))
		.order_by_asc(annotations::Column::CreatedAt)
		.all(&state.db)
		.await?;

	Ok(Json(
		annotations
			.into_iter()
			.map(|a| AnnotationResponse {
				id: a.id,
				book_id: a.book_id,
				section_id: a.section_id,
				block_index: a.block_index,
				start_offset: a.start_offset,
				end_offset: a.end_offset,
				color: a.color,
				note: a.note,
				created_at: a.created_at.to_string(),
				updated_at: a.updated_at.to_string(),
			})
			.collect(),
	))
}

async fn create_annotation(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
	Json(req): Json<CreateAnnotationRequest>,
) -> Result<(StatusCode, Json<AnnotationResponse>), AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let ann = Annotations::insert(annotations::ActiveModel {
		id: Set(Uuid::now_v7()),
		user_id: Set(auth.user_id),
		book_id: Set(book_id),
		section_id: Set(req.section_id),
		block_index: Set(req.block_index),
		start_offset: Set(req.start_offset),
		end_offset: Set(req.end_offset),
		color: Set(req.color),
		note: Set(req.note),
		created_at: Set(now),
		updated_at: Set(now),
	})
	.exec_with_returning(&state.db)
	.await?;

	Ok((
		StatusCode::CREATED,
		Json(AnnotationResponse {
			id: ann.id,
			book_id: ann.book_id,
			section_id: ann.section_id,
			block_index: ann.block_index,
			start_offset: ann.start_offset,
			end_offset: ann.end_offset,
			color: ann.color,
			note: ann.note,
			created_at: ann.created_at.to_string(),
			updated_at: ann.updated_at.to_string(),
		}),
	))
}

async fn update_annotation(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(annotation_id): Path<Uuid>,
	Json(req): Json<UpdateAnnotationRequest>,
) -> Result<Json<AnnotationResponse>, AppError> {
	let ann = Annotations::find_by_id(annotation_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Annotation not found".into()))?;

	if ann.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let mut active: annotations::ActiveModel = ann.into();
	if let Some(v) = req.color {
		active.color = Set(Some(v));
	}
	if let Some(v) = req.note {
		active.note = Set(Some(v));
	}
	active.updated_at = Set(now);

	let ann = Annotations::update(active).exec(&state.db).await?;
	Ok(Json(AnnotationResponse {
		id: ann.id,
		book_id: ann.book_id,
		section_id: ann.section_id,
		block_index: ann.block_index,
		start_offset: ann.start_offset,
		end_offset: ann.end_offset,
		color: ann.color,
		note: ann.note,
		created_at: ann.created_at.to_string(),
		updated_at: ann.updated_at.to_string(),
	}))
}

async fn delete_annotation(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(annotation_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	let ann = Annotations::find_by_id(annotation_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Annotation not found".into()))?;

	if ann.user_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	ann.delete(&state.db).await?;
	Ok(Json(serde_json::json!({ "message": "annotation deleted" })))
}
