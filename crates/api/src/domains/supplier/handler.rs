//! Handler supplier — master + katalog.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::supplier::repository::SupplierRepository;
use crate::domains::supplier::service::SupplierService;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_one).put(update).delete(soft_delete))
        .route("/:id/items", get(list_items).post(add_item))
        .route("/:id/items/:katalog_id", axum::routing::delete(remove_item))
        .route("/:id/supplier-item/:item_id", get(supplier_untuk_item))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SupplierBody {
    nama: String,
    npwp: Option<String>,
    kontak: Option<String>,
    alamat: Option<String>,
    #[serde(default = "default_true")]
    is_active: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct KatalogBody {
    item_id: i64,
    harga_referensi: Option<BigDecimal>,
    catatan: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = SupplierRepository::list(
        &state.pool,
        page,
        per_page,
        q.q.as_deref(),
        q.include_inactive.unwrap_or(false),
    )
    .await?;

    let items = rows
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id, "nama": s.nama, "npwp": s.npwp, "kontak": s.kontak,
                "alamat": s.alamat, "is_active": s.is_active,
                "jumlah_item": s.jumlah_item.unwrap_or(0),
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
    let s = SupplierRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Supplier".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": s.id, "nama": s.nama, "npwp": s.npwp, "kontak": s.kontak,
        "alamat": s.alamat, "is_active": s.is_active,
        "jumlah_item": s.jumlah_item.unwrap_or(0),
    })))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<SupplierBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let s = SupplierService::create(
        &state,
        actor.id,
        &body.nama,
        body.npwp,
        body.kontak,
        body.alamat,
    )
    .await?;
    Ok(ApiResponse::ok(
        serde_json::json!({"id": s.id, "nama": s.nama}),
    ))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<SupplierBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let s = SupplierService::update(
        &state,
        actor.id,
        id,
        crate::domains::supplier::service::UpdateSupplierInput {
            nama: body.nama,
            npwp: body.npwp,
            kontak: body.kontak,
            alamat: body.alamat,
            is_active: body.is_active,
        },
    )
    .await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": s.id, "nama": s.nama, "is_active": s.is_active
    })))
}

async fn soft_delete(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    SupplierService::soft_delete(&state, actor.id, id).await?;
    Ok(ApiResponse::ok(()))
}

async fn list_items(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let rows = SupplierService::list_items(&state, id).await?;
    let items = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "item_id": r.item_id, "item_nama": r.item_nama,
                "kategori": r.kategori, "satuan": r.satuan_kode,
                "harga_referensi": r.harga_referensi,
                "catatan": r.catatan, "is_active": r.is_active,
            })
        })
        .collect();
    Ok(ApiResponse::ok(items))
}

async fn add_item(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<KatalogBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let katalog_id = SupplierService::add_item(
        &state,
        actor.id,
        id,
        body.item_id,
        body.harga_referensi,
        body.catatan,
    )
    .await?;
    Ok(ApiResponse::ok(serde_json::json!({"id": katalog_id})))
}

async fn remove_item(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path((_id, katalog_id)): Path<(i64, i64)>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    SupplierService::remove_item(&state, actor.id, katalog_id).await?;
    Ok(ApiResponse::ok(()))
}

/// Supplier yang menyediakan item tertentu (dipakai form penawaran).
async fn supplier_untuk_item(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((_id, item_id)): Path<(i64, i64)>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let rows = SupplierRepository::supplier_untuk_item(&state.pool, item_id).await?;
    let items = rows
        .into_iter()
        .map(|(id, nama)| serde_json::json!({"id": id, "nama": nama}))
        .collect();
    Ok(ApiResponse::ok(items))
}

// `post` dipakai untuk add_item; impor agar tidak dianggap unused.
#[allow(dead_code)]
fn _use_post() -> axum::routing::MethodRouter<AppState> {
    post(|| async { "" })
}
