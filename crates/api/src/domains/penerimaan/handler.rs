//! Handler penerimaan.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::penerimaan::repository::PenerimaanRow;
use crate::domains::penerimaan::service::{PenerimaanService, TerimaInput};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(terima))
        .route("/po/:po_id", get(list_by_po))
        .route("/po/:po_id/siap", get(detail_siap_terima))
}

#[derive(Debug, Deserialize)]
struct PagingQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TerimaBody {
    po_detail_id: i64,
    qty_terima: BigDecimal,
    #[serde(default)]
    qty_reject: Option<BigDecimal>,
    kondisi: Option<String>,
    no_do: Option<String>,
    no_invoice: Option<String>,
    no_faktur_pajak: Option<String>,
    catatan: Option<String>,
    tanggal_terima: Option<NaiveDate>,
    tahun: Option<i16>,
}

fn serialize(p: &PenerimaanRow) -> serde_json::Value {
    serde_json::json!({
        "id": p.id, "no_terima": p.no_terima, "no_po": p.no_po,
        "po_detail_id": p.po_detail_id, "item": p.item_nama,
        "tanggal_terima": p.tanggal_terima,
        "qty_terima": p.qty_terima, "qty_reject": p.qty_reject, "qty_po": p.qty_po,
        "kondisi": p.kondisi, "no_do": p.no_do, "no_invoice": p.no_invoice,
        "no_faktur_pajak": p.no_faktur_pajak,
        "diterima_oleh": p.diterima_oleh, "catatan": p.catatan, "updated_at": p.updated_at,
    })
}

async fn list(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<PagingQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::PetugasGudang,
            Role::KasiSarpras,
            Role::Auditor,
            Role::KabagKeuangan,
        ],
    )?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = PenerimaanService::list(&state, page, per_page).await?;
    Ok(ApiResponse::ok_paged(
        rows.iter().map(serialize).collect(),
        page,
        per_page,
        total,
    ))
}

async fn list_by_po(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(po_id): Path<i64>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::PetugasGudang,
            Role::KasiSarpras,
            Role::Auditor,
            Role::KabagKeuangan,
        ],
    )?;
    let rows = PenerimaanService::list_by_po(&state, po_id).await?;
    Ok(ApiResponse::ok(rows.iter().map(serialize).collect()))
}

async fn detail_siap_terima(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(po_id): Path<i64>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[Role::Admin, Role::PetugasGudang, Role::KasiSarpras],
    )?;
    Ok(ApiResponse::ok(
        PenerimaanService::detail_siap_terima(&state, po_id).await?,
    ))
}

async fn terima(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<TerimaBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::PetugasGudang])?;
    let tahun = body.tahun.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2026)
    });

    let row = PenerimaanService::terima(
        &state,
        actor.id,
        TerimaInput {
            po_detail_id: body.po_detail_id,
            qty_terima: body.qty_terima,
            qty_reject: body.qty_reject.unwrap_or_else(|| BigDecimal::from(0)),
            kondisi: body.kondisi,
            no_do: body.no_do,
            no_invoice: body.no_invoice,
            no_faktur_pajak: body.no_faktur_pajak,
            catatan: body.catatan,
            tanggal: body.tanggal_terima,
        },
        tahun,
    )
    .await?;

    Ok(ApiResponse::ok(serialize(&row)))
}
