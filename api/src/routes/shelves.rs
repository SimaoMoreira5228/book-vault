use axum::extract::{Path, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::{books, shelf_entries, shelves};
use crate::shelves::eval;
use crate::{AppError, SharedState};

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ShelfResponse {
	pub id: Uuid,
	pub library_id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub kind: String,
	pub book_count: usize,
	pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateShelfRequest {
	pub name: String,
	pub description: Option<String>,
	pub kind: Option<String>,
	pub rule_ast: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ShelfBookResponse {
	pub book_id: Uuid,
	pub title: String,
	pub author: Option<String>,
	pub read_status: String,
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/", get(list_shelves))
		.route("/", post(create_shelf))
		.route("/{id}", get(get_shelf))
		.route("/{id}", put(update_shelf))
		.route("/{id}", delete(delete_shelf))
		.route("/{id}/books", get(list_shelf_books))
		.route("/{id}/books", post(add_book_to_shelf))
		.route("/{id}/books/{book_id}", delete(remove_book_from_shelf))
}

async fn list_shelves(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
) -> Result<Json<Vec<ShelfResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let shelf_rows = Shelves::find()
		.filter(shelves::Column::LibraryId.is_in(library_ids))
		.order_by_desc(shelves::Column::CreatedAt)
		.all(&state.db)
		.await?;

	let mut resp = Vec::new();
	for s in shelf_rows {
		let count = if s.kind == "dynamic" {
			if let Some(ref ast) = s.rule_ast {
				let condition = eval::build_condition(ast)?;
				Books::find()
					.filter(
						books::Column::LibraryId
							.is_in(crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?),
					)
					.filter(condition)
					.count(&state.db)
					.await? as usize
			} else {
				0
			}
		} else {
			ShelfEntries::find()
				.filter(shelf_entries::Column::ShelfId.eq(s.id))
				.count(&state.db)
				.await? as usize
		};
		resp.push(ShelfResponse {
			id: s.id,
			library_id: s.library_id,
			name: s.name,
			description: s.description,
			kind: s.kind,
			book_count: count,
			created_at: s.created_at.to_string(),
		});
	}
	Ok(Json(resp))
}

async fn list_shelf_books(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(shelf_id): Path<Uuid>,
) -> Result<Json<Vec<ShelfBookResponse>>, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&shelf.library_id) && shelf.owner_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let book_rows = if shelf.kind == "dynamic" {
		if let Some(ref ast) = shelf.rule_ast {
			let condition = eval::build_condition(ast)?;
			Books::find()
				.filter(books::Column::LibraryId.is_in(library_ids))
				.filter(condition)
				.order_by_desc(books::Column::UpdatedAt)
				.all(&state.db)
				.await?
		} else {
			Vec::new()
		}
	} else {
		let entries = ShelfEntries::find()
			.filter(shelf_entries::Column::ShelfId.eq(shelf_id))
			.all(&state.db)
			.await?;
		let book_ids: Vec<Uuid> = entries.into_iter().map(|e| e.book_id).collect();
		if book_ids.is_empty() {
			Vec::new()
		} else {
			Books::find().filter(books::Column::Id.is_in(book_ids)).all(&state.db).await?
		}
	};

	Ok(Json(
		book_rows
			.into_iter()
			.map(|b| ShelfBookResponse {
				book_id: b.id,
				title: b.title,
				author: b.author,
				read_status: b.read_status,
			})
			.collect(),
	))
}

async fn create_shelf(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Json(req): Json<CreateShelfRequest>,
) -> Result<Json<ShelfResponse>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let library_id = library_ids
		.first()
		.copied()
		.ok_or_else(|| AppError::BadRequest("No library found".into()))?;

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let shelf = shelves::ActiveModel {
		id: Set(Uuid::now_v7()),
		name: Set(req.name),
		description: Set(req.description),
		kind: Set(req.kind.unwrap_or_else(|| "static".to_string())),
		rule_ast: Set(req.rule_ast),
		library_id: Set(library_id),
		owner_id: Set(auth.user_id),
		created_at: Set(now),
	};

	let shelf = shelves::Entity::insert(shelf).exec_with_returning(&state.db).await?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let count = if shelf.kind == "dynamic" {
		if let Some(ref ast) = shelf.rule_ast {
			let condition = eval::build_condition(ast)?;
			Books::find()
				.filter(books::Column::LibraryId.is_in(library_ids))
				.filter(condition)
				.count(&state.db)
				.await? as usize
		} else {
			0
		}
	} else {
		0
	};

	Ok(Json(ShelfResponse {
		id: shelf.id,
		library_id: shelf.library_id,
		name: shelf.name,
		description: shelf.description,
		kind: shelf.kind,
		book_count: count,
		created_at: shelf.created_at.to_string(),
	}))
}

async fn get_shelf(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(shelf_id): Path<Uuid>,
) -> Result<Json<ShelfResponse>, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&shelf.library_id) && shelf.owner_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let count = if shelf.kind == "dynamic" {
		if let Some(ref ast) = shelf.rule_ast {
			let condition = eval::build_condition(ast)?;
			Books::find()
				.filter(books::Column::LibraryId.is_in(library_ids))
				.filter(condition)
				.count(&state.db)
				.await? as usize
		} else {
			0
		}
	} else {
		ShelfEntries::find()
			.filter(shelf_entries::Column::ShelfId.eq(shelf.id))
			.count(&state.db)
			.await? as usize
	};

	Ok(Json(ShelfResponse {
		id: shelf.id,
		library_id: shelf.library_id,
		name: shelf.name,
		description: shelf.description,
		kind: shelf.kind,
		book_count: count,
		created_at: shelf.created_at.to_string(),
	}))
}

async fn update_shelf(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(shelf_id): Path<Uuid>,
	Json(req): Json<CreateShelfRequest>,
) -> Result<Json<ShelfResponse>, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	if shelf.owner_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let mut active: shelves::ActiveModel = shelf.into();
	active.name = Set(req.name);
	active.description = Set(req.description);
	if let Some(kind) = req.kind {
		active.kind = Set(kind);
	}
	active.rule_ast = Set(req.rule_ast);

	let shelf = shelves::Entity::update(active).exec(&state.db).await?;
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let count = if shelf.kind == "dynamic" {
		if let Some(ref ast) = shelf.rule_ast {
			let condition = eval::build_condition(ast)?;
			Books::find()
				.filter(books::Column::LibraryId.is_in(library_ids))
				.filter(condition)
				.count(&state.db)
				.await? as usize
		} else {
			0
		}
	} else {
		ShelfEntries::find()
			.filter(shelf_entries::Column::ShelfId.eq(shelf.id))
			.count(&state.db)
			.await? as usize
	};

	Ok(Json(ShelfResponse {
		id: shelf.id,
		library_id: shelf.library_id,
		name: shelf.name,
		description: shelf.description,
		kind: shelf.kind,
		book_count: count,
		created_at: shelf.created_at.to_string(),
	}))
}

async fn delete_shelf(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(shelf_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	if shelf.owner_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	shelf.delete(&state.db).await?;
	Ok(Json(serde_json::json!({ "message": "shelf deleted" })))
}

async fn add_book_to_shelf(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(shelf_id): Path<Uuid>,
	Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	if shelf.owner_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}
	if shelf.kind != "static" {
		return Err(AppError::BadRequest("Can only add books to static shelves".into()));
	}

	let book_id: Uuid = serde_json::from_value(body["book_id"].clone())
		.map_err(|_| AppError::BadRequest("Missing or invalid book_id".into()))?;

	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let existing = ShelfEntries::find()
		.filter(shelf_entries::Column::ShelfId.eq(shelf_id))
		.filter(shelf_entries::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?;

	if existing.is_none() {
		let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
		ShelfEntries::insert(shelf_entries::ActiveModel {
			shelf_id: Set(shelf_id),
			book_id: Set(book_id),
			position: Set(0),
			added_at: Set(now),
		})
		.exec(&state.db)
		.await?;
	}

	Ok(Json(serde_json::json!({ "message": "book added to shelf" })))
}

async fn remove_book_from_shelf(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path((shelf_id, book_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	if shelf.owner_id != auth.user_id {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	ShelfEntries::delete_many()
		.filter(shelf_entries::Column::ShelfId.eq(shelf_id))
		.filter(shelf_entries::Column::BookId.eq(book_id))
		.exec(&state.db)
		.await?;

	Ok(Json(serde_json::json!({ "message": "book removed from shelf" })))
}
