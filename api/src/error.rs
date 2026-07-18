use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
	NotFound(String),
	Unauthorized(String),
	Forbidden(String),
	BadRequest(String),
	Conflict(String),
	TooManyRequests(String),
	Internal(String),
	Db(sea_orm::DbErr),
}

impl std::fmt::Display for AppError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
			AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
			AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
			AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
			AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
			AppError::TooManyRequests(msg) => write!(f, "Too many requests: {}", msg),
			AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
			AppError::Db(err) => write!(f, "Database error: {}", err),
		}
	}
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let (status, message) = match &self {
			AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
			AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
			AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
			AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
			AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
			AppError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone()),
			AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
			AppError::Db(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", err)),
		};

		(status, Json(json!({ "error": message }))).into_response()
	}
}

impl From<sea_orm::DbErr> for AppError {
	fn from(err: sea_orm::DbErr) -> Self {
		AppError::Db(err)
	}
}

impl From<serde_json::Error> for AppError {
	fn from(err: serde_json::Error) -> Self {
		AppError::Internal(format!("Serialization error: {}", err))
	}
}

impl From<std::io::Error> for AppError {
	fn from(err: std::io::Error) -> Self {
		AppError::Internal(format!("IO error: {}", err))
	}
}

impl From<zip::result::ZipError> for AppError {
	fn from(err: zip::result::ZipError) -> Self {
		AppError::Internal(format!("Zip error: {}", err))
	}
}
