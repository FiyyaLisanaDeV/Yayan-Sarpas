//! Handler serah terima.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::serah_terima::repository::SerahRow;
use crate::domains::serah_terima::service::{SerahInput, SerahTerimaService};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(buat))
        .route("/siap", get(siap))
        .route("/:id", get(get_one))
        .route("/:id/konfirmasi", post(konfirmasi))
        .route("/:id/dispute", post(dispute))
        .route("/:id/resolve", post(resolve))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BuatBody {
    penerimaan_id: i64,
    ruangan_id: i64,
    penerima_ruangan_id: i64,
    qty: Option<BigDecimal>,
    tanggal_serah: Option<NaiveDate>,
    catatan: Option<String>,
    tahun: Option<i16>,
}

#[derive(Debug, Deserialize)]
struct CatatanBody {
    catatan: Option<String>,
}

fn serialize(s: &SerahRow) -> serde_json::Value {
    serde_json::json!({
        "id": s.id, "no_ba": s.no_ba, "no_po": s.no_po, "no_terima": s.no_terima,
        "penerimaan_id": s.penerimaan_id, "item": s.item_nama,
        "ruangan": {"id": s.ruangan_id, "nama": s.ruangan_nama},
        "tanggal_serah": s.tanggal_serah, "qty_serah": s.qty_serah,
        "penerima": {"id": s.penerima_ruangan_id, "nama": s.penerima_nama},
        "pemberi": {"id": s.pemberi_id, "nama": s.pemberi_nama},
        "file_ba": s.file_ba, "catatan": s.catatan,
        "status_konfirmasi": s.status_konfirmasi, "updated_at": s.updated_at,
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
            Role::PetugasGudang,
            Role::KasiSarpras,
            Role::UserRuangan,
            Role::Auditor,
        ],
    )?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);

    // USER_RUANGAN hanya melihat serah terima ruangannya.
    let ruangan = if actor.role == Role::UserRuangan {
        actor.ruangan_id
    } else {
        None
    };

    let (rows, total) =
        SerahTerimaService::list(&state, page, per_page, q.status.as_deref(), ruangan).await?;
    Ok(ApiResponse::ok_paged(
        rows.iter().map(serialize).collect(),
        page,
        per_page,
        total,
    ))
}

async fn siap(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[Role::Admin, Role::PetugasGudang, Role::KasiSarpras],
    )?;
    Ok(ApiResponse::ok(
        SerahTerimaService::siap_diserehkan(&state).await?,
    ))
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
            Role::PetugasGudang,
            Role::KasiSarpras,
            Role::UserRuangan,
            Role::Auditor,
        ],
    )?;
    let row = crate::domains::serah_terima::repository::SerahRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Serah terima".into()))?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn buat(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<BuatBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[Role::Admin, Role::PetugasGudang, Role::KasiSarpras],
    )?;
    let tahun = body.tahun.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2026)
    });

    let row = SerahTerimaService::buat(
        &state,
        actor.id,
        SerahInput {
            penerimaan_id: body.penerimaan_id,
            ruangan_id: body.ruangan_id,
            penerima_ruangan_id: body.penerima_ruangan_id,
            qty: body.qty,
            tanggal: body.tanggal_serah,
            catatan: body.catatan,
        },
        tahun,
    )
    .await?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn konfirmasi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let row = SerahTerimaService::konfirmasi(&state, &actor, id).await?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn dispute(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<CatatanBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let row = SerahTerimaService::dispute(&state, &actor, id, body.catatan).await?;
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn resolve(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<CatatanBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::KasiSarpras])?;
    let row = SerahTerimaService::resolve(&state, actor.id, id, body.catatan).await?;
    Ok(ApiResponse::ok(serialize(&row)))
}
