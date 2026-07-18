use axum::extract::{Path, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::series;
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct SeriesResponse {
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub book_count: usize,
	pub created_at: String,
	pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateSeriesRequest {
	pub name: String,
	pub description: Option<String>,
}
#[derive(Deserialize)]
pub struct UpdateSeriesRequest {
	pub name: Option<String>,
	pub description: Option<String>,
}

pub fn series_routes() -> Router<SharedState> {
	Router::new()
		.route("/", get(list_series))
		.route("/", post(create_series))
		.route("/{id}", get(get_series))
		.route("/{id}", put(update_series))
		.route("/{id}", delete(delete_series))
}

async fn list_series(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
) -> Result<Json<Vec<SeriesResponse>>, AppError> {
	let libs = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let rows = Series::find()
		.filter(series::Column::LibraryId.is_in(libs))
		.order_by_asc(series::Column::Name)
		.all(&state.db)
		.await?;
	let mut r = Vec::new();
	for s in rows {
		let bc = Books::find()
			.filter(crate::db::entities::books::Column::SeriesId.eq(Some(s.id)))
			.all(&state.db)
			.await?
			.len();
		r.push(SeriesResponse {
			id: s.id,
			name: s.name,
			description: s.description,
			book_count: bc,
			created_at: s.created_at.to_string(),
			updated_at: s.updated_at.to_string(),
		});
	}
	Ok(Json(r))
}

async fn create_series(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Json(req): Json<CreateSeriesRequest>,
) -> Result<Json<SeriesResponse>, AppError> {
	let libs = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let s = series::ActiveModel {
		id: Set(Uuid::new_v4()),
		library_id: Set(libs[0]),
		name: Set(req.name),
		description: Set(req.description),
		created_at: Set(now),
		updated_at: Set(now),
	}
	.insert(&state.db)
	.await?;
	Ok(Json(SeriesResponse {
		id: s.id,
		name: s.name,
		description: s.description,
		book_count: 0,
		created_at: s.created_at.to_string(),
		updated_at: s.updated_at.to_string(),
	}))
}

async fn get_series(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(id): Path<Uuid>,
) -> Result<Json<SeriesResponse>, AppError> {
	let libs = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let s = Series::find_by_id(id)
		.one(&state.db)
		.await?
		.filter(|x| libs.contains(&x.library_id))
		.ok_or_else(|| AppError::NotFound("Series not found".into()))?;
	let bc = Books::find()
		.filter(crate::db::entities::books::Column::SeriesId.eq(Some(s.id)))
		.all(&state.db)
		.await?
		.len();
	Ok(Json(SeriesResponse {
		id: s.id,
		name: s.name,
		description: s.description,
		book_count: bc,
		created_at: s.created_at.to_string(),
		updated_at: s.updated_at.to_string(),
	}))
}

async fn update_series(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(id): Path<Uuid>,
	Json(req): Json<UpdateSeriesRequest>,
) -> Result<Json<SeriesResponse>, AppError> {
	let libs = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let existing = Series::find_by_id(id)
		.one(&state.db)
		.await?
		.filter(|x| libs.contains(&x.library_id))
		.ok_or_else(|| AppError::NotFound("Series not found".into()))?;
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let mut active: series::ActiveModel = existing.into();
	active.updated_at = Set(now);
	if let Some(v) = req.name {
		active.name = Set(v);
	}
	if let Some(v) = req.description {
		active.description = Set(Some(v));
	}
	let s = active.update(&state.db).await?;
	let bc = Books::find()
		.filter(crate::db::entities::books::Column::SeriesId.eq(Some(s.id)))
		.all(&state.db)
		.await?
		.len();
	Ok(Json(SeriesResponse {
		id: s.id,
		name: s.name,
		description: s.description,
		book_count: bc,
		created_at: s.created_at.to_string(),
		updated_at: s.updated_at.to_string(),
	}))
}

async fn delete_series(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(id): Path<Uuid>,
) -> Result<Json<()>, AppError> {
	let libs = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let existing = Series::find_by_id(id)
		.one(&state.db)
		.await?
		.filter(|x| libs.contains(&x.library_id))
		.ok_or_else(|| AppError::NotFound("Series not found".into()))?;
	Books::update_many()
		.col_expr(
			crate::db::entities::books::Column::SeriesId,
			sea_orm::sea_query::Expr::val(Option::<Uuid>::None),
		)
		.filter(crate::db::entities::books::Column::SeriesId.eq(Some(existing.id)))
		.exec(&state.db)
		.await?;
	series::Entity::delete_by_id(existing.id).exec(&state.db).await?;
	Ok(Json(()))
}
