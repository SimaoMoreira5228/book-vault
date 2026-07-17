use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::SharedState;
use crate::auth::session::SessionManager;

pub struct AuthenticatedUser {
	pub user_id: Uuid,
	pub session_id: Uuid,
	pub is_admin: bool,
}

pub enum AuthError {
	MissingSession,
	InvalidSession,
}

impl IntoResponse for AuthError {
	fn into_response(self) -> Response {
		let status = match self {
			AuthError::MissingSession | AuthError::InvalidSession => StatusCode::UNAUTHORIZED,
		};
		(status, axum::Json(serde_json::json!({ "error": "unauthorized" }))).into_response()
	}
}

impl FromRequestParts<SharedState> for AuthenticatedUser {
	type Rejection = AuthError;

	async fn from_request_parts(parts: &mut Parts, state: &SharedState) -> Result<Self, Self::Rejection> {
		let shared_state = state.clone();

		let token = parts
			.headers
			.get(header::COOKIE)
			.and_then(|v| v.to_str().ok())
			.and_then(|cookies| {
				cookies.split(';').find_map(|c| {
					let c = c.trim();
					c.strip_prefix("bv_session=").map(|s| s.to_string())
				})
			})
			.ok_or(AuthError::MissingSession)?;

		let session = SessionManager::validate_session(&shared_state.db, &token)
			.await
			.map_err(|_| AuthError::InvalidSession)?;

		let user = crate::db::entities::users::Entity::find_by_id(session.user_id)
			.one(&shared_state.db)
			.await
			.map_err(|_| AuthError::InvalidSession)?
			.ok_or(AuthError::InvalidSession)?;

		let _ = SessionManager::touch_session(
			&shared_state.db,
			session.id,
			shared_state.config.auth.session_idle_days,
			shared_state.config.auth.session_ttl_days,
		)
		.await;

		Ok(AuthenticatedUser {
			user_id: session.user_id,
			session_id: session.id,
			is_admin: user.is_admin,
		})
	}
}
