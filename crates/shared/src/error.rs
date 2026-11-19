//! AppError — error terstruktur dengan kode + HTTP status + envelope respons.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::api::{ApiResponse, ErrorBody};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Silakan login terlebih dahulu")]
    Unauthorized,
    #[error("Anda tidak memiliki akses untuk aksi ini")]
    Forbidden,
    #[error("Email atau password salah")]
    InvalidCredentials,
    #[error("Token tidak valid atau sudah kedaluwarsa")]
    InvalidToken,
    #[error("Terlalu banyak percobaan, coba lagi beberapa saat lagi")]
    RateLimited,
    #[error("{0}")]
    Validation(String),
    #[error("{0} tidak ditemukan")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Config(String),
    #[error("Password tidak memenuhi kebijakan keamanan")]
    WeakPassword,
    #[error("Terjadi kesalahan internal server")]
    Internal,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "AUTH_001",
            Self::Forbidden => "AUTH_002",
            Self::InvalidCredentials => "AUTH_003",
            Self::InvalidToken => "AUTH_004",
            Self::RateLimited => "AUTH_005",
            Self::WeakPassword => "AUTH_006",
            Self::Validation(_) => "VAL_001",
            Self::NotFound(_) => "NF_001",
            Self::Conflict(_) => "CF_001",
            Self::Config(_) => "CFG_001",
            Self::Database(_) => "DB_001",
            Self::Internal => "SRV_001",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized | Self::InvalidCredentials | Self::InvalidToken => {
                StatusCode::UNAUTHORIZED
            }
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Validation(_) | Self::WeakPassword | Self::Config(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) | Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Pesan yang aman dikirim ke client (tanpa detail internal).
    fn public_message(&self) -> String {
        match self {
            Self::Database(e) => {
                tracing::error!(error = %e, "kesalahan database");
                "Terjadi kesalahan pada server. Silakan coba lagi.".to_string()
            }
            other => other.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ApiResponse {
            success: false,
            data: (),
            error: Some(ErrorBody {
                code: self.code().to_string(),
                message: self.public_message(),
                details: None,
            }),
            meta: None,
        };
        (self.status(), Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
