//! Handler config.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::config::simulate;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::config::service::{resolve_effective, ConfigService};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/groups", get(groups))
        .route("/items", get(items))
        .route("/values/:kode", get(effective_value).put(update_value))
        .route("/change-requests", get(change_requests))
        .route(
            "/change-requests/:id/approve",
            get(approve_request).post(approve_request),
        )
        .route(
            "/change-requests/:id/reject",
            get(reject_request).post(reject_request),
        )
        .route("/audit", get(audit))
        .route("/simulate", get(simulate_impact).post(simulate_impact))
}

#[derive(Debug, Deserialize)]
struct ItemsQuery {
    q: Option<String>,
    group: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ValueQuery {
    ruangan_id: Option<i64>,
    kategori_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UpdateValueBody {
    pub nilai: String,
    pub alasan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    kode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RequestsQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SimulateBody {
    pub kode: String,
    pub nilai_baru: String,
}

async fn groups(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    Ok(ApiResponse::ok(
        ConfigService::list_groups(&state.pool).await?,
    ))
}

async fn items(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<ItemsQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    // Satu query berisi nilai efektif — UI Config Center tidak perlu
    // memanggil /config/values per kode (menghindari N+1 & rate limit).
    let items =
        ConfigService::list_items_with_values(&state.pool, q.group.as_deref(), q.q.as_deref())
            .await?;
    let total = items.len() as i64;
    Ok(ApiResponse::ok_paged(items, 1, total.max(1), total))
}

async fn effective_value(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(kode): Path<String>,
    Query(q): Query<ValueQuery>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let v = resolve_effective(&state.config, &kode, q.ruangan_id, q.kategori_id).await?;
    Ok(ApiResponse::ok(v))
}

async fn update_value(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(kode): Path<String>,
    Json(body): Json<UpdateValueBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    let hasil =
        ConfigService::update_value(&state, actor.id, &kode, &body.nilai, body.alasan.as_deref())
            .await?;
    Ok(ApiResponse::ok(hasil))
}

async fn change_requests(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<RequestsQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::WadirYanmed,
            Role::KabidYanmed,
            Role::KabagKeuangan,
        ],
    )?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) =
        ConfigService::list_change_requests(&state.pool, page, per_page, q.status.as_deref())
            .await?;
    let items = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "kode": r.kode, "label": r.label, "tipe": r.tipe,
                "nilai_baru": r.nilai_baru, "alasan": r.alasan, "status": r.status,
                "requested_by": r.requested_by, "requested_nama": r.requested_nama,
                "approved_by": r.approved_by, "approved_nama": r.approved_nama,
                "approved_at": r.approved_at, "created_at": r.created_at,
            })
        })
        .collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

async fn approve_request(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    // Otorisasi detail di service (role harus approver untuk kode tsb).
    let hasil = ConfigService::decide_request(&state, &actor, id, true).await?;
    Ok(ApiResponse::ok(hasil))
}

async fn reject_request(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let hasil = ConfigService::decide_request(&state, &actor, id, false).await?;
    Ok(ApiResponse::ok(hasil))
}

async fn audit(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (items, total) =
        ConfigService::list_audit(&state.pool, page, per_page, q.kode.as_deref()).await?;
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

async fn simulate_impact(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(body): Json<SimulateBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let hasil = simulate::simulate(&state.pool, &body.kode, &body.nilai_baru).await?;
    Ok(ApiResponse::ok(serde_json::json!(hasil)))
}
