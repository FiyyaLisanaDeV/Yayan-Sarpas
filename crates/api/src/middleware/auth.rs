//! Extractor AuthUser — decode Bearer token, validasi user aktif, rate limit API.

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use sarpras_shared::auth::jwt::{decode_token, TokenType};
use sarpras_shared::auth::AuthUser as User;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::Role;

use crate::state::AppState;

/// Maks request API per user per menit (§9).
pub const API_LIMIT_PER_MINUTE: u32 = 100;

pub struct AuthUser(pub User);

impl std::ops::Deref for AuthUser {
    type Target = User;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = decode_token(token, &state.cfg.jwt_secret)?;
        if claims.token_type != TokenType::Access {
            return Err(AppError::InvalidToken);
        }
        let user_id: i64 = claims.sub.parse().map_err(|_| AppError::InvalidToken)?;

        // Rate limit API per user (fixed window 60 dtk).
        state.rate.check_api(user_id, API_LIMIT_PER_MINUTE)?;

        // Data user diambil dari DB agar role/is_active selalu terkini.
        let row = sqlx::query!(
            "SELECT id, nama, nip, email, role, ruangan_id, is_active FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

        if !row.is_active {
            return Err(AppError::Unauthorized);
        }
        let role = Role::from_kode(&row.role).ok_or(AppError::InvalidToken)?;

        Ok(AuthUser(User {
            id: row.id,
            nama: row.nama,
            nip: row.nip,
            email: row.email,
            role,
            ruangan_id: row.ruangan_id,
        }))
    }
}
