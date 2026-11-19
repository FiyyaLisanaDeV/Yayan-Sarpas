//! Handler PO.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::po::repository::{PoDetailRow, PoHeaderRow, PoRepository};
use crate::domains::po::service::{DetailInput, PoService};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/kandidat", get(kandidat))
        .route("/realisasi", get(realisasi))
        .route("/:id", get(get_one))
        .route("/:id/detail", get(details))
        .route("/:id/approve", post(approve))
        .route("/:id/reject", post(reject))
        .route("/:id/kirim", post(kirim))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    tahun: Option<i16>,
    status: Option<String>,
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KandidatQuery {
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RealisasiQuery {
    tahun: Option<i16>,
}

#[derive(Debug, Deserialize)]
struct DetailBody {
    permintaan_id: i64,
    item_id: i64,
    qty: BigDecimal,
    /// Bila kosong, diambil dari penawaran DIPILIH.
    harga_satuan: Option<BigDecimal>,
    satuan_id: i64,
}

#[derive(Debug, Deserialize)]
struct CreateBody {
    supplier_id: i64,
    tanggal_po: Option<NaiveDate>,
    catatan: Option<String>,
    tahun: Option<i16>,
    details: Vec<DetailBody>,
}

#[derive(Debug, Deserialize)]
struct CatatanBody {
    catatan: Option<String>,
}

fn serialize_header(p: &PoHeaderRow) -> serde_json::Value {
    serde_json::json!({
        "id": p.id, "no_po": p.no_po, "tahun": p.tahun,
        "supplier": {"id": p.supplier_id, "nama": p.supplier_nama},
        "tanggal_po": p.tanggal_po,
        "nilai_total": p.nilai_total, "ppn": p.ppn,
        "status": p.status, "created_by": p.created_by,
        "approved_by": p.approved_by, "approved_at": p.approved_at,
        "catatan": p.catatan, "sla_deadline": p.sla_deadline,
        "sla_status": p.sla_status, "updated_at": p.updated_at,
    })
}

fn serialize_detail(d: &PoDetailRow) -> serde_json::Value {
    serde_json::json!({
        "id": d.id, "permintaan_id": d.permintaan_id,
        "item": {"id": d.item_id, "nama": d.item_nama, "kategori": d.kategori},
        "qty": d.qty, "harga_satuan": d.harga_satuan, "subtotal": d.subtotal,
        "satuan": d.satuan_kode, "qty_diterima": d.qty_diterima,
    })
}

async fn list(
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
            Role::WadirYanmed,
            Role::Auditor,
            Role::PetugasGudang,
        ],
    )?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = PoService::list(
        &state,
        page,
        per_page,
        q.tahun,
        q.status.as_deref(),
        q.q.as_deref(),
    )
    .await?;
    Ok(ApiResponse::ok_paged(
        rows.iter().map(serialize_header).collect(),
        page,
        per_page,
        total,
    ))
}

async fn kandidat(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<KandidatQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let rows = PoRepository::permintaan_final(&state.pool, q.q.as_deref()).await?;
    Ok(ApiResponse::ok(rows))
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
            Role::WadirYanmed,
            Role::Auditor,
            Role::PetugasGudang,
        ],
    )?;
    let row = PoRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("PO".into()))?;
    let details = PoService::details(&state, id).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "header": serialize_header(&row),
        "details": details.iter().map(serialize_detail).collect::<Vec<_>>(),
    })))
}

async fn details(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KasiSarpras,
            Role::KabagKeuangan,
            Role::WadirYanmed,
            Role::Auditor,
            Role::PetugasGudang,
        ],
    )?;
    let rows = PoService::details(&state, id).await?;
    Ok(ApiResponse::ok(rows.iter().map(serialize_detail).collect()))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<CreateBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;

    let mut input = Vec::with_capacity(body.details.len());
    for d in body.details {
        let harga = match d.harga_satuan {
            Some(h) => h,
            None => {
                let terpilih =
                    crate::domains::penawaran::repository::PenawaranRepository::terpilih(
                        &state.pool,
                        d.item_id,
                    )
                    .await?;
                match terpilih {
                    Some((_, h)) => h,
                    None => {
                        return Err(AppError::Validation(format!(
                            "Harga item #{} tidak diisi dan belum ada penawaran dipilih",
                            d.item_id
                        )))
                    }
                }
            }
        };
        input.push(DetailInput {
            permintaan_id: d.permintaan_id,
            item_id: d.item_id,
            qty: d.qty,
            harga_satuan: harga,
            satuan_id: d.satuan_id,
        });
    }

    let tanggal = body
        .tanggal_po
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let tahun = body.tahun.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2026)
    });

    let po = PoService::create(
        &state,
        actor.id,
        body.supplier_id,
        tanggal,
        body.catatan,
        input,
        tahun,
    )
    .await?;

    Ok(ApiResponse::ok(serialize_header(&po)))
}

async fn approve(
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
            Role::WadirYanmed,
        ],
    )?;
    let po = PoService::approve(&state, &actor, id).await?;
    Ok(ApiResponse::ok(serialize_header(&po)))
}

async fn reject(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<CatatanBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KasiSarpras,
            Role::KabagKeuangan,
            Role::WadirYanmed,
        ],
    )?;
    let po = PoService::reject(&state, &actor, id, body.catatan.as_deref()).await?;
    Ok(ApiResponse::ok(serialize_header(&po)))
}

async fn kirim(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let po = PoService::kirim(&state, actor.id, id).await?;
    Ok(ApiResponse::ok(serialize_header(&po)))
}

async fn realisasi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<RealisasiQuery>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KabagKeuangan,
            Role::WadirYanmed,
            Role::KasiSarpras,
            Role::Auditor,
        ],
    )?;
    let tahun = q.tahun.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2026)
    });
    Ok(ApiResponse::ok(PoService::realisasi(&state, tahun).await?))
}
