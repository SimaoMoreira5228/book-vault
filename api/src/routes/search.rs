use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::books;
use crate::db::entities::prelude::*;
use crate::{AppError, SharedState};

#[derive(Deserialize)]
pub struct SearchQuery {
	pub q: String,
	pub kind: Option<String>,
	pub book_id: Option<Uuid>,
	pub limit: Option<u64>,
	pub offset: Option<u64>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct SearchResult {
	pub books: Vec<BookHit>,
	pub content_hits: Vec<ContentHit>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct BookHit {
	pub id: Uuid,
	pub title: String,
	pub author: Option<String>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ContentHit {
	pub book_id: Uuid,
	pub section_id: Uuid,
	pub block_index: u32,
	pub snippet: String,
	pub score: f64,
}

pub fn routes() -> Router<SharedState> {
	Router::new().route("/", get(search_handler))
}

async fn search_handler(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	axum::extract::Query(q): axum::extract::Query<SearchQuery>,
) -> Result<Json<SearchResult>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let limit = std::cmp::min(q.limit.unwrap_or(20), 100);
	let offset = q.offset.unwrap_or(0);

	let user_book_ids: Vec<Uuid> = Books::find()
		.filter(books::Column::LibraryId.is_in(library_ids.clone()))
		.all(&state.db)
		.await?
		.into_iter()
		.map(|b| b.id)
		.collect();

	let pattern = format!("%{}%", q.q);
	let hits = Books::find()
		.filter(books::Column::LibraryId.is_in(library_ids))
		.filter(
			Condition::any()
				.add(books::Column::Title.like(&pattern))
				.add(books::Column::Author.like(&pattern)),
		)
		.order_by_desc(books::Column::UpdatedAt)
		.offset(offset)
		.limit(limit)
		.all(&state.db)
		.await?;

	let books: Vec<BookHit> = hits
		.into_iter()
		.map(|b| BookHit {
			id: b.id,
			title: b.title,
			author: b.author,
		})
		.collect();

	let raw_hits = state.search_engine.search(&q.q, limit as usize);

	let content_hits: Vec<ContentHit> = raw_hits
		.into_iter()
		.filter(|h| user_book_ids.contains(&h.book_id))
		.map(|h| ContentHit {
			book_id: h.book_id,
			section_id: h.section_id,
			block_index: h.block_index,
			snippet: h.snippet,
			score: h.score,
		})
		.collect();

	Ok(Json(SearchResult { books, content_hits }))
}
