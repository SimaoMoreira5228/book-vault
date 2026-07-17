use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::prelude::*;
use crate::db::entities::{books, job_queue, libraries};
use crate::{AppError, SharedState};
use sea_orm::{
    ColumnTrait, EntityTrait, ModelTrait, QueryFilter, QueryOrder, Set,
};

#[derive(Serialize, TS)]
#[ts(export)]
pub struct BookResponse {
    pub id: Uuid,
    pub library_id: Uuid,
    pub title: String,
    pub author: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<i64>,
    pub page_count: Option<i64>,
    pub read_status: String,
    pub rating: Option<i64>,
    pub format: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CreateBookRequest {
    pub title: String,
    pub author: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<i64>,
    pub page_count: Option<i64>,
    pub format: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBookRequest {
    pub title: Option<String>,
    pub author: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<i64>,
    pub page_count: Option<i64>,
    pub read_status: Option<String>,
    pub rating: Option<i64>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub job_id: Uuid,
    pub book_id: Uuid,
}

impl From<books::Model> for BookResponse {
    fn from(b: books::Model) -> Self {
        Self {
            id: b.id,
            library_id: b.library_id,
            title: b.title,
            author: b.author,
            isbn: b.isbn,
            language: b.language,
            publisher: b.publisher,
            series: b.series,
            series_index: b.series_index,
            page_count: b.page_count,
            read_status: b.read_status,
            rating: b.rating,
            format: b.format,
            created_at: b.created_at.to_string(),
            updated_at: b.updated_at.to_string(),
        }
    }
}

fn detect_format(data: &[u8], filename: &str) -> &'static str {
    if data.len() > 4 && &data[0..4] == b"%PDF" {
        return "pdf";
    }
    if data.len() > 2 && data[0..2] == [0x1f, 0x8b] {
        return "bvir";
    }
    if data.len() > 4 {
        if let Ok(header) = std::str::from_utf8(&data[0..4]) {
            if header == "PK\x03\x04" {
                let lower = filename.to_lowercase();
                if lower.ends_with(".cbz") {
                    return "cbz";
                }
                return "epub";
            }
        }
    }
    "unknown"
}

pub async fn get_user_library_ids(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let libraries = Libraries::find()
        .filter(libraries::Column::OwnerId.eq(user_id))
        .all(db)
        .await?;
    Ok(libraries.into_iter().map(|l| l.id).collect())
}

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_books))
        .route("/", post(create_book))
        .route("/upload", post(upload_book))
        .route("/{id}", get(get_book))
        .route("/{id}", put(update_book))
        .route("/{id}", delete(delete_book))
}

async fn list_books(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<BookResponse>>, AppError> {
    let library_ids = get_user_library_ids(&state.db, auth.user_id).await?;
    let books = Books::find()
        .filter(books::Column::LibraryId.is_in(library_ids))
        .order_by_desc(books::Column::UpdatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(books.into_iter().map(BookResponse::from).collect()))
}

async fn get_book(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
) -> Result<Json<BookResponse>, AppError> {
    let book = Books::find_by_id(book_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;
    let library_ids = get_user_library_ids(&state.db, auth.user_id).await?;
    if !library_ids.contains(&book.library_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    Ok(Json(book.into()))
}

async fn create_book(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Json(req): Json<CreateBookRequest>,
) -> Result<(StatusCode, Json<BookResponse>), AppError> {
    let library_ids = get_user_library_ids(&state.db, auth.user_id).await?;
    let library_id = library_ids
        .first()
        .copied()
        .ok_or_else(|| AppError::BadRequest("No library found".into()))?;

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let book = books::ActiveModel {
        id: Set(Uuid::now_v7()),
        library_id: Set(library_id),
        title: Set(req.title),
        author: Set(req.author),
        isbn: Set(req.isbn),
        language: Set(req.language),
        publisher: Set(req.publisher),
        series: Set(req.series),
        series_index: Set(req.series_index),
        page_count: Set(req.page_count),
        read_status: Set("unread".to_string()),
        rating: Set(None),
        format: Set(req.format.unwrap_or_else(|| "native".to_string())),
        source_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let book = books::Entity::insert(book).exec_with_returning(&state.db).await?;
    Ok((StatusCode::CREATED, Json(book.into())))
}

async fn upload_book(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), AppError> {
    let library_ids = get_user_library_ids(&state.db, auth.user_id).await?;
    let library_id = library_ids
        .first()
        .copied()
        .ok_or_else(|| AppError::BadRequest("No library found".into()))?;

    let mut file_data = Vec::new();
    let mut filename = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.file_name().unwrap_or("file").to_string();
        filename = name;
        let data = field.bytes().await.map_err(|_| AppError::BadRequest("Failed to read file".into()))?;
        file_data = data.to_vec();
    }

    if file_data.is_empty() {
        return Err(AppError::BadRequest("No file uploaded".into()));
    }

    let fmt = detect_format(&file_data, &filename);
    if fmt == "unknown" {
        return Err(AppError::BadRequest("Unsupported file format".into()));
    }

    let book_id = Uuid::now_v7();
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

    state.storage.put(&book_id.to_string(), &file_data).await?;

    let title = filename.rsplit('.').next().unwrap_or(&filename).to_string();
    books::Entity::insert(books::ActiveModel {
        id: Set(book_id),
        library_id: Set(library_id),
        title: Set(title),
        author: Set(None),
        isbn: Set(None),
        language: Set(None),
        publisher: Set(None),
        series: Set(None),
        series_index: Set(None),
        page_count: Set(None),
        read_status: Set("pending".to_string()),
        rating: Set(None),
        format: Set(fmt.to_string()),
        source_hash: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .exec(&state.db)
    .await?;

    let job_kind = format!("ingest_{}", fmt);
    let job_id = Uuid::now_v7();
    let payload = serde_json::json!({ "book_id": book_id.to_string() });

    job_queue::Entity::insert(job_queue::ActiveModel {
        id: Set(job_id),
        kind: Set(job_kind),
        status: Set("pending".to_string()),
        payload: Set(payload),
        error: Set(None),
        retry_count: Set(0),
        max_retries: Set(3),
        scheduled_at: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        created_at: Set(now),
    })
    .exec(&state.db)
    .await?;

    Ok((StatusCode::ACCEPTED, Json(UploadResponse { job_id, book_id })))
}

async fn update_book(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
    Json(req): Json<UpdateBookRequest>,
) -> Result<Json<BookResponse>, AppError> {
    let book = Books::find_by_id(book_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;
    let library_ids = get_user_library_ids(&state.db, auth.user_id).await?;
    if !library_ids.contains(&book.library_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let mut active: books::ActiveModel = book.into();
    if let Some(v) = req.title { active.title = Set(v); }
    if let Some(v) = req.author { active.author = Set(Some(v)); }
    if let Some(v) = req.isbn { active.isbn = Set(Some(v)); }
    if let Some(v) = req.language { active.language = Set(Some(v)); }
    if let Some(v) = req.publisher { active.publisher = Set(Some(v)); }
    if let Some(v) = req.series { active.series = Set(Some(v)); }
    if let Some(v) = req.series_index { active.series_index = Set(Some(v)); }
    if let Some(v) = req.page_count { active.page_count = Set(Some(v)); }
    if let Some(v) = req.read_status { active.read_status = Set(v); }
    if let Some(v) = req.rating { active.rating = Set(Some(v)); }
    active.updated_at = Set(now);

    let book = books::Entity::update(active).exec(&state.db).await?;
    Ok(Json(book.into()))
}

async fn delete_book(
    State(state): State<SharedState>,
    auth: AuthenticatedUser,
    Path(book_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let book = Books::find_by_id(book_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;
    let library_ids = get_user_library_ids(&state.db, auth.user_id).await?;
    if !library_ids.contains(&book.library_id) {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    book.delete(&state.db).await?;
    state.storage.delete(&book_id.to_string()).await.ok();
    Ok(Json(serde_json::json!({ "message": "book deleted" })))
}
