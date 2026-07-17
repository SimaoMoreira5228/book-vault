use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::book_ir;
use crate::db::entities::prelude::*;
use crate::export::{EpubExporter, Exporter, MarkdownExporter, PdfExporter};
use crate::{AppError, SharedState};

#[derive(Deserialize)]
pub struct ExportParams {
	format: Option<String>,
}

pub fn routes() -> Router<SharedState> {
	Router::new().route("/{id}/export", get(export_book))
}

async fn export_book(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
	Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let ir_row = BookIr::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	let ir: crate::ir::BookIr =
		rmp_serde::from_slice(&ir_row.payload).map_err(|e| AppError::Internal(format!("Failed to decode IR: {}", e)))?;

	let fmt = params.format.as_deref().unwrap_or("bvir");

	let (bytes, content_type) = match fmt {
		"epub" => {
			let exporter = EpubExporter;
			let data = exporter.export(&ir).await?;
			(data, "application/epub+zip".to_string())
		}
		"pdf" => {
			let exporter = PdfExporter;
			let data = exporter.export(&ir).await?;
			(data, "application/pdf".to_string())
		}
		"md" | "markdown" => {
			let exporter = MarkdownExporter;
			let data = exporter.export(&ir).await?;
			(data, "text/markdown".to_string())
		}
		_ => {
			let data = crate::ingest::serialize_ir(&ir)?;
			(data, "application/x-bvir".to_string())
		}
	};

	let filename = format!("{}.{}", book.title.replace(' ', "_"), fmt);
	Ok((
		StatusCode::OK,
		[
			(
				axum::http::header::CONTENT_DISPOSITION,
				format!("attachment; filename=\"{}\"", filename),
			),
			(axum::http::header::CONTENT_TYPE, content_type),
		],
		bytes,
	))
}
