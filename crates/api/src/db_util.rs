//! Util — mapping error Postgres ke AppError yang ramah.

use sarpras_shared::error::{AppError, AppResult};

/// Ubah unique violation → Conflict agar pesan ke user jelas.
/// Menerima AppResult karena repository sudah membungkus sqlx::Error.
pub fn map_pg_error<T>(result: AppResult<T>, what: &str) -> AppResult<T> {
    match result {
        Ok(v) => Ok(v),
        Err(AppError::Database(e)) => {
            if let sqlx::Error::Database(db) = &e {
                if db.is_unique_violation() {
                    return Err(AppError::Conflict(format!("{what} sudah terdaftar")));
                }
            }
            Err(AppError::Database(e))
        }
        Err(other) => Err(other),
    }
}
