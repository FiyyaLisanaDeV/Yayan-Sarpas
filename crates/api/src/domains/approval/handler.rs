//! Handler approval — Inbox, setujui, tolak, riwayat.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::approval::repository::InboxRow;
use crate::domains::approval::service::ApprovalService;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/inbox", get(inbox))
        .route("/pending-count", get(pending_count))
        .route("/rantai", get(rantai))
        .route("/:batch_id/riwayat", get(riwayat))
        .route("/:batch_id/setujui", post(setujui))
        .route("/:batch_id/tolak", post(tolak))
        .route(
            "/:batch_id/disposisi",
            get(list_disposisi).post(buat_disposisi),
        )
}

#[derive(Debug, Deserialize)]
struct DisposisiBody {
    isi: String,
    untuk_user_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InboxQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeputusanBody {
    catatan: Option<String>,
}

fn serialize(r: &InboxRow) -> serde_json::Value {
    serde_json::json!({
        "id": r.id,
        "batch_id": r.batch_id,
        "tahun": r.tahun,
        "ruangan": r.ruangan_nama,
        "item": r.item_nama,
        "kategori": r.kategori_kode,
        "jumlah": r.jumlah,
        "satuan": r.satuan_kode,
        "keterangan": r.keterangan,
        "status": r.status,
        "current_step": r.current_step,
        "sla_deadline": r.sla_deadline,
        "pemohon": r.pemohon_nama,
        "submitted_at": r.submitted_at,
        "updated_at": r.updated_at,
    })
}

/// Role yang boleh memakai modul approval.
const ROLE_APPROVER: &[Role] = &[
    Role::KasiSarpras,
    Role::KabidYanmed,
    Role::WadirYanmed,
    Role::Admin,
    Role::Auditor,
];

async fn inbox(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<InboxQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, ROLE_APPROVER)?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) =
        ApprovalService::inbox(&state, actor.role, q.q.as_deref(), page, per_page).await?;

    Ok(ApiResponse::ok_paged(
        rows.iter().map(serialize).collect(),
        page,
        per_page,
        total,
    ))
}

async fn pending_count(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, ROLE_APPROVER)?;
    let n = ApprovalService::jumlah_menunggu(&state, actor.role).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "pending": n })))
}

async fn rantai(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, ROLE_APPROVER)?;
    let steps = ApprovalService::rantai(&state).await?;
    let data = steps
        .iter()
        .map(|s| {
            serde_json::json!({
                "step": s.nomor, "label": s.label, "role": s.role.as_str(),
            })
        })
        .collect();
    Ok(ApiResponse::ok(data))
}

async fn riwayat(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::KasiSarpras,
            Role::KabidYanmed,
            Role::WadirYanmed,
            Role::Admin,
            Role::Auditor,
            Role::UserRuangan,
        ],
    )?;

    // Riwayat diambil per baris pertama batch.
    let ids =
        crate::domains::approval::repository::ApprovalRepository::batch_ids(&state.pool, batch_id)
            .await?;
    if ids.is_empty() {
        return Err(AppError::NotFound("Permintaan".into()));
    }

    let mut keluar = Vec::new();
    for id in ids {
        let logs = ApprovalService::riwayat(&state, id).await?;
        for l in logs {
            keluar.push(serde_json::json!({
                "id": l.id,
                "permintaan_id": l.permintaan_id,
                "step": l.step,
                "actor": l.actor_nama,
                "actor_role": l.actor_role,
                "aksi": l.aksi,
                "catatan": l.catatan,
                "timestamp": l.timestamp,
            }));
        }
    }
    Ok(ApiResponse::ok(keluar))
}

async fn setujui(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
    Json(body): Json<KeputusanBody>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::KasiSarpras,
            Role::KabidYanmed,
            Role::WadirYanmed,
            Role::Admin,
        ],
    )?;
    let rows = ApprovalService::setujui(&state, &actor, batch_id, body.catatan.as_deref()).await?;
    Ok(ApiResponse::ok(rows.iter().map(serialize).collect()))
}

async fn tolak(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
    Json(body): Json<KeputusanBody>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::KasiSarpras,
            Role::KabidYanmed,
            Role::WadirYanmed,
            Role::Admin,
        ],
    )?;
    let rows = ApprovalService::tolak(&state, &actor, batch_id, body.catatan.as_deref()).await?;
    Ok(ApiResponse::ok(rows.iter().map(serialize).collect()))
}

// ---------------------------------------------------------------- disposisi

/// Buat disposisi pimpinan pada satu pengajuan.
async fn buat_disposisi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
    Json(body): Json<DisposisiBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::WadirYanmed, Role::Admin])?;
    let row = crate::domains::approval::disposisi::DisposisiService::buat(
        &state,
        &actor,
        batch_id,
        &body.isi,
        body.untuk_user_id,
    )
    .await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": row.id,
        "batch_id": row.batch_id,
        "dari": {"id": row.dari_user_id, "nama": row.dari_nama, "role": row.dari_role},
        "untuk": row.untuk_user_id.map(|id| serde_json::json!({"id": id, "nama": row.untuk_nama})),
        "status_asal": row.status_asal,
        "isi": row.isi,
        "created_at": row.created_at,
    })))
}

/// Daftar disposisi pada satu pengajuan.
async fn list_disposisi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(batch_id): Path<Uuid>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::KasiSarpras,
            Role::KabidYanmed,
            Role::WadirYanmed,
            Role::Admin,
            Role::Auditor,
            Role::UserRuangan,
        ],
    )?;
    // USER_RUANGAN hanya melihat disposisi pada pengajuannya sendiri.
    let hanya_milik = if actor.role == Role::UserRuangan {
        Some(actor.id)
    } else {
        None
    };
    let rows =
        crate::domains::approval::disposisi::DisposisiService::list(&state, batch_id, hanya_milik)
            .await?;
    Ok(ApiResponse::ok(
        rows.iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "dari": {"id": r.dari_user_id, "nama": r.dari_nama, "role": r.dari_role},
                    "untuk": r.untuk_user_id.map(|id| serde_json::json!({"id": id, "nama": r.untuk_nama})),
                    "status_asal": r.status_asal,
                    "isi": r.isi,
                    "created_at": r.created_at,
                })
            })
            .collect(),
    ))
}
