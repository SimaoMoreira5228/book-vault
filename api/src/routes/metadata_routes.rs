use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::metadata::{MetadataField, MetadataQuery, ProspectiveMetadata};
use crate::{AppError, SharedState};
use sea_orm::EntityTrait;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct CandidateQuery {
    pub title: Option<String>,
    pub author: Option<String>,
    pub isbn: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfirmRequest {
    pub candidate: ProspectiveMetadata,
}

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/{id}/metadata", get(get_metadata))
        .route("/{id}/metadata/candidates", get(search_candidates))
        .route("/{id}/metadata/confirm", post(confirm_match))
        .route("/{id}/metadata/refresh", post(refresh_metadata_handler))
        .route("/{id}/metadata/lock/{field}", post(lock_field))
        .route("/{id}/metadata/lock/{field}", delete(unlock_field))
}

async fn get_metadata(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_book_access(&state, &auth, book_id).await?;
    let result = state.metadata_service.get_metadata(&state.db, book_id).await?;
    Ok(Json(result))
}

async fn search_candidates(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
    Query(q): Query<CandidateQuery>,
) -> Result<Json<Vec<ProspectiveMetadata>>, AppError> {
    verify_book_access(&state, &auth, book_id).await?;

    let query = MetadataQuery {
        title: q.title,
        author: q.author,
        isbn: q.isbn,
    };

    let results = state
        .metadata_service
        .search_candidates(&state.db, book_id, &query)
        .await?;
    Ok(Json(results))
}

async fn confirm_match(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
    Json(req): Json<ConfirmRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_book_access(&state, &auth, book_id).await?;
    let result = state
        .metadata_service
        .confirm_match(&state.db, book_id, &req.candidate)
        .await?;
    Ok(Json(result))
}

async fn refresh_metadata_handler(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_book_access(&state, &auth, book_id).await?;
    let result = state
        .metadata_service
        .refresh_metadata(&state.db, book_id)
        .await?;
    Ok(Json(result))
}

async fn lock_field(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path((book_id, field_str)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_book_access(&state, &auth, book_id).await?;
    let field = MetadataField::from_str(&field_str)
        .map_err(|_| AppError::BadRequest(format!("Invalid field: {field_str}")))?;
    let result = state
        .metadata_service
        .lock_field(&state.db, book_id, field)
        .await?;
    Ok(Json(result))
}

async fn unlock_field(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path((book_id, field_str)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_book_access(&state, &auth, book_id).await?;
    let field = MetadataField::from_str(&field_str)
        .map_err(|_| AppError::BadRequest(format!("Invalid field: {field_str}")))?;
    let result = state
        .metadata_service
        .unlock_field(&state.db, book_id, field)
        .await?;
    Ok(Json(result))
}

async fn verify_book_access(
    state: &SharedState,
    auth: &AuthenticatedUser,
    book_id: Uuid,
) -> Result<(), AppError> {
    let book = Books::find_by_id(book_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    let library_ids =
        crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
    if !library_ids.contains(&book.library_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    Ok(())
}
