use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::{books, kobo_tokens, reading_progress};
use crate::export::{EpubExporter, Exporter, PdfExporter};
use crate::{AppError, SharedState};
use sea_orm::ModelTrait;

#[derive(Deserialize)]
pub struct ReadingStatePayload {
	#[serde(default)]
	pub book_id: String,
	#[serde(default)]
	pub percentage: f64,
	#[serde(default)]
	pub status: String,
	#[serde(default)]
	pub current_page: u32,
	#[serde(default)]
	pub total_pages: u32,
}

fn kobo_json(body: Value) -> Response {
	(
		StatusCode::OK,
		[("content-type", "application/json")],
		serde_json::to_string(&body).unwrap_or_default(),
	)
		.into_response()
}

fn kobo_err(status: StatusCode, msg: &str) -> Response {
	(
		StatusCode::OK,
		[("content-type", "application/json")],
		serde_json::json!({ "error": msg, "code": status.as_u16() }).to_string(),
	)
		.into_response()
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/{token}/v1/initialization", get(kobo_init))
		.route("/{token}/v1/library", get(kobo_library))
		.route("/{token}/v1/entitlements", get(kobo_entitlements))
		.route("/{token}/v1/readingstate", get(kobo_get_reading_state))
		.route("/{token}/v1/readingstate", post(kobo_set_reading_state))
		.route("/{token}/v1/cover/{book_id}", get(kobo_cover))
		.route("/{token}/v1/download/{book_id}", get(kobo_download))
}

pub fn admin_routes() -> Router<SharedState> {
	Router::new()
		.route("/kobo-tokens", get(list_kobo_tokens))
		.route("/kobo-tokens", post(create_kobo_token))
		.route("/kobo-tokens/{id}", delete(delete_kobo_token))
}

async fn resolve_token(state: &SharedState, token: &str) -> Result<Uuid, Response> {
	let row = KoboTokens::find()
		.filter(kobo_tokens::Column::Token.eq(token))
		.one(&state.db)
		.await
		.map_err(|_| kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
		.ok_or_else(|| kobo_err(StatusCode::UNAUTHORIZED, "invalid token"))?;
	Ok(row.user_id)
}

async fn kobo_init(State(state): State<SharedState>, Path(token): Path<String>) -> Response {
	let user_id = match resolve_token(&state, &token).await {
		Ok(uid) => uid,
		Err(e) => return e,
	};

	let user = match Users::find_by_id(user_id).one(&state.db).await {
		Ok(Some(u)) => u,
		_ => return kobo_err(StatusCode::NOT_FOUND, "user not found"),
	};

	let libraries = Libraries::find()
		.filter(crate::db::entities::libraries::Column::OwnerId.eq(user_id))
		.all(&state.db)
		.await
		.unwrap_or_default();

	kobo_json(serde_json::json!({
		"user": {
			"id": user.id,
			"display_name": user.display_name,
			"email": user.email,
		},
		"libraries": libraries.iter().map(|l| serde_json::json!({
			"id": l.id,
			"name": l.name,
		})).collect::<Vec<_>>(),
		"device_token": token,
	}))
}

async fn kobo_library(State(state): State<SharedState>, Path(token): Path<String>) -> Response {
	let user_id = match resolve_token(&state, &token).await {
		Ok(uid) => uid,
		Err(e) => return e,
	};

	let library_ids = match crate::routes::books::get_user_library_ids(&state.db, user_id).await {
		Ok(ids) => ids,
		Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "db error"),
	};

	let all_books = Books::find()
		.filter(books::Column::LibraryId.is_in(library_ids))
		.order_by_desc(books::Column::UpdatedAt)
		.all(&state.db)
		.await
		.unwrap_or_default();

	let entries: Vec<Value> = all_books
		.into_iter()
		.map(|b| {
			let fmt = match b.format.as_str() {
				"epub" => "application/epub+zip",
				"pdf" => "application/pdf",
				"mobi_raw" | "azw" => "application/x-mobipocket-ebook",
				"cbz" => "application/x-cbz",
				_ => "application/epub+zip",
			};
			serde_json::json!({
				"id": b.id,
				"title": b.title,
				"author": b.author,
				"format": b.format,
				"mime": fmt,
				"last_modified": b.updated_at,
				"cover_url": format!("/api/kobo/{token}/v1/cover/{}", b.id),
				"download_url": format!("/api/kobo/{token}/v1/download/{}", b.id),
			})
		})
		.collect();

	kobo_json(serde_json::json!({ "books": entries }))
}

async fn kobo_entitlements(State(state): State<SharedState>, Path(token): Path<String>) -> Response {
	let user_id = match resolve_token(&state, &token).await {
		Ok(uid) => uid,
		Err(e) => return e,
	};

	let library_ids = match crate::routes::books::get_user_library_ids(&state.db, user_id).await {
		Ok(ids) => ids,
		Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "db error"),
	};

	let all_books = Books::find()
		.filter(books::Column::LibraryId.is_in(library_ids))
		.all(&state.db)
		.await
		.unwrap_or_default();

	let entitlements: Vec<Value> = all_books
		.into_iter()
		.map(|b| {
			serde_json::json!({
				"book_id": b.id,
				"title": b.title,
				"download_url": format!("/api/kobo/{token}/v1/download/{}", b.id),
				"mime": match b.format.as_str() {
					"epub" => "application/epub+zip",
					"pdf" => "application/pdf",
					_ => "application/epub+zip",
				},
			})
		})
		.collect();

	kobo_json(serde_json::json!({ "entitlements": entitlements }))
}

async fn kobo_get_reading_state(State(state): State<SharedState>, Path(token): Path<String>) -> Response {
	let user_id = match resolve_token(&state, &token).await {
		Ok(uid) => uid,
		Err(e) => return e,
	};

	let _library_ids = match crate::routes::books::get_user_library_ids(&state.db, user_id).await {
		Ok(ids) => ids,
		Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "db error"),
	};

	let progress = ReadingProgress::find()
		.filter(reading_progress::Column::UserId.eq(user_id))
		.all(&state.db)
		.await
		.unwrap_or_default();

	let states: Vec<Value> = progress
		.into_iter()
		.map(|p| {
			serde_json::json!({
				"book_id": p.book_id,
				"percentage": p.percentage,
				"last_modified": p.updated_at,
			})
		})
		.collect();

	kobo_json(serde_json::json!({ "reading_states": states }))
}

async fn kobo_set_reading_state(
	State(state): State<SharedState>,
	Path(token): Path<String>,
	Json(payload): Json<ReadingStatePayload>,
) -> Response {
	let user_id = match resolve_token(&state, &token).await {
		Ok(uid) => uid,
		Err(e) => return e,
	};

	let book_id = match Uuid::try_parse(&payload.book_id) {
		Ok(id) => id,
		Err(_) => return kobo_err(StatusCode::BAD_REQUEST, "invalid book_id"),
	};

	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	let pct = payload.percentage.clamp(0.0, 100.0);

	let existing = ReadingProgress::find()
		.filter(
			sea_orm::Condition::all()
				.add(reading_progress::Column::UserId.eq(user_id))
				.add(reading_progress::Column::BookId.eq(book_id)),
		)
		.one(&state.db)
		.await
		.unwrap_or(None);

	if let Some(ep) = existing {
		let mut active: reading_progress::ActiveModel = ep.into();
		active.percentage = Set(pct);
		active.updated_at = Set(now);
		let _ = ReadingProgress::update(active).exec(&state.db).await;
	} else {
		let _ = ReadingProgress::insert(reading_progress::ActiveModel {
			id: Set(Uuid::now_v7()),
			user_id: Set(user_id),
			book_id: Set(book_id),
			section_id: Set(Uuid::nil()),
			block_index: Set(0),
			char_offset: Set(0),
			percentage: Set(pct),
			updated_at: Set(now),
		})
		.exec(&state.db)
		.await;
	}

	kobo_json(serde_json::json!({ "message": "reading state saved", "book_id": book_id }))
}

async fn kobo_cover(State(state): State<SharedState>, Path((token, book_id)): Path<(String, Uuid)>) -> Response {
	match resolve_token(&state, &token).await {
		Ok(_) => {}
		Err(e) => return e,
	}

	let redirect_url = format!("/api/v1/books/{}/cover", book_id);
	(StatusCode::FOUND, [(header::LOCATION, redirect_url.as_str())]).into_response()
}

async fn kobo_download(State(state): State<SharedState>, Path((token, book_id)): Path<(String, Uuid)>) -> Response {
	let user_id = match resolve_token(&state, &token).await {
		Ok(uid) => uid,
		Err(e) => return e,
	};

	let library_ids = match crate::routes::books::get_user_library_ids(&state.db, user_id).await {
		Ok(ids) => ids,
		Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "db error"),
	};

	let book = match Books::find_by_id(book_id).one(&state.db).await {
		Ok(Some(b)) => b,
		_ => return kobo_err(StatusCode::NOT_FOUND, "book not found"),
	};

	if !library_ids.contains(&book.library_id) {
		return kobo_err(StatusCode::FORBIDDEN, "access denied");
	}

	let ir_row = match BookIr::find()
		.filter(crate::db::entities::book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await
	{
		Ok(Some(r)) => r,
		_ => return kobo_err(StatusCode::NOT_FOUND, "book not found"),
	};

	let ir = match crate::ingest::deserialize_ir(&ir_row.payload) {
		Ok(ir) => ir,
		Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "corrupt book"),
	};

	let fmt = &book.format;
	let (bytes, mime, ext) = match fmt.as_str() {
		"pdf" => {
			let e = PdfExporter;
			match e.export(&ir).await {
				Ok(b) => (b, "application/pdf", "pdf"),
				Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "export failed"),
			}
		}
		_ => {
			let e = EpubExporter;
			match e.export(&ir).await {
				Ok(b) => (b, "application/epub+zip", "epub"),
				Err(_) => return kobo_err(StatusCode::INTERNAL_SERVER_ERROR, "export failed"),
			}
		}
	};

	let filename = format!("{}_{}.{}", book.title.replace(' ', "_"), book_id, ext);
	(
		StatusCode::OK,
		[
			(header::CONTENT_TYPE, mime),
			(header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename)),
		],
		bytes,
	)
		.into_response()
}

#[derive(Serialize)]
pub struct KoboTokenResponse {
	pub id: Uuid,
	pub token: String,
	pub device_name: Option<String>,
	pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
	pub device_name: Option<String>,
}

async fn list_kobo_tokens(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
) -> Result<Json<Vec<KoboTokenResponse>>, AppError> {
	let tokens = KoboTokens::find()
		.filter(kobo_tokens::Column::UserId.eq(auth.user_id))
		.all(&state.db)
		.await?;

	Ok(Json(
		tokens
			.into_iter()
			.map(|t| KoboTokenResponse {
				id: t.id,
				token: t.token,
				device_name: t.device_name,
				created_at: t.created_at.to_string(),
			})
			.collect(),
	))
}

async fn create_kobo_token(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Json(req): Json<CreateTokenRequest>,
) -> Result<Json<KoboTokenResponse>, AppError> {
	let token = generate_kobo_token();
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

	let row = KoboTokens::insert(kobo_tokens::ActiveModel {
		id: Set(Uuid::now_v7()),
		user_id: Set(auth.user_id),
		token: Set(token.clone()),
		device_name: Set(req.device_name),
		created_at: Set(now),
	})
	.exec_with_returning(&state.db)
	.await?;

	Ok(Json(KoboTokenResponse {
		id: row.id,
		token: row.token,
		device_name: row.device_name,
		created_at: row.created_at.to_string(),
	}))
}

async fn delete_kobo_token(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(token_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
	let existing = KoboTokens::find_by_id(token_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Token not found".into()))?;

	if existing.user_id != auth.user_id && !auth.is_admin {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	existing.delete(&state.db).await?;
	Ok(Json(serde_json::json!({ "message": "token revoked" })))
}

fn generate_kobo_token() -> String {
	use rand::Rng;
	let mut rng = rand::rng();
	(0..32).map(|_| format!("{:02x}", rng.random::<u8>())).collect()
}
