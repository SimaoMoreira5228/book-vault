use axum::extract::{Path, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::authors;
use crate::db::entities::prelude::*;
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct AuthorResponse {
	pub id: Uuid,
	pub name: String,
	pub sort_name: Option<String>,
	pub bio: Option<String>,
	pub birth_date: Option<String>,
	pub death_date: Option<String>,
	pub photo_asset_id: Option<Uuid>,
	pub book_count: usize,
	pub created_at: String,
	pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateAuthorRequest {
	pub name: String,
	pub sort_name: Option<String>,
	pub bio: Option<String>,
	pub birth_date: Option<String>,
	pub death_date: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateAuthorRequest {
	pub name: Option<String>,
	pub sort_name: Option<String>,
	pub bio: Option<String>,
	pub birth_date: Option<String>,
	pub death_date: Option<String>,
}

pub fn author_routes() -> Router<SharedState> {
	Router::new()
		.route("/", get(list_authors))
		.route("/", post(create_author))
		.route("/{id}", get(get_author))
		.route("/{id}", put(update_author))
		.route("/{id}", delete(delete_author))
}

async fn list_authors(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
) -> Result<Json<Vec<AuthorResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;

	let author_rows = Authors::find()
		.filter(authors::Column::LibraryId.is_in(library_ids))
		.order_by_asc(authors::Column::Name)
		.all(&state.db)
		.await?;

	let mut results = Vec::new();
	for a in author_rows {
		let bc = Books::find()
			.filter(crate::db::entities::books::Column::AuthorId.eq(Some(a.id)))
			.all(&state.db)
			.await?
			.len() as i64;

		results.push(AuthorResponse {
			id: a.id,
			name: a.name,
			sort_name: a.sort_name,
			bio: a.bio,
			birth_date: a.birth_date,
			death_date: a.death_date,
			photo_asset_id: a.photo_asset_id,
			book_count: bc as usize,
			created_at: a.created_at.to_string(),
			updated_at: a.updated_at.to_string(),
		});
	}

	Ok(Json(results))
}

async fn create_author(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Json(req): Json<CreateAuthorRequest>,
) -> Result<Json<AuthorResponse>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let now = chrono::Utc::now();

	let author = authors::ActiveModel {
		id: Set(Uuid::new_v4()),
		library_id: Set(library_ids[0]),
		name: Set(req.name),
		sort_name: Set(req.sort_name),
		bio: Set(req.bio),
		birth_date: Set(req.birth_date),
		death_date: Set(req.death_date),
		photo_asset_id: Set(None),
		created_at: Set(now.into()),
		updated_at: Set(now.into()),
	}
	.insert(&state.db)
	.await?;

	Ok(Json(AuthorResponse {
		id: author.id,
		name: author.name,
		sort_name: author.sort_name,
		bio: author.bio,
		birth_date: author.birth_date,
		death_date: author.death_date,
		photo_asset_id: author.photo_asset_id,
		book_count: 0,
		created_at: author.created_at.to_string(),
		updated_at: author.updated_at.to_string(),
	}))
}

async fn get_author(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(author_id): Path<Uuid>,
) -> Result<Json<AuthorResponse>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;

	let author = Authors::find_by_id(author_id)
		.one(&state.db)
		.await?
		.filter(|a| library_ids.contains(&a.library_id))
		.ok_or_else(|| AppError::NotFound("Author not found".into()))?;

	let bc = Books::find()
		.filter(crate::db::entities::books::Column::AuthorId.eq(Some(author.id)))
		.all(&state.db)
		.await?
		.len() as i64;

	Ok(Json(AuthorResponse {
		id: author.id,
		name: author.name,
		sort_name: author.sort_name,
		bio: author.bio,
		birth_date: author.birth_date,
		death_date: author.death_date,
		photo_asset_id: author.photo_asset_id,
		book_count: bc as usize,
		created_at: author.created_at.to_string(),
		updated_at: author.updated_at.to_string(),
	}))
}

async fn update_author(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(author_id): Path<Uuid>,
	Json(req): Json<UpdateAuthorRequest>,
) -> Result<Json<AuthorResponse>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;

	let existing = Authors::find_by_id(author_id)
		.one(&state.db)
		.await?
		.filter(|a| library_ids.contains(&a.library_id))
		.ok_or_else(|| AppError::NotFound("Author not found".into()))?;

	let now = chrono::Utc::now();
	let mut active: authors::ActiveModel = existing.into();
	active.updated_at = Set(now.into());
	if let Some(name) = req.name {
		active.name = Set(name);
	}
	if let Some(sn) = req.sort_name {
		active.sort_name = Set(Some(sn));
	}
	if let Some(bio) = req.bio {
		active.bio = Set(Some(bio));
	}
	if let Some(birth) = req.birth_date {
		active.birth_date = Set(Some(birth));
	}
	if let Some(death) = req.death_date {
		active.death_date = Set(Some(death));
	}

	let author = active.update(&state.db).await?;
	let bc = Books::find()
		.filter(crate::db::entities::books::Column::AuthorId.eq(Some(author.id)))
		.all(&state.db)
		.await?
		.len() as i64;

	Ok(Json(AuthorResponse {
		id: author.id,
		name: author.name,
		sort_name: author.sort_name,
		bio: author.bio,
		birth_date: author.birth_date,
		death_date: author.death_date,
		photo_asset_id: author.photo_asset_id,
		book_count: bc as usize,
		created_at: author.created_at.to_string(),
		updated_at: author.updated_at.to_string(),
	}))
}

async fn delete_author(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(author_id): Path<Uuid>,
) -> Result<Json<()>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;

	let existing = Authors::find_by_id(author_id)
		.one(&state.db)
		.await?
		.filter(|a| library_ids.contains(&a.library_id))
		.ok_or_else(|| AppError::NotFound("Author not found".into()))?;

	Books::update_many()
		.col_expr(
			crate::db::entities::books::Column::AuthorId,
			sea_orm::sea_query::Expr::val(Option::<Uuid>::None),
		)
		.filter(crate::db::entities::books::Column::AuthorId.eq(Some(existing.id)))
		.exec(&state.db)
		.await?;

	authors::Entity::delete_by_id(existing.id).exec(&state.db).await?;

	Ok(Json(()))
}
