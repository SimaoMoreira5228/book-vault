use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::storage::AssetService;
use crate::{AppError, SharedState};

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/{id}/raw", get(raw_source))
		.route("/{id}/comic/pages", get(comic_page_list))
		.route("/{id}/comic/page/{n}", get(comic_page))
		.route("/{id}/assets/{asset_id}", get(asset_image))
		.route("/{id}/cover", get(book_cover))
}

async fn raw_source(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let data = state.storage.get(&book_id.to_string()).await?;
	let mime = match book.format.as_str() {
		"epub" => "application/epub+zip",
		"pdf" => "application/pdf",
		"cbz" => "application/vnd.comicbook+zip",
		"mobi_raw" | "mobi" => "application/x-mobipocket-ebook",
		_ => "application/octet-stream",
	};

	let filename = format!("{}.{}", book.title.replace(' ', "_"), book.format);
	Ok((
		StatusCode::OK,
		[
			(header::CONTENT_TYPE, mime.to_string()),
			(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
		],
		data,
	))
}

async fn comic_page_list(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let assets = AssetService::list_assets(&state.db, book_id).await?;
	let pages: Vec<serde_json::Value> = assets
		.into_iter()
		.enumerate()
		.map(|(i, a)| {
			serde_json::json!({
				"page": i + 1,
				"asset_id": a.id,
				"mime_type": a.mime_type,
			})
		})
		.collect();
	Ok(Json(pages))
}

async fn comic_page(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path((book_id, page_num)): Path<(Uuid, u32)>,
) -> Result<impl IntoResponse, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let assets = AssetService::list_assets(&state.db, book_id).await?;
	let idx = (page_num as usize).saturating_sub(1);
	let asset = assets.get(idx).ok_or_else(|| AppError::NotFound("Page not found".into()))?;

	let data = AssetService::get_image_data(&(*state.storage), asset.id).await?;
	Ok((StatusCode::OK, [(header::CONTENT_TYPE, asset.mime_type.clone())], data))
}

async fn asset_image(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path((book_id, asset_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let asset = AssetService::get_asset(&state.db, asset_id).await?;
	if asset.book_id != book_id {
		return Err(AppError::NotFound("Asset not found for this book".into()));
	}

	let data = AssetService::get_image_data(&(*state.storage), asset_id).await?;
	Ok((StatusCode::OK, [(header::CONTENT_TYPE, asset.mime_type)], data))
}

async fn book_cover(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let assets = AssetService::list_assets(&state.db, book_id).await?;
	let cover = assets
		.into_iter()
		.find(|a| a.kind == "cover")
		.ok_or_else(|| AppError::NotFound("No cover found for this book".into()))?;

	let data = AssetService::get_image_data(&(*state.storage), cover.id).await?;
	Ok((StatusCode::OK, [(header::CONTENT_TYPE, cover.mime_type)], data))
}
