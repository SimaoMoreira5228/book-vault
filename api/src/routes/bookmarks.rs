use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::bookmarks;
use crate::db::entities::prelude::*;
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct BookmarkResponse {
	pub id: Uuid,
	pub book_id: Uuid,
	pub section_id: Uuid,
	pub block_index: i64,
	pub title: Option<String>,
	pub note: Option<String>,
	pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateBookmarkRequest {
	pub book_id: Uuid,
	pub section_id: Uuid,
	pub block_index: i64,
	pub title: Option<String>,
	pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBookmarkRequest {
	pub title: Option<String>,
	pub note: Option<String>,
}

pub fn bookmark_routes() -> Router<SharedState> {
	Router::new()
		.route("/{book_id}", get(list_bookmarks))
		.route("/{book_id}", post(create_bookmark))
		.route("/single/{bookmark_id}", get(get_bookmark))
		.route("/single/{bookmark_id}", post(update_bookmark))
		.route("/single/{bookmark_id}", delete(delete_bookmark))
}

async fn list_bookmarks(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Vec<BookmarkResponse>>, AppError> {
	let bookmarks = Bookmarks::find()
		.filter(bookmarks::Column::BookId.eq(book_id))
		.filter(bookmarks::Column::UserId.eq(auth.user_id))
		.order_by_desc(bookmarks::Column::CreatedAt)
		.all(&state.db)
		.await?;

	Ok(Json(
		bookmarks
			.into_iter()
			.map(|b| BookmarkResponse {
				id: b.id,
				book_id: b.book_id,
				section_id: b.section_id,
				block_index: b.block_index,
				title: b.title,
				note: b.note,
				created_at: b.created_at.to_string(),
			})
			.collect(),
	))
}

async fn create_bookmark(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(_book_id): Path<Uuid>,
	Json(req): Json<CreateBookmarkRequest>,
) -> Result<Json<BookmarkResponse>, AppError> {
	let now = chrono::Utc::now();

	let bookmark = bookmarks::ActiveModel {
		id: Set(Uuid::new_v4()),
		user_id: Set(auth.user_id),
		book_id: Set(req.book_id),
		section_id: Set(req.section_id),
		block_index: Set(req.block_index),
		title: Set(req.title),
		note: Set(req.note),
		created_at: Set(now.into()),
	}
	.insert(&state.db)
	.await?;

	Ok(Json(BookmarkResponse {
		id: bookmark.id,
		book_id: bookmark.book_id,
		section_id: bookmark.section_id,
		block_index: bookmark.block_index,
		title: bookmark.title,
		note: bookmark.note,
		created_at: bookmark.created_at.to_string(),
	}))
}

async fn get_bookmark(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(bookmark_id): Path<Uuid>,
) -> Result<Json<BookmarkResponse>, AppError> {
	let bookmark = Bookmarks::find_by_id(bookmark_id)
		.one(&state.db)
		.await?
		.filter(|b| b.user_id == auth.user_id)
		.ok_or_else(|| AppError::NotFound("Bookmark not found".into()))?;

	Ok(Json(BookmarkResponse {
		id: bookmark.id,
		book_id: bookmark.book_id,
		section_id: bookmark.section_id,
		block_index: bookmark.block_index,
		title: bookmark.title,
		note: bookmark.note,
		created_at: bookmark.created_at.to_string(),
	}))
}

async fn update_bookmark(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(bookmark_id): Path<Uuid>,
	Json(req): Json<UpdateBookmarkRequest>,
) -> Result<Json<BookmarkResponse>, AppError> {
	let existing = Bookmarks::find_by_id(bookmark_id)
		.one(&state.db)
		.await?
		.filter(|b| b.user_id == auth.user_id)
		.ok_or_else(|| AppError::NotFound("Bookmark not found".into()))?;

	let mut active: bookmarks::ActiveModel = existing.into();
	if let Some(title) = req.title {
		active.title = Set(Some(title));
	}
	if let Some(note) = req.note {
		active.note = Set(Some(note));
	}
	let bookmark = active.update(&state.db).await?;

	Ok(Json(BookmarkResponse {
		id: bookmark.id,
		book_id: bookmark.book_id,
		section_id: bookmark.section_id,
		block_index: bookmark.block_index,
		title: bookmark.title,
		note: bookmark.note,
		created_at: bookmark.created_at.to_string(),
	}))
}

async fn delete_bookmark(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(bookmark_id): Path<Uuid>,
) -> Result<Json<()>, AppError> {
	let existing = Bookmarks::find_by_id(bookmark_id)
		.one(&state.db)
		.await?
		.filter(|b| b.user_id == auth.user_id)
		.ok_or_else(|| AppError::NotFound("Bookmark not found".into()))?;

	bookmarks::Entity::delete_by_id(existing.id).exec(&state.db).await?;

	Ok(Json(()))
}
