//! Handler import — wizard Excel → permintaan historis.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Datelike;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::import::repository::ImportRepository;
use crate::domains::import::service::{ImportReport, ImportService};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/preview", get(preview))
        .route("/excel", post(import_excel))
        .route("/batch", get(list_batch))
        .route("/batch/:id", get(get_batch))
        .route("/batch/:id/rows", get(row_log))
}

#[derive(Debug, Deserialize)]
struct PagingQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ImportBody {
    /// Path file Excel; kosongkan untuk auto-detect `REKAPAN*.xlsx`.
    file_path: Option<String>,
    /// Tahun anggaran baris historis (default tahun berjalan).
    tahun: Option<i16>,
    /// Pratinjau saja tanpa menyimpan.
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct PreviewQuery {
    file_path: Option<String>,
}

async fn preview(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<PreviewQuery>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let data = ImportService::preview(&state, q.file_path).await?;
    Ok(ApiResponse::ok(data))
}

async fn import_excel(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<ImportBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin])?;

    if body.dry_run {
        let data = ImportService::preview(&state, body.file_path).await?;
        return Ok(ApiResponse::ok(serde_json::json!({
            "mode": "DRY_RUN",
            "preview": data,
        })));
    }

    let tahun = body
        .tahun
        .unwrap_or_else(|| chrono::Utc::now().year() as i16);
    let report: ImportReport =
        ImportService::import_excel(&state, actor.id, body.file_path, tahun).await?;

    Ok(ApiResponse::ok(serde_json::json!({
        "mode": "COMMIT",
        "batch_id": report.batch_id,
        "file_path": report.file_path,
        "tahun": tahun,
        "total_row": report.total_row,
        "sukses": report.sukses,
        "gagal": report.gagal,
        "duplikat": report.duplikat,
        "dilewati_parser": report.dilewati_parser,
        "ruangan_tidak_cocok": report.ruangan_tidak_cocok,
        "item_tidak_cocok": report.item_tidak_cocok,
    })))
}

async fn list_batch(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<PagingQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = ImportRepository::list_batch(&state.pool, page, per_page).await?;
    let items = rows
        .iter()
        .map(|b| {
            serde_json::json!({
                "id": b.id, "tipe": b.tipe, "file_path": b.file_path,
                "total_row": b.total_row, "sukses": b.sukses, "gagal": b.gagal,
                "created_by": b.created_by, "created_at": b.created_at,
            })
        })
        .collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

async fn get_batch(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let b = ImportRepository::find_batch(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Import batch".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": b.id, "tipe": b.tipe, "file_path": b.file_path,
        "total_row": b.total_row, "sukses": b.sukses, "gagal": b.gagal,
        "created_by": b.created_by, "created_at": b.created_at,
    })))
}

async fn row_log(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Query(q): Query<PagingQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = ImportRepository::list_row_log(&state.pool, id, page, per_page).await?;
    let items = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "row_no": r.row_no, "status": r.status,
                "error_msg": r.error_msg, "raw_json": r.raw_json,
            })
        })
        .collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}
