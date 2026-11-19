//! Handler auth.

use axum::extract::{ConnectInfo, State};
use axum::Json;
use serde::Deserialize;
use std::net::SocketAddr;
use validator::Validate;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::auth::password::hash_password;
use sarpras_shared::auth::AuthUser as User;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::Role;

use crate::domains::auth::service::AuthService;
use crate::domains::user::repository::UserRepository;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterBody {
    #[validate(length(min = 2, max = 200, message = "nama wajib 2-200 karakter"))]
    pub nama: String,
    #[validate(length(min = 3, max = 50))]
    pub nip: String,
    #[validate(email(message = "format email tidak valid"))]
    pub email: String,
    #[validate(length(min = 8, message = "password minimal 8 karakter"))]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    /// Identitas masuk: username (utama) atau email (untuk kompatibilitas).
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshBody {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutBody {
    pub refresh_token: String,
}

/// Bootstrap: hanya berfungsi saat tabel users masih kosong.
/// Setelah itu, pembuatan user dilakukan ADMIN via /users.
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if UserRepository::count_users(&state.pool).await? > 0 {
        return Err(AppError::Conflict(
            "Registrasi bootstrap hanya tersedia saat belum ada user. Gunakan menu Master User."
                .into(),
        ));
    }

    let hash = hash_password(&body.password)?;
    let email = body.email.trim().to_lowercase();
    // Username default: bagian sebelum "@" pada email.
    let username = email.split('@').next().unwrap_or("admin").to_lowercase();
    let id = crate::db_util::map_pg_error(
        UserRepository::create(
            &state.pool,
            body.nama.trim(),
            body.nip.trim(),
            &email,
            &username,
            &hash,
            Role::Admin.as_str(),
            None,
        )
        .await,
        "User (nip/email/username)",
    )?;

    Ok(ApiResponse::ok(serde_json::json!({
        "id": id,
        "username": username,
        "email": email,
        "pesan": "Registrasi berhasil. Silakan login."
    })))
}

pub async fn login(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<LoginBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    // IP dipakai sebagai bagian key rate limit; jika tidak tersedia
    // (mis. reverse proxy tanpa X-Forwarded-For atau test harness), pakai "-".
    let ip = connect_info
        .map(|ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "-".to_string());
    let identifier = body
        .username
        .as_deref()
        .or(body.email.as_deref())
        .ok_or_else(|| AppError::Validation("username atau email wajib diisi".into()))?;
    let (pair, user) = AuthService::login(&state, identifier, &body.password, &ip).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "access_token": pair.access_token,
        "refresh_token": pair.refresh_token,
        "user": user,
    })))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let (pair, user) = AuthService::refresh(&state, &body.refresh_token).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "access_token": pair.access_token,
        "refresh_token": pair.refresh_token,
        "user": user,
    })))
}

pub async fn me(AuthUser(user): AuthUser) -> Result<ApiResponse<User>, AppError> {
    Ok(ApiResponse::ok(user))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutBody>,
) -> Result<ApiResponse<()>, AppError> {
    AuthService::logout(&state, &body.refresh_token).await?;
    Ok(ApiResponse::ok(()))
}
