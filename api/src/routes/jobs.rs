use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::{AppError, SharedState};

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
	Router::new().route("/{id}", get(get_job))
}

async fn get_job(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(job_id): Path<Uuid>,
) -> Result<Json<JobResponse>, AppError> {
	let job = JobQueue::find_by_id(job_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Job not found".into()))?;

	if let Some(book_id_str) = job.payload.get("book_id").and_then(|v| v.as_str()) {
		if let Ok(book_id) = Uuid::parse_str(book_id_str) {
			if let Some(book) = Books::find_by_id(book_id).one(&state.db).await? {
				let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
				if !library_ids.contains(&book.library_id) {
					return Err(AppError::Forbidden("Access denied".into()));
				}
			}
		}
	}

	Ok(Json(JobResponse {
		id: job.id,
		kind: job.kind,
		status: job.status,
		error: job.error,
		retry_count: job.retry_count,
		max_retries: job.max_retries,
		created_at: job.created_at.to_string(),
	}))
}
