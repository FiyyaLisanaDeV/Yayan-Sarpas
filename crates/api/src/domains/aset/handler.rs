//! Handler aset — registrasi, mutasi, QR label.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use image::{DynamicImage, ImageFormat, Luma};
use qrcode::QrCode;
use serde::Deserialize;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::aset::repository::AsetRow;
use crate::domains::aset::service::{AsetService, RegistrasiInput};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/registrasi", post(registrasi))
        .route("/:id", get(get_one))
        .route("/:id/status", post(ubah_status))
        .route("/:id/mutasi", get(riwayat).post(mutasi))
        .route("/:id/qr", get(qr_label))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    ruangan_id: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistrasiBody {
    item_id: i64,
    ruangan_id: i64,
    no_seri: Option<String>,
    tgl_perolehan: Option<NaiveDate>,
    nilai: BigDecimal,
    umur_ekonomis: Option<i16>,
    kondisi: Option<String>,
    kode_ruangan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StatusBody {
    status: String,
}

#[derive(Debug, Deserialize)]
struct MutasiBody {
    ke_ruangan_id: i64,
    alasan: Option<String>,
    tanggal: Option<NaiveDate>,
}

fn serialize(a: &AsetRow) -> serde_json::Value {
    serde_json::json!({
        "id": a.id,
        "item": {"id": a.item_id, "nama": a.item_nama, "kategori": a.kategori},
        "ruangan": {"id": a.ruangan_id, "nama": a.ruangan_nama},
        "kode_aset": a.kode_aset, "no_seri": a.no_seri,
        "tgl_perolehan": a.tgl_perolehan, "nilai": a.nilai,
        "umur_ekonomis": a.umur_ekonomis, "kondisi": a.kondisi,
        "status": a.status, "qr_path": a.qr_path,
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
            Role::AdminAset,
            Role::Auditor,
            Role::KasiSarpras,
        ],
    )?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = AsetService::list(
        &state,
        page,
        per_page,
        q.q.as_deref(),
        q.ruangan_id,
        q.status.as_deref(),
    )
    .await?;
    Ok(ApiResponse::ok_paged(
        rows.iter().map(serialize).collect(),
        page,
        per_page,
        total,
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
            Role::AdminAset,
            Role::Auditor,
            Role::KasiSarpras,
        ],
    )?;
    let a = AsetService::find(&state, id).await?;
    Ok(ApiResponse::ok(serialize(&a)))
}

async fn registrasi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<RegistrasiBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::AdminAset])?;
    let a = AsetService::registrasi(
        &state,
        actor.id,
        RegistrasiInput {
            item_id: body.item_id,
            ruangan_id: body.ruangan_id,
            no_seri: body.no_seri,
            tgl_perolehan: body.tgl_perolehan,
            nilai: body.nilai,
            umur_ekonomis: body.umur_ekonomis,
            kondisi: body.kondisi,
            kode_ruangan: body.kode_ruangan,
        },
    )
    .await?;
    Ok(ApiResponse::ok(serialize(&a)))
}

async fn ubah_status(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<StatusBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::AdminAset])?;
    let a = AsetService::ubah_status(&state, actor.id, id, &body.status).await?;
    Ok(ApiResponse::ok(serialize(&a)))
}

async fn riwayat(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::AdminAset, Role::Auditor])?;
    let rows = AsetService::riwayat_mutasi(&state, id).await?;
    let items = rows
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id, "kode_aset": m.kode_aset,
                "dari": m.dari_ruangan, "ke": m.ke_ruangan,
                "tanggal": m.tanggal, "alasan": m.alasan, "oleh": m.oleh_nama,
            })
        })
        .collect();
    Ok(ApiResponse::ok(items))
}

async fn mutasi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<MutasiBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::AdminAset])?;
    let m = AsetService::mutasi(
        &state,
        actor.id,
        id,
        body.ke_ruangan_id,
        body.alasan,
        body.tanggal,
    )
    .await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": m.id, "kode_aset": m.kode_aset,
        "dari": m.dari_ruangan, "ke": m.ke_ruangan, "tanggal": m.tanggal,
    })))
}

/// QR label aset: generate PNG QR (payload format G05), simpan, kembalikan
/// payload + path + data barang untuk popup & cetak stiker.
async fn qr_label(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::AdminAset,
            Role::Auditor,
            Role::KasiSarpras,
        ],
    )?;
    let a = AsetService::find(&state, id).await?;

    // Format label dari G05: "kode_aset|nama|ruangan".
    let format = state
        .config
        .get_string("G05", &sarpras_shared::config::ConfigContext::global())
        .await
        .unwrap_or_else(|_| "kode_aset|nama|ruangan".to_string());

    let isi = format
        .replace("kode_aset", &a.kode_aset)
        .replace("nama", &a.item_nama)
        .replace("ruangan", &a.ruangan_nama);

    let code = QrCode::new(isi.as_bytes())
        .map_err(|e| AppError::Config(format!("gagal membuat QR: {e}")))?;
    let img = code
        .render::<Luma<u8>>()
        .quiet_zone(true)
        .min_dimensions(320, 320)
        .build();
    let mut png_bytes: Vec<u8> = Vec::new();
    DynamicImage::ImageLuma8(img)
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| AppError::Config(format!("gagal encode PNG QR: {e}")))?;

    let dir = std::path::Path::new(&state.cfg.upload_dir).join("qr");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| AppError::Config(format!("gagal membuat direktori QR: {e}")))?;
    let safe_kode = a.kode_aset.replace('/', "_");
    let file_name = format!("{safe_kode}.png");
    let file = dir.join(&file_name);
    tokio::fs::write(&file, &png_bytes)
        .await
        .map_err(|e| AppError::Config(format!("gagal menyimpan QR: {e}")))?;

    let rel = format!("qr/{file_name}");
    AsetService::simpan_qr(&state, id, &rel).await?;

    Ok(ApiResponse::ok(serde_json::json!({
        "kode_aset": a.kode_aset,
        "payload": isi,
        "qr_path": rel,
        "item": {"nama": a.item_nama, "kategori": a.kategori},
        "ruangan": {"id": a.ruangan_id, "nama": a.ruangan_nama},
        "nilai": a.nilai,
        "tgl_perolehan": a.tgl_perolehan,
        "no_seri": a.no_seri,
    })))
}
