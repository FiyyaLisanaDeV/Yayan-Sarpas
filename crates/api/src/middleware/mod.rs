//! Middleware — extractor auth (JWT → AuthUser) + helper audit trail.

pub mod auth;

use serde_json::Value;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

/// Tulis audit trail untuk aksi mutasi. Dipanggil oleh semua service layer.
pub async fn record_audit(
    pool: &PgPool,
    user_id: i64,
    tabel: &str,
    record_id: i64,
    aksi: &str,
    before: Option<&Value>,
    after: Option<&Value>,
) -> AppResult<()> {
    sqlx::query!(
        "INSERT INTO audit_trail (user_id, tabel, record_id, aksi, before_json, after_json)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        tabel,
        record_id,
        aksi,
        before,
        after
    )
    .execute(pool)
    .await?;
    Ok(())
}
