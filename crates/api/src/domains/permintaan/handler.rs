//! Handler permintaan — Form Permintaan & My Requests.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use bigdecimal::BigDecimal;
use serde::Deserialize;
use uuid::Uuid;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::permintaan::repository::{PermintaanRepository, PermintaanRow};
use crate::domains::permintaan::service::{ItemInput, PermintaanService};
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_one).put(update_row))
        .route("/batch/:batch_id", get(get_batch).delete(delete_batch))
        .route("/batch/:batch_id/submit", post(submit_batch))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    tahun: Option<i16>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ItemBody {
    item_id: i64,
    jumlah: BigDecimal,
    satuan_id: i64,
    keterangan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateBody {
    ruangan_id: Option<i64>,
    items: Vec<ItemBody>,
    #[serde(default)]
    lampiran_ada: bool,
}

#[derive(Debug, Deserialize)]
struct UpdateBody {
    jumlah: BigDecimal,
    satuan_id: i64,
    keterangan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubmitBody {
    #[serde(default)]
    urgent: bool,
}

fn serialize(r: &PermintaanRow) -> serde_json::Value {
    serde_json::json!({
        "id": r.id,
        "batch_id": r.batch_id,
        "tahun": r.tahun,
        "ruangan": {"id": r.ruangan_id, "nama": r.ruangan_nama},
        "item": {"id": r.item_id, "nama": r.item_nama, "kategori": r.kategori_kode},
        "jumlah": r.jumlah,
        "satuan": {"id": r.satuan_id, "kode": r.satuan_kode},
        "keterangan": r.keterangan,
        "status": r.status,
        "current_step": r.current_step,
        "revisi_ke": r.revisi_ke,
        "sla_deadline": r.sla_deadline,
        "sla_status": r.sla_status,
        "created_by": r.created_by,
        "submitted_at": r.submitted_at,
        "closed_at": r.closed_at,
        "updated_at": r.updated_at,
    })
}

/// Ruangan efektif: USER_RUANGAN hanya boleh ruangannya sendiri.
fn resolve_ruangan(
    actor: &sarpras_shared::auth::AuthUser,
    diminta: Option<i64>,
) -> Result<i64, AppError> {
    match actor.role {
        Role::UserRuangan => actor.ruangan_id.ok_or_else(|| {
            AppError::Validation(
                "Akun Anda belum terhubung ke ruangan — hubungi Administrator".into(),
            )
        }),
        _ => diminta.ok_or_else(|| AppError::Validation("ruangan_id wajib diisi".into())),
    }
}

async fn list(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);

    // USER_RUANGAN dibatasi ke ruangannya sendiri.
    let ruangan_filter = if actor.role == Role::UserRuangan {
        actor.ruangan_id
    } else {
        None
    };

    let (rows, total) = PermintaanRepository::list(
        &state.pool,
        page,
        per_page,
        q.tahun,
        q.status.as_deref(),
        ruangan_filter,
        q.q.as_deref(),
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
    let row = PermintaanRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Permintaan".into()))?;

    if actor.role == Role::UserRuangan && Some(row.ruangan_id) != actor.ruangan_id {
        return Err(AppError::Forbidden);
    }
    Ok(ApiResponse::ok(serialize(&row)))
}

async fn get_batch(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    let rows = PermintaanRepository::by_batch(&state.pool, batch_id).await?;
    if rows.is_empty() {
        return Err(AppError::NotFound("Permintaan".into()));
    }
    if actor.role == Role::UserRuangan && Some(rows[0].ruangan_id) != actor.ruangan_id {
        return Err(AppError::Forbidden);
    }
    Ok(ApiResponse::ok(rows.iter().map(serialize).collect()))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<CreateBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[Role::UserRuangan, Role::KasiSarpras, Role::Admin],
    )?;

    let ruangan_id = resolve_ruangan(&actor, body.ruangan_id)?;

    let items: Vec<ItemInput> = body
        .items
        .into_iter()
        .map(|i| ItemInput {
            item_id: i.item_id,
            jumlah: i.jumlah,
            satuan_id: i.satuan_id,
            keterangan: i.keterangan,
        })
        .collect();

    let batch_id =
        PermintaanService::create_draft(&state, actor.id, ruangan_id, &items, body.lampiran_ada)
            .await?;

    let rows = PermintaanRepository::by_batch(&state.pool, batch_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "batch_id": batch_id,
        "jumlah_baris": rows.len(),
        "status": "DRAFT",
        "data": rows.iter().map(serialize).collect::<Vec<_>>(),
    })))
}

async fn update_row(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> Result<ApiResponse<()>, AppError> {
    PermintaanService::update_row(
        &state,
        actor.id,
        id,
        body.jumlah,
        body.satuan_id,
        body.keterangan,
    )
    .await?;
    Ok(ApiResponse::ok(()))
}

async fn submit_batch(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
    Json(body): Json<SubmitBody>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[Role::UserRuangan, Role::KasiSarpras, Role::Admin],
    )?;
    let urgent = body.urgent && matches!(actor.role, Role::KasiSarpras | Role::Admin);

    let rows = PermintaanService::submit(&state, actor.id, batch_id, urgent).await?;
    Ok(ApiResponse::ok(rows.iter().map(serialize).collect()))
}

async fn delete_batch(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let n = PermintaanService::delete_draft(&state, actor.id, batch_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({"dihapus": n})))
}
