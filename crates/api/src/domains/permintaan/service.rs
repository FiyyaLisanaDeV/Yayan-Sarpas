//! Service permintaan — aturan bisnis, snapshot config, notifikasi, audit.

use bigdecimal::BigDecimal;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{PermintaanRepository, PermintaanRow};

/// Satu baris input form.
#[derive(Debug, Clone, Deserialize)]
pub struct ItemInput {
    pub item_id: i64,
    pub jumlah: BigDecimal,
    pub satuan_id: i64,
    pub keterangan: Option<String>,
}

/// True bila jumlah tidak positif (0 atau negatif).
fn tidak_positif(v: &BigDecimal) -> bool {
    use bigdecimal::Zero;
    v.is_zero() || v.sign() == bigdecimal::num_bigint::Sign::Minus
}

pub struct PermintaanService;

/// Konfigurasi aktif yang dipakai sepanjang satu operasi.
struct Aturan {
    tahun: i16,
    periode_aktif: bool,
    maks_item: i64,
    maks_qty: f64,
    lampiran_wajib: bool,
    sla_hari: i64,
    basis_kerja: bool,
}

impl PermintaanService {
    /// Batas periode dari E01–E04 dibandingkan tanggal hari ini.
    fn dalam_periode(
        today: NaiveDate,
        buka_bulan: i64,
        buka_tgl: i64,
        tutup_bulan: i64,
        tutup_tgl: i64,
    ) -> bool {
        let buka = NaiveDate::from_ymd_opt(today.year(), buka_bulan as u32, buka_tgl as u32);
        let tutup = NaiveDate::from_ymd_opt(today.year(), tutup_bulan as u32, tutup_tgl as u32);
        match (buka, tutup) {
            (Some(b), Some(t)) => today >= b && today <= t,
            _ => false,
        }
    }

    async fn muat_aturan(state: &AppState) -> AppResult<Aturan> {
        let ctx = ConfigContext::global();
        let today = Utc::now().date_naive();

        let buka_bulan = state.config.get_i64("E01", &ctx).await?;
        let buka_tgl = state.config.get_i64("E02", &ctx).await?;
        let tutup_bulan = state.config.get_i64("E03", &ctx).await?;
        let tutup_tgl = state.config.get_i64("E04", &ctx).await?;
        let basis = state.config.get_string("C03", &ctx).await?;

        Ok(Aturan {
            tahun: today.year() as i16,
            periode_aktif: Self::dalam_periode(today, buka_bulan, buka_tgl, tutup_bulan, tutup_tgl),
            maks_item: state.config.get_i64("E06", &ctx).await?,
            maks_qty: state.config.get_f64("E07", &ctx).await?,
            lampiran_wajib: state.config.get_bool("E08", &ctx).await?,
            sla_hari: state.config.get_i64("C01", &ctx).await?,
            basis_kerja: basis.eq_ignore_ascii_case("KERJA"),
        })
    }

    /// Validasi input baris (jumlah & referensi item/satuan).
    async fn validasi_baris(
        state: &AppState,
        items: &[ItemInput],
        aturan: &Aturan,
    ) -> AppResult<()> {
        if items.is_empty() {
            return Err(AppError::Validation("Minimal satu item harus diisi".into()));
        }
        if aturan.maks_item > 0 && items.len() as i64 > aturan.maks_item {
            return Err(AppError::Validation(format!(
                "Maksimal {} item per permintaan",
                aturan.maks_item
            )));
        }

        for (idx, it) in items.iter().enumerate() {
            let no = idx + 1;
            if tidak_positif(&it.jumlah) {
                return Err(AppError::Validation(format!(
                    "Jumlah baris {no} harus lebih dari 0"
                )));
            }
            if aturan.maks_qty > 0.0 {
                let qty: f64 = it.jumlah.to_string().parse().unwrap_or(f64::MAX);
                if qty > aturan.maks_qty {
                    return Err(AppError::Validation(format!(
                        "Jumlah baris {no} melebihi batas {}",
                        aturan.maks_qty
                    )));
                }
            }

            let item_ok = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM items WHERE id = $1 AND is_active",
                it.item_id
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0);
            if item_ok == 0 {
                return Err(AppError::Validation(format!(
                    "Item baris {no} tidak ditemukan atau nonaktif"
                )));
            }

            let satuan_ok =
                sqlx::query_scalar!("SELECT COUNT(*) FROM satuan WHERE id = $1", it.satuan_id)
                    .fetch_one(&state.pool)
                    .await?
                    .unwrap_or(0);
            if satuan_ok == 0 {
                return Err(AppError::Validation(format!(
                    "Satuan baris {no} tidak ditemukan"
                )));
            }
        }
        Ok(())
    }

    async fn pastikan_ruangan(state: &AppState, ruangan_id: i64) -> AppResult<()> {
        let ok = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM ruangan WHERE id = $1 AND is_active",
            ruangan_id
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(0);
        if ok == 0 {
            return Err(AppError::Validation(
                "Ruangan tidak ditemukan atau nonaktif".into(),
            ));
        }
        Ok(())
    }

    /// Buat draft multi-row. Mengembalikan batch_id.
    pub async fn create_draft(
        state: &AppState,
        actor_id: i64,
        ruangan_id: i64,
        items: &[ItemInput],
        lampiran_ada: bool,
    ) -> AppResult<Uuid> {
        let aturan = Self::muat_aturan(state).await?;
        Self::pastikan_ruangan(state, ruangan_id).await?;
        Self::validasi_baris(state, items, &aturan).await?;

        if aturan.lampiran_wajib && !lampiran_ada {
            return Err(AppError::Validation(
                "Lampiran wajib diunggah untuk permintaan ini".into(),
            ));
        }

        let batch_id = Uuid::new_v4();
        let mut tx = state.pool.begin().await?;

        let mut ids = Vec::with_capacity(items.len());
        for it in items {
            let id = PermintaanRepository::insert_row(
                &mut tx,
                batch_id,
                aturan.tahun,
                ruangan_id,
                it.item_id,
                &it.jumlah,
                it.satuan_id,
                it.keterangan.as_deref(),
                actor_id,
            )
            .await?;
            ids.push(id);
        }

        tx.commit().await?;

        for id in &ids {
            record_audit(
                &state.pool,
                actor_id,
                "permintaan",
                *id,
                "CREATE",
                None,
                Some(&json!({"batch_id": batch_id, "status": "DRAFT"})),
            )
            .await?;
        }

        Ok(batch_id)
    }

    /// Submit satu batch draft → SUBMITTED, plus snapshot & notifikasi.
    pub async fn submit(
        state: &AppState,
        actor_id: i64,
        batch_id: Uuid,
        boleh_di_luar_periode: bool,
    ) -> AppResult<Vec<PermintaanRow>> {
        let aturan = Self::muat_aturan(state).await?;
        if !aturan.periode_aktif && !boleh_di_luar_periode {
            let urgent = state
                .config
                .get_bool("E09", &ConfigContext::global())
                .await
                .unwrap_or(false);
            if !urgent {
                return Err(AppError::Validation(
                    "Di luar periode permintaan. Hubungi Kasi untuk pengajuan urgent.".into(),
                ));
            }
        }

        let rows = PermintaanRepository::by_batch(&state.pool, batch_id).await?;
        if rows.is_empty() {
            return Err(AppError::NotFound("Permintaan".into()));
        }
        if rows.iter().any(|r| r.created_by != actor_id) {
            return Err(AppError::Forbidden);
        }
        // Submit ulang setelah penolakan (RETURNED) diizinkan; status lain tidak.
        let ada_returned = rows.iter().any(|r| r.status == "RETURNED");
        let resubmit = ada_returned;
        if resubmit {
            if rows.iter().any(|r| r.status != "RETURNED") {
                return Err(AppError::Conflict(
                    "Semua baris pengajuan harus berstatus sama untuk diajukan ulang".into(),
                ));
            }
        } else if rows.iter().any(|r| r.status != "DRAFT") {
            return Err(AppError::Conflict(
                "Hanya permintaan berstatus DRAFT atau hasil penolakan yang dapat diajukan".into(),
            ));
        }

        let ruangan_id = rows[0].ruangan_id;

        // Deadline SLA: B02 (sla per step) atau C01, basis KERJA/KALENDER (C03).
        let sla_hari = {
            let ctx_step = ConfigContext {
                ref_type: None,
                ref_id: None,
                ruangan_id: Some(ruangan_id),
                kategori_id: None,
            };
            let per_step: serde_json::Value = state
                .config
                .get_json("C02", &ctx_step)
                .await
                .unwrap_or(serde_json::json!({}));
            per_step
                .get("SUBMITTED")
                .and_then(|v| v.as_i64())
                .unwrap_or(aturan.sla_hari)
        };

        let deadline = if aturan.basis_kerja {
            hitung_hari_kerja(&state.pool, Utc::now(), sla_hari).await?
        } else {
            Utc::now() + Duration::days(sla_hari)
        };

        let mut tx = state.pool.begin().await?;
        for r in &rows {
            if resubmit {
                PermintaanRepository::mark_resubmitted(&mut tx, r.id, Some(deadline)).await?;
            } else {
                PermintaanRepository::mark_submitted(&mut tx, r.id, "SUBMITTED", Some(deadline))
                    .await?;
            }
            // Riwayat pengajuan (approval_log) memuat SUBMIT/RESUBMIT agar
            // pemohon & approver melihat jejak lengkap termasuk revisi.
            crate::domains::approval::repository::ApprovalRepository::insert_log(
                &mut tx,
                r.id,
                0,
                actor_id,
                if resubmit { "RESUBMIT" } else { "SUBMIT" },
                None,
            )
            .await?;
        }
        tx.commit().await?;

        // Snapshot config sesuai §8 (submit permintaan).
        let ctx_ruangan = ConfigContext {
            ref_type: Some("permintaan".to_string()),
            ref_id: Some(rows[0].id),
            ruangan_id: Some(ruangan_id),
            kategori_id: None,
        };
        let kode_snapshot = ["C01", "C02", "E11", "F11", "G03", "J04"];
        for r in &rows {
            let mut ctx = ctx_ruangan.clone();
            ctx.ref_id = Some(r.id);
            state
                .config
                .snapshot("permintaan", r.id, &kode_snapshot, &ctx)
                .await?;
        }

        // Notifikasi ke KASI_SARPRAS + audit.
        let total_item = rows.len();
        let status_sebelum = if resubmit { "RETURNED" } else { "DRAFT" };
        for r in &rows {
            record_audit(
                &state.pool,
                actor_id,
                "permintaan",
                r.id,
                if resubmit { "RESUBMIT" } else { "SUBMIT" },
                Some(&json!({"status": status_sebelum})),
                Some(&json!({"status": "SUBMITTED", "sla_deadline": deadline})),
            )
            .await?;
        }

        crate::notifikasi::kirim_ke_role(
            state,
            "KASI_SARPRAS",
            "SUBMIT",
            Some(rows[0].id),
            &format!(
                "{} dari ruangan {} ({total_item} item) menunggu verifikasi",
                if resubmit {
                    "Permintaan revisi diajukan ulang"
                } else {
                    "Permintaan baru"
                },
                rows[0].ruangan_nama
            ),
        )
        .await?;

        PermintaanRepository::by_batch(&state.pool, batch_id).await
    }

    /// Ubah satu baris selama DRAFT dan milik actor.
    pub async fn update_row(
        state: &AppState,
        actor_id: i64,
        id: i64,
        jumlah: BigDecimal,
        satuan_id: i64,
        keterangan: Option<String>,
    ) -> AppResult<()> {
        let row = PermintaanRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Permintaan".into()))?;

        if row.created_by != actor_id {
            return Err(AppError::Forbidden);
        }
        // DRAFT dan RETURNED (hasil penolakan) boleh diedit untuk revisi.
        if row.status != "DRAFT" && row.status != "RETURNED" {
            let batas = state
                .config
                .get_string("E10", &ConfigContext::global())
                .await
                .unwrap_or_else(|_| "SEBELUM_VERIFIKASI".to_string());
            return Err(AppError::Conflict(format!(
                "Permintaan tidak dapat diubah (status {}, batas edit: {batas})",
                row.status
            )));
        }
        if tidak_positif(&jumlah) {
            return Err(AppError::Validation("Jumlah harus lebih dari 0".into()));
        }

        let mut tx = state.pool.begin().await?;
        PermintaanRepository::update_draft(&mut tx, id, &jumlah, satuan_id, keterangan.as_deref())
            .await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "permintaan",
            id,
            "UPDATE",
            Some(&json!({"jumlah": row.jumlah.to_string()})),
            Some(&json!({"jumlah": jumlah.to_string()})),
        )
        .await?;
        Ok(())
    }

    /// Batalkan seluruh batch DRAFT.
    pub async fn delete_draft(state: &AppState, actor_id: i64, batch_id: Uuid) -> AppResult<u64> {
        let rows = PermintaanRepository::by_batch(&state.pool, batch_id).await?;
        if rows.is_empty() {
            return Err(AppError::NotFound("Permintaan".into()));
        }
        if rows.iter().any(|r| r.created_by != actor_id) {
            return Err(AppError::Forbidden);
        }
        if rows.iter().any(|r| r.status != "DRAFT") {
            let batas = state
                .config
                .get_string("E11", &ConfigContext::global())
                .await
                .unwrap_or_else(|_| "SEBELUM_APPROVAL_WADIR".to_string());
            return Err(AppError::Conflict(format!(
                "Hanya permintaan DRAFT yang dapat dibatalkan (batas batal: {batas})"
            )));
        }

        let mut tx = state.pool.begin().await?;
        let n = PermintaanRepository::delete_draft_batch(&mut tx, batch_id, actor_id).await?;
        tx.commit().await?;

        for r in &rows {
            record_audit(
                &state.pool,
                actor_id,
                "permintaan",
                r.id,
                "DELETE",
                Some(&json!({"status": r.status, "batch_id": batch_id})),
                None,
            )
            .await?;
        }
        Ok(n)
    }
}

/// Tambah N hari kerja dengan melewati Sabtu/Minggu dan `hari_libur`.
async fn hitung_hari_kerja(
    pool: &sqlx::PgPool,
    mulai: chrono::DateTime<Utc>,
    hari: i64,
) -> AppResult<chrono::DateTime<Utc>> {
    let mut current = mulai;
    let mut sisa = hari.max(0);
    let mut pengaman = 0;

    while sisa > 0 && pengaman < 400 {
        current += Duration::days(1);
        pengaman += 1;
        let tgl = current.date_naive();
        let weekend = matches!(tgl.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);
        if weekend {
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
