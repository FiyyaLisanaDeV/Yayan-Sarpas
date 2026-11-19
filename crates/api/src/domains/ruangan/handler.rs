//! Handler ruangan — endpoint HTTP.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use validator::Validate;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::ruangan::repository::RuanganRepository;
use crate::domains::ruangan::service::RuanganService;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_one).put(update).delete(soft_delete))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RuanganBody {
    #[validate(length(min = 1, max = 50, message = "kode wajib 1-50 karakter"))]
    pub kode: String,
    #[validate(length(min = 2, max = 200, message = "nama wajib 2-200 karakter"))]
    pub nama: String,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = RuanganRepository::list(
        &state.pool,
        page,
        per_page,
        q.q.as_deref(),
        q.include_inactive.unwrap_or(false),
    )
    .await?;
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::json!({"id": r.id, "kode": r.kode, "nama": r.nama, "is_active": r.is_active}))
        .collect();
    let _ = actor;
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

async fn get_one(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let row = RuanganRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Ruangan".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": row.id, "kode": row.kode, "nama": row.nama, "is_active": row.is_active
    })))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<RuanganBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let row = RuanganService::create(&state, actor.id, &body.kode, &body.nama).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": row.id, "kode": row.kode, "nama": row.nama, "is_active": row.is_active
    })))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<RuanganBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let row = RuanganService::update(&state, actor.id, id, &body.kode, &body.nama).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": row.id, "kode": row.kode, "nama": row.nama, "is_active": row.is_active
    })))
}

async fn soft_delete(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    RuanganService::soft_delete(&state, actor.id, id).await?;
    Ok(ApiResponse::ok(()))
}
