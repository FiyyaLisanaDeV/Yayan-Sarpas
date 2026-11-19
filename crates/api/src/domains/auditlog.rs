//! Audit log — daftar audit_trail (ADMIN & AUDITOR).

use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list))
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    tabel: Option<String>,
    user_id: Option<i64>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let offset = (page - 1) * per_page;
    let pattern = q.q.as_deref().map(|s| format!("%{}%", s.to_lowercase()));

    let rows = sqlx::query!(
        r#"SELECT at.id, at.user_id, u.nama AS user_nama, at.tabel, at.record_id, at.aksi,
                  at.before_json, at.after_json, at.timestamp
           FROM audit_trail at JOIN users u ON u.id = at.user_id
           WHERE ($3::text IS NULL OR at.tabel = $3)
             AND ($4::bigint IS NULL OR at.user_id = $4)
             AND ($5::text IS NULL OR lower(u.nama) LIKE $5 OR lower(at.tabel) LIKE $5 OR lower(at.aksi) LIKE $5)
           ORDER BY at.timestamp DESC
           LIMIT $1 OFFSET $2"#,
        per_page,
        offset,
        q.tabel,
        q.user_id,
        pattern
    )
    .fetch_all(&state.pool)
    .await?;

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM audit_trail at JOIN users u ON u.id = at.user_id
           WHERE ($1::text IS NULL OR at.tabel = $1)
             AND ($2::bigint IS NULL OR at.user_id = $2)
             AND ($3::text IS NULL OR lower(u.nama) LIKE $3 OR lower(at.tabel) LIKE $3 OR lower(at.aksi) LIKE $3)"#,
        q.tabel,
        q.user_id,
        pattern
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(0);

    let items = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "user_id": r.user_id, "user_nama": r.user_nama,
                "tabel": r.tabel, "record_id": r.record_id, "aksi": r.aksi,
                "before": r.before_json, "after": r.after_json, "timestamp": r.timestamp,
            })
        })
        .collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}
