//! Service approval — rantai berjenjang dari config B01/B02, SLA, notifikasi.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, Utc};
use serde_json::Value as JsonValue;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};
use sarpras_shared::rbac::Role;

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{ApprovalRepository, InboxRow, LogRow};

/// Satu langkah dalam rantai approval.
#[derive(Debug, Clone)]
pub struct Step {
    pub nomor: i16,
    pub label: String,
    pub role: Role,
}

pub struct ApprovalService;

/// Status permintaan yang menunggu aksi step ke-`step`.
pub fn status_untuk_step(step: i16) -> &'static str {
    match step {
        1 => "SUBMITTED",
        2 => "VERIFIKASI_KASI",
        3 => "APPROVED_KABID",
        4 => "APPROVED_WADIR",
        _ => "APPROVED_WADIR",
    }
}

/// Status ketika step ke-`step` disetujui.
pub fn status_setelah_setuju(step: i16, total_step: i16) -> &'static str {
    if step >= total_step {
        return "FINAL";
    }
    match step {
        1 => "VERIFIKASI_KASI",
        2 => "APPROVED_KABID",
        _ => "APPROVED_WADIR",
    }
}

/// Label status untuk key C02 (SLA per step).
fn sla_key(status: &str) -> &str {
    match status {
        "SUBMITTED" => "SUBMITTED",
        "VERIFIKASI_KASI" => "VERIFIKASI_KASI",
        "APPROVED_KABID" => "APPROVED_KABID",
        "APPROVED_WADIR" => "APPROVED_WADIR",
        "FINAL" => "PO_APPROVAL",
        other => other,
    }
}

impl ApprovalService {
    /// Bangun rantai step dari B01 (urutan) + B02 (role per step).
    pub async fn rantai(state: &AppState) -> AppResult<Vec<Step>> {
        let ctx = ConfigContext::global();
        let urutan: JsonValue = state.config.get_json("B01", &ctx).await?;
        let role_map: JsonValue = state.config.get_json("B02", &ctx).await?;

        let labels: Vec<String> = urutan
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if labels.is_empty() {
            return Err(AppError::Config("Config B01 (urutan_step) kosong".into()));
        }

        let mut steps = Vec::with_capacity(labels.len());
        for (i, label) in labels.iter().enumerate() {
            let nomor = (i + 1) as i16;
            let role_str = role_map
                .get(nomor.to_string())
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::Config(format!("Config B02 tidak punya role untuk step {nomor}"))
                })?;
            let role = Role::from_kode(role_str).ok_or_else(|| {
                AppError::Config(format!("Role '{role_str}' pada B02 tidak dikenal"))
            })?;
            steps.push(Step {
                nomor,
                label: label.clone(),
                role,
            });
        }
        Ok(steps)
    }

    /// Daftar status yang relevan untuk inbox role ini.
    ///
    /// - Normalnya: hanya step yang role-nya cocok dengan role actor.
    /// - Bila B03 (boleh_skip_level) aktif: role mana pun dalam rantai boleh
    ///   mengambil alih step mana pun, jadi semua antrean terlihat.
    /// - ADMIN selalu melihat seluruh antrean (superuser).
    async fn status_inbox(state: &AppState, role: Role) -> AppResult<Vec<String>> {
        let steps = Self::rantai(state).await?;
        let skip = state
            .config
            .get_bool("B03", &ConfigContext::global())
            .await
            .unwrap_or(false);

        let semua: Vec<String> = steps
            .iter()
            .map(|s| {
                if s.nomor == 1 {
                    "SUBMITTED".to_string()
                } else {
                    status_untuk_step(s.nomor).to_string()
                }
            })
            .collect();

        let statuses = if role == Role::Admin || skip {
            semua
        } else {
            let milik_role: Vec<String> = steps
                .iter()
                .filter(|s| s.role == role)
                .map(|s| {
                    if s.nomor == 1 {
                        "SUBMITTED".to_string()
                    } else {
                        status_untuk_step(s.nomor).to_string()
                    }
                })
                .collect();
            milik_role
        };

        if statuses.is_empty() {
            // Role tidak terlibat dalam rantai approval: antrean kosong.
            return Ok(vec!["__tidak_ada__".to_string()]);
        }
        Ok(statuses)
    }

    pub async fn inbox(
        state: &AppState,
        role: Role,
        search: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<InboxRow>, i64)> {
        let statuses = Self::status_inbox(state, role).await?;
        ApprovalRepository::inbox(
            &state.pool,
            role.as_str(),
            &statuses,
            search,
            page,
            per_page,
        )
        .await
    }

    pub async fn jumlah_menunggu(state: &AppState, role: Role) -> AppResult<i64> {
        let statuses = Self::status_inbox(state, role).await?;
        ApprovalRepository::count_pending(&state.pool, &statuses).await
    }

    pub async fn riwayat(state: &AppState, permintaan_id: i64) -> AppResult<Vec<LogRow>> {
        ApprovalRepository::logs(&state.pool, permintaan_id).await
    }

    /// Deadline SLA untuk status tujuan, dari C02 + basis hari kerja C03.
    async fn deadline_untuk(
        state: &AppState,
        status: &str,
        dari: DateTime<Utc>,
    ) -> AppResult<DateTime<Utc>> {
        let ctx = ConfigContext::global();
        let per_step: JsonValue = state
            .config
            .get_json("C02", &ctx)
            .await
            .unwrap_or(serde_json::json!({}));
        let default_hari = state.config.get_i64("C01", &ctx).await.unwrap_or(5);
        let hari = per_step
            .get(sla_key(status))
            .and_then(|v| v.as_i64())
            .unwrap_or(default_hari);

        let basis = state
            .config
            .get_string("C03", &ctx)
            .await
            .unwrap_or_else(|_| "KERJA".into());
        if basis.eq_ignore_ascii_case("KERJA") {
            hitung_hari_kerja(&state.pool, dari, hari).await
        } else {
            Ok(dari + Duration::days(hari))
        }
    }

    /// Setujui satu batch pada step saat ini.
    pub async fn setujui(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        batch_id: uuid::Uuid,
        catatan: Option<&str>,
    ) -> AppResult<Vec<InboxRow>> {
        let steps = Self::rantai(state).await?;
        let total_step = steps.len() as i16;

        let ids = ApprovalRepository::batch_ids(&state.pool, batch_id).await?;
        if ids.is_empty() {
            return Err(AppError::NotFound("Permintaan".into()));
        }

        // Semua baris batch harus berada pada step yang sama.
        let mut pertama: Option<InboxRow> = None;
        for id in &ids {
            let row = ApprovalRepository::find(&state.pool, *id)
                .await?
                .ok_or_else(|| AppError::NotFound("Permintaan".into()))?;
            if pertama.is_none() {
                pertama = Some(row);
                continue;
            }
            if row.status
                != pertama
                    .as_ref()
                    .map(|p| p.status.clone())
                    .unwrap_or_default()
            {
                return Err(AppError::Conflict(
                    "Baris dalam satu batch berada pada status berbeda".into(),
                ));
            }
        }
        let current = pertama.ok_or_else(|| AppError::NotFound("Permintaan".into()))?;

        // Tentukan step saat ini dari status.
        let step_sekarang = steps
            .iter()
            .find(|s| {
                let st = if s.nomor == 1 {
                    "SUBMITTED"
                } else {
                    status_untuk_step(s.nomor)
                };
                st == current.status
            })
            .ok_or_else(|| {
                AppError::Conflict(format!(
                    "Status '{}' tidak menunggu approval",
                    current.status
                ))
            })?;

        // Otorisasi: role actor harus role step ini (ADMIN selalu boleh).
        let boleh_skip = state
            .config
            .get_bool("B03", &ConfigContext::global())
            .await
            .unwrap_or(false);
        if actor.role != step_sekarang.role && actor.role != Role::Admin && !boleh_skip {
            return Err(AppError::Forbidden);
        }

        let status_baru = status_setelah_setuju(step_sekarang.nomor, total_step);
        let selesai = status_baru == "FINAL";
        let deadline = if selesai {
            None
        } else {
            Some(Self::deadline_untuk(state, status_baru, Utc::now()).await?)
        };

        let mut tx = state.pool.begin().await?;
        for id in &ids {
            ApprovalRepository::insert_log(
                &mut tx,
                *id,
                step_sekarang.nomor,
                actor.id,
                "APPROVE",
                catatan,
            )
            .await?;
            ApprovalRepository::close_sla(&mut tx, *id).await?;
            ApprovalRepository::set_status_step(
                &mut tx,
                *id,
                status_baru,
                step_sekarang.nomor + 1,
                deadline,
            )
            .await?;
            if !selesai {
                ApprovalRepository::open_sla(
                    &mut tx,
                    *id,
                    status_baru,
                    deadline.ok_or_else(|| AppError::Internal)?,
                )
                .await?;
            }
        }
        tx.commit().await?;

        // Snapshot config saat approve (§8): B01, B02, C01.
        let ctx_snap = ConfigContext {
            ref_type: Some("permintaan".to_string()),
            ref_id: Some(ids[0]),
            ruangan_id: None,
            kategori_id: None,
        };
        for id in &ids {
            let mut ctx = ctx_snap.clone();
            ctx.ref_id = Some(*id);
            state
                .config
                .snapshot("permintaan", *id, &["B01", "B02", "C01"], &ctx)
                .await?;
        }

        for id in &ids {
            record_audit(
                &state.pool,
                actor.id,
                "permintaan",
                *id,
                if selesai { "APPROVE_FINAL" } else { "APPROVE" },
                Some(&serde_json::json!({"status": current.status})),
                Some(&serde_json::json!({"status": status_baru, "step": step_sekarang.nomor})),
            )
            .await?;
        }

        // Notifikasi: ke pemohon, dan ke approver step berikutnya.
        let pesan_pemohon = if selesai {
            format!(
                "Permintaan Anda dari ruangan {} telah DISETUJUI FINAL ({} item)",
                current.ruangan_nama,
                ids.len()
            )
        } else {
            format!(
                "Permintaan Anda dari ruangan {} disetujui {} — menunggu {}",
                current.ruangan_nama,
                step_sekarang.label,
                status_baru.replace('_', " ")
            )
        };
        crate::notifikasi::kirim_ke_user(
            state,
            current.created_by,
            if selesai { "APPROVE_FINAL" } else { "APPROVE" },
            Some(ids[0]),
            &pesan_pemohon,
        )
        .await?;

        if !selesai {
            if let Some(berikut) = steps.iter().find(|s| s.nomor == step_sekarang.nomor + 1) {
                crate::notifikasi::kirim_ke_role(
                    state,
                    berikut.role.as_str(),
                    "APPROVE",
                    Some(ids[0]),
                    &format!(
                        "Permintaan dari ruangan {} menunggu {} Anda",
                        current.ruangan_nama, berikut.label
                    ),
                )
                .await?;
            }
        }

        self_batch(state, batch_id).await
    }

    /// Tolak pada step saat ini → RETURNED (kembali DRAFT) dengan catatan wajib.
    pub async fn tolak(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        batch_id: uuid::Uuid,
        catatan: Option<&str>,
    ) -> AppResult<Vec<InboxRow>> {
        let wajib_catatan = state
            .config
            .get_bool("B05", &ConfigContext::global())
            .await
            .unwrap_or(true);
        let catatan_bersih = catatan.map(str::trim).filter(|s| !s.is_empty());
        if wajib_catatan && catatan_bersih.is_none() {
            return Err(AppError::Validation(
                "Catatan wajib diisi saat menolak permintaan".into(),
            ));
        }

        let steps = Self::rantai(state).await?;
        let ids = ApprovalRepository::batch_ids(&state.pool, batch_id).await?;
        if ids.is_empty() {
            return Err(AppError::NotFound("Permintaan".into()));
        }

        let current = ApprovalRepository::find(&state.pool, ids[0])
            .await?
            .ok_or_else(|| AppError::NotFound("Permintaan".into()))?;

        let step_sekarang = steps
            .iter()
            .find(|s| {
                let st = if s.nomor == 1 {
                    "SUBMITTED"
                } else {
                    status_untuk_step(s.nomor)
                };
                st == current.status
            })
            .ok_or_else(|| {
                AppError::Conflict(format!(
                    "Status '{}' tidak menunggu approval",
                    current.status
                ))
            })?;

        let boleh_skip = state
            .config
            .get_bool("B03", &ConfigContext::global())
            .await
            .unwrap_or(false);
        if actor.role != step_sekarang.role && actor.role != Role::Admin && !boleh_skip {
            return Err(AppError::Forbidden);
        }

        let mut tx = state.pool.begin().await?;
        for id in &ids {
            ApprovalRepository::insert_log(
                &mut tx,
                *id,
                step_sekarang.nomor,
                actor.id,
                "REJECT",
                catatan_bersih,
            )
            .await?;
            ApprovalRepository::close_sla(&mut tx, *id).await?;
            // RETURNED: kembali ke DRAFT agar pemohon dapat memperbaiki.
            ApprovalRepository::set_status_step(&mut tx, *id, "RETURNED", 0, None).await?;
        }
        tx.commit().await?;

        for id in &ids {
            record_audit(
                &state.pool,
                actor.id,
                "permintaan",
                *id,
                "REJECT",
                Some(&serde_json::json!({"status": current.status})),
                Some(&serde_json::json!({"status": "RETURNED", "catatan": catatan_bersih})),
            )
            .await?;
        }

        crate::notifikasi::kirim_ke_user(
            state,
            current.created_by,
            "REJECT",
            Some(ids[0]),
            &format!(
                "Permintaan Anda dari ruangan {} DITOLAK pada {}. Catatan: {}",
                current.ruangan_nama,
                step_sekarang.label,
                catatan_bersih.unwrap_or("-")
            ),
        )
        .await?;

        self_batch(state, batch_id).await
    }
}

/// Ambil seluruh baris satu batch (dipakai sebagai respons).
async fn self_batch(state: &AppState, batch_id: uuid::Uuid) -> AppResult<Vec<InboxRow>> {
    let ids = ApprovalRepository::batch_ids(&state.pool, batch_id).await?;
    let mut rows = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(r) = ApprovalRepository::find(&state.pool, id).await? {
            rows.push(r);
        }
    }
    Ok(rows)
}

/// Tambah N hari kerja dengan melewati Sabtu/Minggu dan `hari_libur`.
pub async fn hitung_hari_kerja(
    pool: &sqlx::PgPool,
    mulai: DateTime<Utc>,
    hari: i64,
) -> AppResult<DateTime<Utc>> {
    let mut current = mulai;
    let mut sisa = hari.max(0);
    let mut pengaman = 0;

    while sisa > 0 && pengaman < 400 {
        current += Duration::days(1);
        pengaman += 1;
        let tgl = current.date_naive();
        if matches!(tgl.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun) {
            continue;
        }
        let libur = sqlx::query_scalar!("SELECT COUNT(*) FROM hari_libur WHERE tanggal = $1", tgl)
            .fetch_one(pool)
            .await?
            .unwrap_or(0)
            > 0;
        if libur {
            continue;
        }
        sisa -= 1;
    }
    Ok(current)
}

/// Peta step → label untuk laporan (dipakai handler).
pub fn label_step_map(steps: &[Step]) -> BTreeMap<i16, String> {
    steps.iter().map(|s| (s.nomor, s.label.clone())).collect()
}
