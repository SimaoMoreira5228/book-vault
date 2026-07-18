use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::{book_ir, book_revisions};
use crate::ir::block::Block;
use crate::{AppError, SharedState};

#[derive(Serialize)]
pub struct RevisionResponse {
	pub id: Uuid,
	pub book_id: Uuid,
	pub section_id: Uuid,
	pub version: i64,
	pub created_at: String,
}

#[derive(Serialize)]
pub struct SectionRestoreResponse {
	pub message: String,
	pub version: i64,
}

async fn save_section(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path((book_id, section_id)): Path<(Uuid, Uuid)>,
	Json(req): Json<serde_json::Value>,
) -> Result<Json<SectionRestoreResponse>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}
	if book.format != "native" {
		return Err(AppError::BadRequest("Can only edit native books".into()));
	}

	let ir_row = BookIr::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	let mut ir: crate::ir::BookIr = crate::ingest::deserialize_ir(&ir_row.payload)?;

	let blocks: Vec<Block> =
		serde_json::from_value(req["blocks"].clone()).map_err(|e| AppError::BadRequest(format!("Invalid blocks: {}", e)))?;

	let mut found = false;
	for section in &mut ir.spine {
		if section.id == section_id {
			section.blocks = blocks.clone();
			found = true;
			break;
		}
	}
	if !found {
		return Err(AppError::NotFound("Section not found".into()));
	}

	let payload = crate::ingest::serialize_ir(&ir)?;
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

	let mut active: book_ir::ActiveModel = ir_row.into();
	active.payload = Set(payload);
	active.updated_at = Set(now);
	BookIr::update(active).exec(&state.db).await?;

	let latest_version = BookRevisions::find()
		.filter(book_revisions::Column::BookId.eq(book_id))
		.filter(book_revisions::Column::SectionId.eq(section_id))
		.order_by_desc(book_revisions::Column::Version)
		.one(&state.db)
		.await?;
	let next_version = latest_version.map(|v| v.version + 1).unwrap_or(1);

	let snapshot_val = serde_json::to_value(&blocks).map_err(|e| AppError::Internal(format!("Serialize blocks: {}", e)))?;

	BookRevisions::insert(book_revisions::ActiveModel {
		id: Set(Uuid::now_v7()),
		book_id: Set(book_id),
		section_id: Set(section_id),
		snapshot: Set(snapshot_val),
		version: Set(next_version),
		created_at: Set(now),
	})
	.exec(&state.db)
	.await?;

	Ok(Json(SectionRestoreResponse {
		message: "section saved".to_string(),
		version: next_version,
	}))
}

async fn list_revisions(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Vec<RevisionResponse>>, AppError> {
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let revisions = BookRevisions::find()
		.filter(book_revisions::Column::BookId.eq(book_id))
		.order_by_desc(book_revisions::Column::Version)
		.limit(100)
		.all(&state.db)
		.await?;

	Ok(Json(
		revisions
			.into_iter()
			.map(|r| RevisionResponse {
				id: r.id,
				book_id: r.book_id,
				section_id: r.section_id,
				version: r.version,
				created_at: r.created_at.to_string(),
			})
			.collect(),
	))
}

async fn get_revision(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(revision_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	let revision = BookRevisions::find_by_id(revision_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Revision not found".into()))?;

	let book = Books::find_by_id(revision.book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	Ok(Json(serde_json::json!({
		"id": revision.id,
		"book_id": revision.book_id,
		"section_id": revision.section_id,
		"version": revision.version,
		"snapshot": revision.snapshot,
		"created_at": revision.created_at,
	})))
}

async fn restore_revision(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(revision_id): Path<Uuid>,
) -> Result<Json<SectionRestoreResponse>, AppError> {
	let revision = BookRevisions::find_by_id(revision_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Revision not found".into()))?;

	let book = Books::find_by_id(revision.book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let ir_row = BookIr::find()
		.filter(book_ir::Column::BookId.eq(revision.book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	let mut ir: crate::ir::BookIr = crate::ingest::deserialize_ir(&ir_row.payload)?;

	let restored_blocks: Vec<Block> = serde_json::from_value(revision.snapshot.clone())
		.map_err(|e| AppError::Internal(format!("Parse snapshot: {}", e)))?;

	let mut found = false;
	for section in &mut ir.spine {
		if section.id == revision.section_id {
			section.blocks = restored_blocks;
			found = true;
			break;
		}
	}
	if !found {
		return Err(AppError::NotFound("Section no longer exists".into()));
	}

	let payload = crate::ingest::serialize_ir(&ir)?;
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

	let mut active: book_ir::ActiveModel = ir_row.into();
	active.payload = Set(payload);
	active.updated_at = Set(now);
	BookIr::update(active).exec(&state.db).await?;

	let next_version = revision.version + 1;
	let snapshot_val = revision.snapshot.clone();

	BookRevisions::insert(book_revisions::ActiveModel {
		id: Set(Uuid::now_v7()),
		book_id: Set(revision.book_id),
		section_id: Set(revision.section_id),
		snapshot: Set(snapshot_val),
		version: Set(next_version),
		created_at: Set(now),
	})
	.exec(&state.db)
	.await?;

	Ok(Json(SectionRestoreResponse {
		message: "revision restored".to_string(),
		version: revision.version,
	}))
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/{id}/sections/{section_id}", put(save_section))
		.route("/{id}/revisions", get(list_revisions))
		.route("/revisions/{id}", get(get_revision))
		.route("/revisions/{id}/restore", post(restore_revision))
}
