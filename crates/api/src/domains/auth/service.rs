//! Service auth — login/refresh/logout + issue token pair.

use chrono::{Duration, Utc};

use sarpras_shared::auth::jwt::{decode_token, hash_token, issue_access, issue_refresh, TokenType};
use sarpras_shared::auth::password::verify_password;
use sarpras_shared::auth::AuthUser;
use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};
use sarpras_shared::rbac::Role;

use crate::state::AppState;

use super::repository::AuthRepository;

#[derive(Debug, serde::Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(skip)]
    pub refresh_hash: String,
    #[serde(skip)]
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct AuthService;

impl AuthService {
    /// Login: verifikasi password + rate limit (K05) + terbitkan token pair.
    /// `identifier` bisa username ATAU email (lookup case-insensitive).
    pub async fn login(
        state: &AppState,
        identifier: &str,
        password: &str,
        ip: &str,
    ) -> AppResult<(TokenPair, AuthUser)> {
        let key = format!("login:{}:{ip}", identifier.trim().to_lowercase());
        let ctx = ConfigContext::global();

        // Rate limit login (K05 = maks percobaan per 15 menit).
        let max_login = state.config.get_i64("K05", &ctx).await?.max(1) as u32;
        state.rate.check_login(&key, max_login)?;

        let row =
            crate::domains::user::repository::UserRepository::find_login(&state.pool, identifier)
                .await?
                .ok_or_else(|| {
                    state.rate.record_login_failure(&key);
                    AppError::InvalidCredentials
                })?;

        if !row.is_active {
            return Err(AppError::Unauthorized);
        }

        let ok = verify_password(password, &row.password_hash)?;
        if !ok {
            state.rate.record_login_failure(&key);
            return Err(AppError::InvalidCredentials);
        }

        let role =
            Role::from_kode(&row.role).ok_or(AppError::Config("role user tidak valid".into()))?;
        let user = AuthUser {
            id: row.id,
            nama: row.nama,
            nip: row.nip,
            email: row.email,
            role,
            ruangan_id: row.ruangan_id,
        };

        let pair = Self::issue_pair(state, &user).await?;
        tracing::info!(user_id = user.id, ip, "login berhasil");
        Ok((pair, user))
    }

    /// Terbitkan access + refresh (refresh hash tersimpan di DB).
    pub async fn issue_pair(state: &AppState, user: &AuthUser) -> AppResult<TokenPair> {
        let ctx = ConfigContext::global();
        let access_min = state.config.get_i64("K01", &ctx).await?.max(1);
        let refresh_hours = state.config.get_i64("K02", &ctx).await?.max(1);

        let access_token = issue_access(
            user.id,
            &user.nama,
            user.role,
            user.ruangan_id,
            &state.cfg.jwt_secret,
            access_min,
        )?;
        let refresh_token = issue_refresh(
            user.id,
            &user.nama,
            user.role,
            &state.cfg.jwt_secret,
            refresh_hours,
        )?;
        let pair = TokenPair {
            refresh_hash: hash_token(&refresh_token),
            refresh_expires_at: Utc::now() + Duration::hours(refresh_hours),
            access_token,
            refresh_token,
        };
        AuthRepository::issue(&state.pool, user.id, &pair).await?;
        Ok(pair)
    }

    /// Refresh: validasi refresh token + rotasi (cabut lama, terbit baru).
    pub async fn refresh(
        state: &AppState,
        refresh_token: &str,
    ) -> AppResult<(TokenPair, AuthUser)> {
        let claims = decode_token(refresh_token, &state.cfg.jwt_secret)?;
        if claims.token_type != TokenType::Refresh {
            return Err(AppError::InvalidToken);
        }
        let token_hash = hash_token(refresh_token);
        let (db_id, user_id) = AuthRepository::find_valid(&state.pool, &token_hash)
            .await?
            .ok_or(AppError::InvalidToken)?;

        let row = crate::domains::user::repository::UserRepository::find(&state.pool, user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        if !row.is_active {
            return Err(AppError::Unauthorized);
        }

        // Rotasi: cabut token lama.
        AuthRepository::revoke(&state.pool, &token_hash).await?;

        let role =
            Role::from_kode(&row.role).ok_or(AppError::Config("role user tidak valid".into()))?;
        let user = AuthUser {
            id: row.id,
            nama: row.nama.clone(),
            nip: row.nip.clone(),
            email: row.email.clone(),
            role,
            ruangan_id: row.ruangan_id,
        };
        let pair = Self::issue_pair(state, &user).await?;
        tracing::info!(
            refresh_id = db_id,
            user_id = user.id,
            "refresh token dirotasi"
        );
        Ok((pair, user))
    }

    /// Logout: cabut refresh token.
    pub async fn logout(state: &AppState, refresh_token: &str) -> AppResult<()> {
        let token_hash = hash_token(refresh_token);
        AuthRepository::revoke(&state.pool, &token_hash).await?;
        Ok(())
    }
}
