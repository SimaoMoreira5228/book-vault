use crate::db::entities::prelude::*;
use crate::db::entities::sessions;
use crate::AppError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordHash, PasswordVerifier, SaltString},
    Argon2,
};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

pub struct SessionManager;

impl SessionManager {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_token() -> (String, Vec<u8>) {
        let mut bytes = [0u8; 32];
        use argon2::password_hash::rand_core::RngCore;
        OsRng.fill_bytes(&mut bytes);
        let token = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
        let hash = blake3::hash(&bytes).as_bytes().to_vec();
        (token, hash)
    }

    pub async fn create_session(
        db: &sea_orm::DatabaseConnection,
        user_id: Uuid,
        user_agent: Option<String>,
        ip_address: Option<String>,
        ttl_days: i64,
        _idle_days: i64,
    ) -> Result<(String, sessions::Model), AppError> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let expires_at = now + chrono::Duration::days(ttl_days);

        let (token, token_hash) = Self::generate_token();

        let model = sessions::ActiveModel {
            id: Set(Uuid::now_v7()),
            user_id: Set(user_id),
            token_hash: Set(token_hash),
            user_agent: Set(user_agent),
            ip_address: Set(ip_address),
            created_at: Set(now),
            last_seen_at: Set(now),
            expires_at: Set(expires_at),
            revoked_at: Set(None),
        };

        let session = sessions::Entity::insert(model)
            .exec_with_returning(db)
            .await?;
        Ok((token, session))
    }

    pub async fn validate_session(
        db: &sea_orm::DatabaseConnection,
        token: &str,
    ) -> Result<sessions::Model, AppError> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, token)
            .map_err(|_| AppError::Unauthorized("Invalid session token".into()))?;
        let hash = blake3::hash(&bytes).as_bytes().to_vec();

        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let session = Sessions::find()
            .filter(sessions::Column::TokenHash.eq(hash))
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Session not found".into()))?;

        if session.revoked_at.is_some() {
            return Err(AppError::Unauthorized("Session revoked".into()));
        }

        if session.expires_at < now {
            return Err(AppError::Unauthorized("Session expired".into()));
        }

        Ok(session)
    }

    pub async fn touch_session(
        db: &sea_orm::DatabaseConnection,
        session_id: Uuid,
        idle_days: i64,
        absolute_max_days: i64,
    ) -> Result<(), AppError> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let session = Sessions::find_by_id(session_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

        let absolute_max = session.created_at + chrono::Duration::days(absolute_max_days);
        let idle_extend = now + chrono::Duration::days(idle_days);
        let new_expires = std::cmp::Ord::min(idle_extend, absolute_max);

        let mut active: sessions::ActiveModel = session.into();
        active.last_seen_at = Set(now);
        active.expires_at = Set(new_expires);
        sessions::Entity::update(active).exec(db).await?;
        Ok(())
    }

    pub async fn revoke_session(
        db: &sea_orm::DatabaseConnection,
        session_id: Uuid,
    ) -> Result<(), AppError> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let session = Sessions::find_by_id(session_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

        let mut active: sessions::ActiveModel = session.into();
        active.revoked_at = Set(Some(now));
        sessions::Entity::update(active).exec(db).await?;
        Ok(())
    }

    pub async fn revoke_all_user_sessions(
        db: &sea_orm::DatabaseConnection,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        sessions::Entity::update_many()
            .col_expr(
                sessions::Column::RevokedAt,
                sea_orm::Value::ChronoDateTimeUtc(Some(now.into())).into(),
            )
            .filter(sessions::Column::UserId.eq(user_id))
            .filter(sessions::Column::RevokedAt.is_null())
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn list_user_sessions(
        db: &sea_orm::DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Vec<sessions::Model>, AppError> {
        let sessions = Sessions::find()
            .filter(sessions::Column::UserId.eq(user_id))
            .filter(sessions::Column::RevokedAt.is_null())
            .order_by_desc(sessions::Column::LastSeenAt)
            .all(db)
            .await?;
        Ok(sessions)
    }

    pub async fn cleanup_expired(
        db: &sea_orm::DatabaseConnection,
        retention_days: i64,
    ) -> Result<u64, AppError> {
        let cutoff: chrono::DateTime<chrono::FixedOffset> =
            (chrono::Utc::now() - chrono::Duration::days(retention_days)).into();
        let result = Sessions::delete_many()
            .filter(
                sessions::Column::ExpiresAt
                    .lt(cutoff)
                    .or(sessions::Column::RevokedAt
                        .is_not_null()
                        .and(sessions::Column::RevokedAt.lt(cutoff))),
            )
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }

    pub fn verify_password(hash: &str, password: &str) -> Result<(), AppError> {
        let argon2 = Argon2::default();
        let stored = PasswordHash::new(hash)
            .map_err(|e| AppError::Internal(format!("Invalid password hash: {}", e)))?;
        argon2
            .verify_password(password.as_bytes(), &stored)
            .map_err(|_| AppError::Unauthorized("Invalid password".into()))
    }

    pub fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;
        Ok(hash.to_string())
    }
}
