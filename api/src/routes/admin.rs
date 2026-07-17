use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::{job_queue, users};
use crate::AppError;
use crate::SharedState;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

#[derive(Serialize)]
pub struct AdminUserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub is_admin: bool,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct JobResponse {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub error: Option<String>,
    pub retry_count: i8,
    pub max_retries: i8,
    pub created_at: String,
}

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/jobs", get(list_jobs))
        .route("/users", get(list_users))
        .route("/sessions/cleanup", get(cleanup_sessions))
}

async fn require_admin(auth: &AuthenticatedUser) -> Result<(), AppError> {
    if !auth.is_admin {
        return Err(AppError::Forbidden("Admin access required".into()));
    }
    Ok(())
}

async fn list_jobs(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<JobResponse>>, AppError> {
    require_admin(&auth).await?;

    let jobs = JobQueue::find()
        .order_by_desc(job_queue::Column::CreatedAt)
        .limit(50)
        .all(&state.db)
        .await?;

    Ok(Json(
        jobs.into_iter()
            .map(|j| JobResponse {
                id: j.id,
                kind: j.kind,
                status: j.status,
                error: j.error,
                retry_count: j.retry_count,
                max_retries: j.max_retries,
                created_at: j.created_at.to_string(),
            })
            .collect(),
    ))
}

async fn list_users(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<AdminUserResponse>>, AppError> {
    require_admin(&auth).await?;

    let users = Users::find()
        .order_by_desc(users::Column::CreatedAt)
        .all(&state.db)
        .await?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| AdminUserResponse {
                id: u.id,
                email: u.email,
                display_name: u.display_name,
                is_admin: u.is_admin,
                created_at: u.created_at.to_string(),
            })
            .collect(),
    ))
}

async fn cleanup_sessions(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&auth).await?;

    let deleted = crate::auth::session::SessionManager::cleanup_expired(&state.db, 30).await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
