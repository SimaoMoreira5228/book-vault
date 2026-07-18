use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::session::SessionManager;
use crate::db::entities::prelude::*;
use crate::db::entities::{libraries, users};
use crate::{AppError, SharedState};

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
	pub email: String,
	pub password: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct RegisterRequest {
	pub email: String,
	pub password: String,
	pub display_name: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct UserResponse {
	pub id: Uuid,
	pub email: String,
	pub display_name: String,
	pub is_admin: bool,
}

#[derive(Serialize)]
pub struct SessionResponse {
	pub id: Uuid,
	pub user_agent: Option<String>,
	pub ip_address: Option<String>,
	pub created_at: String,
	pub last_seen_at: String,
	pub expires_at: String,
	pub is_current: bool,
}

impl From<users::Model> for UserResponse {
	fn from(u: users::Model) -> Self {
		Self {
			id: u.id,
			email: u.email,
			display_name: u.display_name,
			is_admin: u.is_admin,
		}
	}
}

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
	headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).and_then(|cookies| {
		cookies.split(';').find_map(|c| {
			let c = c.trim();
			c.strip_prefix("bv_session=").map(|s| s.to_string())
		})
	})
}

fn set_session_cookie(token: &str, ttl_days: i64) -> String {
	format!(
		"bv_session={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={}",
		token,
		ttl_days * 86400
	)
}

fn clear_session_cookie() -> String {
	"bv_session=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0".to_string()
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/login", post(login_handler))
		.route("/logout", post(logout_handler))
		.route("/register", post(register_handler))
		.route("/profile", get(get_profile).put(update_profile))
		.route("/password", put(change_password))
		.route("/sessions", get(list_sessions))
		.route("/sessions/{id}", delete(revoke_session))
}

async fn login_handler(
	State(state): State<SharedState>,
	headers: HeaderMap,
	Json(req): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<serde_json::Value>), AppError> {
	let ip = headers
		.get("X-Forwarded-For")
		.and_then(|v| v.to_str().ok().map(|s| s.to_string()))
		.or_else(|| headers.get("X-Real-IP").and_then(|v| v.to_str().ok().map(|s| s.to_string())))
		.unwrap_or_else(|| "unknown".to_string());

	state.rate_limiter.check_ip(&ip)?;
	state.rate_limiter.check_email(&req.email)?;

	let user = match Users::find().filter(users::Column::Email.eq(&req.email)).one(&state.db).await {
		Ok(Some(u)) => u,
		Ok(None) => {
			state.rate_limiter.record_failure_ip(&ip);
			state.rate_limiter.record_failure_email(&req.email);
			return Err(AppError::Unauthorized("Invalid email or password".into()));
		}
		Err(e) => return Err(AppError::Db(e)),
	};

	if SessionManager::verify_password(&user.password_hash, &req.password).is_err() {
		state.rate_limiter.record_failure_ip(&ip);
		state.rate_limiter.record_failure_email(&req.email);
		return Err(AppError::Unauthorized("Invalid email or password".into()));
	}

	state.rate_limiter.reset_ip(&ip);
	state.rate_limiter.reset_email(&req.email);

	let user_agent = headers
		.get(header::USER_AGENT)
		.and_then(|v| v.to_str().ok().map(|s| s.to_string()));
	let ip_address = Some(ip);

	let (token, _session) = SessionManager::create_session(
		&state.db,
		user.id,
		user_agent,
		ip_address,
		state.config.auth.session_ttl_days,
		state.config.auth.session_idle_days,
	)
	.await?;

	let cookie = set_session_cookie(&token, state.config.auth.session_ttl_days);
	let user_resp: UserResponse = user.into();

	let mut resp_headers = HeaderMap::new();
	resp_headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

	Ok((
		resp_headers,
		Json(serde_json::json!({
			"user": user_resp,
		})),
	))
}

async fn logout_handler(State(state): State<SharedState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, AppError> {
	let token = extract_session_cookie(&headers).ok_or_else(|| AppError::Unauthorized("No session cookie".into()))?;

	let session = SessionManager::validate_session(&state.db, &token).await?;
	SessionManager::revoke_session(&state.db, session.id).await?;

	Ok(Json(
		serde_json::json!({ "message": "logged out", "cookie": clear_session_cookie() }),
	))
}

async fn register_handler(
	State(state): State<SharedState>,
	Json(req): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, AppError> {
	if state.config.auth.mode == "closed" {
		return Err(AppError::Forbidden("Registration is closed".into()));
	}

	let existing = Users::find()
		.filter(users::Column::Email.eq(&req.email))
		.one(&state.db)
		.await?;

	if existing.is_some() {
		return Err(AppError::Conflict("Email already registered".into()));
	}

	let password_hash = SessionManager::hash_password(&req.password)?;
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

	let user = users::ActiveModel {
		id: Set(Uuid::now_v7()),
		email: Set(req.email),
		password_hash: Set(password_hash),
		display_name: Set(req.display_name),
		is_admin: Set(false),
		created_at: Set(now),
		updated_at: Set(now),
	};

	let user = users::Entity::insert(user).exec_with_returning(&state.db).await?;

	let default_lib = libraries::ActiveModel {
		id: Set(Uuid::now_v7()),
		name: Set("My Library".to_string()),
		description: Set(None),
		owner_id: Set(user.id),
		created_at: Set(now),
	};
	libraries::Entity::insert(default_lib).exec(&state.db).await?;

	Ok(Json(user.into()))
}

async fn list_sessions(
	State(state): State<SharedState>,
	headers: HeaderMap,
) -> Result<Json<Vec<SessionResponse>>, AppError> {
	let token = extract_session_cookie(&headers).ok_or_else(|| AppError::Unauthorized("No session cookie".into()))?;
	let current = SessionManager::validate_session(&state.db, &token).await?;

	let sessions = SessionManager::list_user_sessions(&state.db, current.user_id).await?;

	let resp: Vec<SessionResponse> = sessions
		.into_iter()
		.map(|s| SessionResponse {
			id: s.id,
			user_agent: s.user_agent,
			ip_address: s.ip_address,
			created_at: s.created_at.to_string(),
			last_seen_at: s.last_seen_at.to_string(),
			expires_at: s.expires_at.to_string(),
			is_current: s.id == current.id,
		})
		.collect();

	Ok(Json(resp))
}

async fn revoke_session(
	State(state): State<SharedState>,
	headers: HeaderMap,
	Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
	let token = extract_session_cookie(&headers).ok_or_else(|| AppError::Unauthorized("No session cookie".into()))?;
	let current = SessionManager::validate_session(&state.db, &token).await?;

	let target = Sessions::find_by_id(session_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Session not found".into()))?;

	if target.user_id != current.user_id {
		let user = Users::find_by_id(current.user_id)
			.one(&state.db)
			.await?
			.ok_or_else(|| AppError::NotFound("User not found".into()))?;
		if !user.is_admin {
			return Err(AppError::Forbidden("Cannot revoke another user's session".into()));
		}
	}

	SessionManager::revoke_session(&state.db, session_id).await?;
	Ok(Json(serde_json::json!({ "message": "session revoked" })))
}

#[derive(Deserialize)]
struct UpdateProfileRequest {
	display_name: Option<String>,
}

async fn get_profile(State(state): State<SharedState>, headers: HeaderMap) -> Result<Json<UserResponse>, AppError> {
	let token = extract_session_cookie(&headers).ok_or_else(|| AppError::Unauthorized("No session cookie".into()))?;
	let session = SessionManager::validate_session(&state.db, &token).await?;

	let user = Users::find_by_id(session.user_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("User not found".into()))?;

	Ok(Json(user.into()))
}

async fn update_profile(
	State(state): State<SharedState>,
	headers: HeaderMap,
	Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, AppError> {
	let token = extract_session_cookie(&headers).ok_or_else(|| AppError::Unauthorized("No session cookie".into()))?;
	let current = SessionManager::validate_session(&state.db, &token).await?;

	let user = Users::find_by_id(current.user_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("User not found".into()))?;

	let mut active: users::ActiveModel = user.into();
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	active.updated_at = Set(now);
	if let Some(name) = req.display_name {
		if !name.trim().is_empty() {
			active.display_name = Set(name.trim().to_string());
		}
	}
	let updated = active.update(&state.db).await?;
	Ok(Json(updated.into()))
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
	current_password: String,
	new_password: String,
}

async fn change_password(
	State(state): State<SharedState>,
	headers: HeaderMap,
	Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
	if req.new_password.len() < 6 {
		return Err(AppError::BadRequest("Password must be at least 6 characters".into()));
	}

	let token = extract_session_cookie(&headers).ok_or_else(|| AppError::Unauthorized("No session cookie".into()))?;
	let current = SessionManager::validate_session(&state.db, &token).await?;

	let user = Users::find_by_id(current.user_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("User not found".into()))?;

	SessionManager::verify_password(&user.password_hash, &req.current_password)?;

	let new_hash = SessionManager::hash_password(&req.new_password)?;

	let mut active: users::ActiveModel = user.into();
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
	active.password_hash = Set(new_hash);
	active.updated_at = Set(now);
	active.update(&state.db).await?;

	Ok(Json(serde_json::json!({ "message": "password changed" })))
}
