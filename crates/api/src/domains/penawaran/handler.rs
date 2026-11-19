//! Handler penawaran — catat harga, perbandingan, penunjukan supplier.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::penawaran::repository::PenawaranRow;
use crate::domains::penawaran::service::PenawaranService;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_items).post(create))
        .route("/item/:item_id", get(by_item))
        .route("/:id", get(get_one).put(update).delete(delete))
        .route("/:id/pilih", post(pilih))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    #[serde(default)]
    dipilih: bool,
}

#[derive(Debug, Deserialize)]
struct CreateBody {
    supplier_id: i64,
    item_id: i64,
    harga_satuan: BigDecimal,
    #[serde(default)]
    tanggal_penawaran: Option<NaiveDate>,
    sumber: Option<String>,
    catatan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateBody {
    harga_satuan: BigDecimal,
    tanggal_penawaran: NaiveDate,
    sumber: Option<String>,
    catatan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PilihBody {
    alasan: Option<String>,
}

fn serialize(p: &PenawaranRow) -> serde_json::Value {
    serde_json::json!({
        "id": p.id,
        "supplier": {"id": p.supplier_id, "nama": p.supplier_nama},
        "item": {"id": p.item_id, "nama": p.item_nama, "kategori": p.kategori, "satuan": p.satuan_kode},
        "harga_satuan": p.harga_satuan,
        "tanggal_penawaran": p.tanggal_penawaran,
        "sumber": p.sumber,
        "status": p.status,
        "alasan_penunjukan": p.alasan_penunjukan,
        "catatan": p.catatan,
        "created_at": p.created_at,
    })
}

async fn list_items(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KasiSarpras,
            Role::KabagKeuangan,
            Role::Auditor,
        ],
    )?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) =
        PenawaranService::list_items(&state, q.q.as_deref(), q.dipilih, page, per_page).await?;

    let items = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "item_id": r.item_id, "item_nama": r.item_nama,
                "kategori": r.kategori, "satuan": r.satuan_kode,
                "jumlah_penawaran": r.jumlah_penawaran,
                "harga_terendah": r.harga_terendah,
                "ada_terpilih": r.ada_terpilih,
            })
        })
        .collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

async fn by_item(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(item_id): Path<i64>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KasiSarpras,
            Role::KabagKeuangan,
            Role::Auditor,
        ],
    )?;
    let rows = PenawaranService::by_item(&state, item_id).await?;
    Ok(ApiResponse::ok(rows.iter().map(serialize).collect()))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KasiSarpras,
            Role::KabagKeuangan,
            Role::Auditor,
        ],
    )?;
    let row = crate::domains::penawaran::repository::PenawaranRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Penawaran".into()))?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<CreateBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let tanggal = body
        .tanggal_penawaran
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let row = PenawaranService::create(
        &state,
        actor.id,
        crate::domains::penawaran::service::PenawaranInput {
            supplier_id: body.supplier_id,
            item_id: body.item_id,
            harga_satuan: body.harga_satuan,
            tanggal,
            sumber: body.sumber,
            catatan: body.catatan,
        },
    )
    .await?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let row = PenawaranService::update(
        &state,
        actor.id,
        id,
        body.harga_satuan,
        body.tanggal_penawaran,
        body.sumber,
        body.catatan,
    )
    .await?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn pilih(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<PilihBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let row = PenawaranService::pilih(&state, actor.id, id, body.alasan).await?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn delete(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    PenawaranService::delete(&state, actor.id, id).await?;
    Ok(ApiResponse::ok(()))
}
