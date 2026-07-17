use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::book_ir;
use crate::db::entities::prelude::*;
use crate::{AppError, SharedState};

pub fn routes() -> Router<SharedState> {
	Router::new().route("/{id}/read", get(read_book))
}

async fn read_book(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let ir = BookIr::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	let decoded: crate::ir::BookIr =
		rmp_serde::from_slice(&ir.payload).map_err(|e| AppError::Internal(format!("Failed to decode IR: {}", e)))?;

	Ok(Json(serde_json::json!({ "book": decoded })))
}
