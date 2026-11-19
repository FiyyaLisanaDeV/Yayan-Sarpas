//! Repository auth — refresh token store.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

use super::service::TokenPair;

pub struct AuthRepository;

impl AuthRepository {
    /// Simpan hash refresh token (rotasi).
    pub async fn save_refresh(
        pool: &PgPool,
        user_id: i64,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            user_id,
            token_hash,
            expires_at
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Cek refresh token: belum kedaluwarsa & belum dicabut.
    pub async fn find_valid(pool: &PgPool, token_hash: &str) -> AppResult<Option<(i64, i64)>> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM refresh_tokens
             WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > now()",
            token_hash
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|r| (r.id, r.user_id)))
    }

    /// Cabut (revoke) refresh token.
    pub async fn revoke(pool: &PgPool, token_hash: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
            token_hash
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn issue(pool: &PgPool, user_id: i64, tokens: &TokenPair) -> AppResult<()> {
        Self::save_refresh(
            pool,
            user_id,
            &tokens.refresh_hash,
            tokens.refresh_expires_at,
        )
        .await
    }
}
