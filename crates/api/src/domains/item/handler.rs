//! Handler item.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use validator::Validate;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::item::repository::ItemRepository;
use crate::domains::item::service::ItemService;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/options", get(options))
        .route("/:id", get(get_one).put(update).delete(soft_delete))
}

/// Daftar ringkas SEMUA item aktif untuk dropdown form.
/// Tanpa paginasi karena form butuh seluruh katalog (246 item pada data RS ini).
async fn options(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT i.id, i.nama, i.satuan_id, s.kode AS satuan_kode, k.kode AS kategori_kode
           FROM items i
           JOIN satuan s ON s.id = i.satuan_id
           JOIN kategori k ON k.id = i.kategori_id
           WHERE i.is_active
           ORDER BY k.kode, i.nama"#
    )
    .fetch_all(&state.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "nama": r.nama,
                "satuan_id": r.satuan_id, "satuan_kode": r.satuan_kode,
                "kategori": r.kategori_kode,
            })
        })
        .collect();

    Ok(ApiResponse::ok(items))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    kategori_id: Option<i64>,
    include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ItemBody {
    #[validate(length(min = 2, max = 300, message = "nama wajib 2-300 karakter"))]
    pub nama: String,
    pub satuan_id: i64,
    pub kategori_id: i64,
    pub threshold_aset: Option<f64>,
}

async fn list(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = ItemRepository::list(
        &state.pool,
        page,
        per_page,
        q.q.as_deref(),
        q.kategori_id,
        q.include_inactive.unwrap_or(false),
    )
    .await?;
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "nama": r.nama,
                "satuan": {"id": r.satuan_id, "kode": r.satuan_kode, "nama": r.satuan_nama},
                "kategori": {"id": r.kategori_id, "kode": r.kategori_kode, "nama": r.kategori_nama},
                "threshold_aset": r.threshold_aset,
                "is_active": r.is_active,
            })
        })
        .collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

async fn get_one(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let r = ItemRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Item".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": r.id, "nama": r.nama,
        "satuan": {"id": r.satuan_id, "kode": r.satuan_kode, "nama": r.satuan_nama},
        "kategori": {"id": r.kategori_id, "kode": r.kategori_kode, "nama": r.kategori_nama},
        "threshold_aset": r.threshold_aset,
        "is_active": r.is_active,
    })))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<ItemBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if body.satuan_id <= 0 || body.kategori_id <= 0 {
        return Err(AppError::Validation(
            "satuan_id dan kategori_id wajib diisi".into(),
        ));
    }
    let r = ItemService::create(
        &state,
        actor.id,
        &body.nama,
        body.satuan_id,
        body.kategori_id,
        body.threshold_aset,
    )
    .await?;
    Ok(ApiResponse::ok(
        serde_json::json!({"id": r.id, "nama": r.nama}),
    ))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<ItemBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if body.satuan_id <= 0 || body.kategori_id <= 0 {
        return Err(AppError::Validation(
            "satuan_id dan kategori_id wajib diisi".into(),
        ));
    }
    let r = ItemService::update(
        &state,
        actor.id,
        id,
        &body.nama,
        body.satuan_id,
        body.kategori_id,
        body.threshold_aset,
    )
    .await?;
    Ok(ApiResponse::ok(
        serde_json::json!({"id": r.id, "nama": r.nama}),
    ))
}

async fn soft_delete(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    ItemService::soft_delete(&state, actor.id, id).await?;
    Ok(ApiResponse::ok(()))
}
