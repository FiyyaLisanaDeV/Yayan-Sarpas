//! Service serah terima — BAST, konfirmasi ruangan, dispute, auto-draft aset.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde_json::json;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{SerahRepository, SerahRow};

pub struct SerahInput {
    pub penerimaan_id: i64,
    pub ruangan_id: i64,
    pub penerima_ruangan_id: i64,
    pub qty: Option<BigDecimal>,
    pub tanggal: Option<NaiveDate>,
    pub catatan: Option<String>,
}

pub struct SerahTerimaService;

impl SerahTerimaService {
    /// Buat BAST (status PENDING, menunggu konfirmasi ruangan bila G01).
    pub async fn buat(
        state: &AppState,
        actor_id: i64,
        input: SerahInput,
        tahun: i16,
    ) -> AppResult<SerahRow> {
        let ctx = ConfigContext::global();
        let wajib_konfirmasi = state.config.get_bool("G01", &ctx).await.unwrap_or(true);

        // Validasi penerimaan & ruangan.
        let pn = sqlx::query!(
            r#"SELECT pn.id, pn.qty_terima FROM penerimaan pn WHERE pn.id = $1"#,
            input.penerimaan_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Penerimaan".into()))?;

        let sudah: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM serah_terima WHERE penerimaan_id = $1")
                .bind(input.penerimaan_id)
                .fetch_one(&state.pool)
                .await?;
        if sudah > 0 {
            return Err(AppError::Conflict(
                "Penerimaan ini sudah memiliki serah terima".into(),
            ));
        }

        let ruangan_ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM ruangan WHERE id = $1 AND is_active")
                .bind(input.ruangan_id)
                .fetch_one(&state.pool)
                .await?;
        if ruangan_ok == 0 {
            return Err(AppError::Validation(
                "Ruangan tidak ditemukan atau nonaktif".into(),
            ));
        }

        let penerima_ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1 AND is_active")
                .bind(input.penerima_ruangan_id)
                .fetch_one(&state.pool)
                .await?;
        if penerima_ok == 0 {
            return Err(AppError::Validation("Penerima ruangan tidak valid".into()));
        }

        let qty = input.qty.unwrap_or(pn.qty_terima);
        if qty.sign() != bigdecimal::num_bigint::Sign::Plus {
            return Err(AppError::Validation("Qty serah harus lebih dari 0".into()));
        }

        let format_ba = state
            .config
            .get_string("J04", &ctx)
            .await
            .unwrap_or_else(|_| "BAST/{tahun}/{seq:04}".to_string());

        let mut tx = state.pool.begin().await?;
        let no_ba = SerahRepository::next_no_ba(&mut tx, &format_ba, tahun).await?;
        let tanggal = input
            .tanggal
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        let id = SerahRepository::insert(
            &mut tx,
            input.penerimaan_id,
            input.ruangan_id,
            tanggal,
            &qty,
            input.penerima_ruangan_id,
            actor_id,
            &no_ba,
            input.catatan.as_deref(),
        )
        .await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "serah_terima",
            id,
            "CREATE",
            None,
            Some(&json!({
                "no_ba": no_ba, "penerimaan_id": input.penerimaan_id,
                "ruangan_id": input.ruangan_id, "qty": qty.to_string(),
                "wajib_konfirmasi": wajib_konfirmasi,
            })),
        )
        .await?;

        // Snapshot (§8): G01, G02, G03, G05.
        let mut snap = ConfigContext::global();
        snap.ref_type = Some("serah_terima".to_string());
        snap.ref_id = Some(id);
        let _ = state
            .config
            .snapshot("serah_terima", id, &["G01", "G02", "G03", "G05"], &snap)
            .await;

        // Notifikasi ke penerima ruangan.
        crate::notifikasi::kirim_ke_user(
            state,
            input.penerima_ruangan_id,
            "SERAH_TERIMA",
            Some(id),
            &format!("Barang siap diserahterimakan ({no_ba}) — mohon konfirmasi"),
        )
        .await?;

        SerahRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Konfirmasi penerimaan barang oleh ruangan.
    pub async fn konfirmasi(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        id: i64,
    ) -> AppResult<SerahRow> {
        let row = SerahRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Serah terima".into()))?;

        if row.status_konfirmasi != "PENDING" {
            return Err(AppError::Conflict(format!(
                "Status {} — hanya PENDING yang dapat dikonfirmasi",
                row.status_konfirmasi
            )));
        }

        // Hanya penerima (atau ADMIN) yang boleh konfirmasi.
        if actor.id != row.penerima_ruangan_id && actor.role != sarpras_shared::rbac::Role::Admin {
            return Err(AppError::Forbidden);
        }

        let mut tx = state.pool.begin().await?;
        SerahRepository::set_status(&mut tx, id, "CONFIRMED").await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor.id,
            "serah_terima",
            id,
            "KONFIRMASI",
            Some(&json!({"status_konfirmasi": "PENDING"})),
            Some(&json!({"status_konfirmasi": "CONFIRMED"})),
        )
        .await?;

        // Auto-draft aset bila nilai ≥ threshold G03 (G04).
        if let Err(e) =
            crate::domains::aset::service::AsetService::auto_draft_dari_serah(state, id).await
        {
            tracing::warn!(error = %e, serah_id = id, "gagal membuat draft aset otomatis");
        }

        SerahRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Ruangan menyatakan dispute — perlu resolusi Kasi.
    pub async fn dispute(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        id: i64,
        catatan: Option<String>,
    ) -> AppResult<SerahRow> {
        let row = SerahRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Serah terima".into()))?;
        if row.status_konfirmasi != "PENDING" {
            return Err(AppError::Conflict(
                "Hanya PENDING yang dapat di-dispute".into(),
            ));
        }
        if actor.id != row.penerima_ruangan_id && actor.role != sarpras_shared::rbac::Role::Admin {
            return Err(AppError::Forbidden);
        }

        let mut tx = state.pool.begin().await?;
        SerahRepository::set_status(&mut tx, id, "DISPUTED").await?;
        if let Some(c) = catatan.as_deref().filter(|s| !s.trim().is_empty()) {
            sqlx::query!(
                "UPDATE serah_terima SET catatan = COALESCE(catatan || E'\\n', '') || $2 WHERE id = $1",
                id,
                c
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor.id,
            "serah_terima",
            id,
            "DISPUTE",
            None,
            Some(&json!({"catatan": catatan})),
        )
        .await?;

        SerahRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Resolusi dispute oleh Kasi → CONFIRMED.
    pub async fn resolve(
        state: &AppState,
        actor_id: i64,
        id: i64,
        catatan: Option<String>,
    ) -> AppResult<SerahRow> {
        let row = SerahRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Serah terima".into()))?;
        if row.status_konfirmasi != "DISPUTED" {
            return Err(AppError::Conflict(
                "Hanya DISPUTED yang dapat diresolusi".into(),
            ));
        }

        let mut tx = state.pool.begin().await?;
        SerahRepository::set_status(&mut tx, id, "CONFIRMED").await?;
        if let Some(c) = catatan.as_deref().filter(|s| !s.trim().is_empty()) {
            sqlx::query!(
                "UPDATE serah_terima SET catatan = COALESCE(catatan || E'\\n', '') || $2 WHERE id = $1",
                id,
                c
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "serah_terima",
            id,
            "RESOLVE",
            Some(&json!({"status_konfirmasi": "DISPUTED"})),
            Some(&json!({"status_konfirmasi": "CONFIRMED", "catatan": catatan})),
        )
        .await?;

        if let Err(e) =
            crate::domains::aset::service::AsetService::auto_draft_dari_serah(state, id).await
        {
            tracing::warn!(error = %e, serah_id = id, "gagal membuat draft aset otomatis");
        }

        SerahRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn list(
        state: &AppState,
        page: i64,
        per_page: i64,
        status: Option<&str>,
        ruangan_id: Option<i64>,
    ) -> AppResult<(Vec<SerahRow>, i64)> {
        SerahRepository::list(&state.pool, page, per_page, status, ruangan_id).await
    }

    pub async fn siap_diserehkan(state: &AppState) -> AppResult<Vec<serde_json::Value>> {
        SerahRepository::penerimaan_siap(&state.pool).await
    }
}
