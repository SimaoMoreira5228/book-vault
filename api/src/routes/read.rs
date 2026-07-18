use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::book_ir;
use crate::db::entities::prelude::*;
use crate::ir::block::Block;
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct SpineItem {
	pub id: Uuid,
	pub title: Option<String>,
	pub sequence_index: u32,
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/{id}/read", get(read_book))
		.route("/{id}/read/spine", get(read_spine))
		.route("/{id}/read/section/{section_id}", get(read_section))
}

async fn verify_access(state: &SharedState, auth: &AuthenticatedUser, book_id: Uuid) -> Result<(), AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}
	Ok(())
}

async fn load_ir(state: &SharedState, book_id: Uuid) -> Result<crate::ir::BookIr, AppError> {
	let ir = BookIr::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	crate::ingest::deserialize_ir(&ir.payload)
}

async fn read_book(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	verify_access(&state, &auth, book_id).await?;
	let decoded = load_ir(&state, book_id).await?;
	Ok(Json(serde_json::json!({ "book": decoded })))
}

async fn read_spine(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Vec<SpineItem>>, AppError> {
	verify_access(&state, &auth, book_id).await?;
	let decoded = load_ir(&state, book_id).await?;

	let spine: Vec<SpineItem> = decoded
		.spine
		.into_iter()
		.map(|s| SpineItem {
			id: s.id,
			title: s.title,
			sequence_index: s.sequence_index,
		})
		.collect();

	Ok(Json(spine))
}

async fn read_section(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path((book_id, section_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Block>>, AppError> {
	verify_access(&state, &auth, book_id).await?;
	let decoded = load_ir(&state, book_id).await?;

	let section = decoded
		.spine
		.into_iter()
		.find(|s| s.id == section_id)
		.ok_or_else(|| AppError::NotFound("Section not found".into()))?;

	Ok(Json(section.blocks))
}
