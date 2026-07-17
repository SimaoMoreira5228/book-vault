use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::books;
use crate::{AppError, SharedState};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect,
};

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
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct BookHit {
    pub id: Uuid,
    pub title: String,
    pub author: Option<String>,
}

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", get(search_handler))
}

async fn search_handler(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    axum::extract::Query(q): axum::extract::Query<SearchQuery>,
) -> Result<Json<SearchResult>, AppError> {
    let library_ids =
        crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
    let limit = std::cmp::min(q.limit.unwrap_or(20), 100);
    let offset = q.offset.unwrap_or(0);
    let pattern = format!("%{}%", q.q);

    let hits = Books::find()
        .filter(books::Column::LibraryId.is_in(library_ids))
        .filter(
            books::Column::Title
                .like(&pattern)
                .or(books::Column::Author.like(&pattern)),
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

    Ok(Json(SearchResult { books }))
}
