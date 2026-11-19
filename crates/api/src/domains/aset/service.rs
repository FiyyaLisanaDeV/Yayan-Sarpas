//! Service aset — registrasi, kode aset (A05), QR (G05), mutasi, auto-draft.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde_json::json;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{AsetRepository, AsetRow, MutasiRow};

pub struct RegistrasiInput {
    pub item_id: i64,
    pub ruangan_id: i64,
    pub no_seri: Option<String>,
    pub tgl_perolehan: Option<NaiveDate>,
    pub nilai: BigDecimal,
    pub umur_ekonomis: Option<i16>,
    pub kondisi: Option<String>,
    pub kode_ruangan: Option<String>,
}

pub struct AsetService;

impl AsetService {
    /// Bentuk kode aset dari format A05.
    fn bentuk_kode(format: &str, tahun: i16, kode_ruangan: &str, seq: i64) -> String {
        format
            .replace("{tahun}", &tahun.to_string())
            .replace("{kode_ruangan}", kode_ruangan)
            .replace("{seq:04}", &format!("{seq:04}"))
            .replace("{seq}", &seq.to_string())
    }

    pub async fn registrasi(
        state: &AppState,
        actor_id: i64,
        input: RegistrasiInput,
    ) -> AppResult<AsetRow> {
        use bigdecimal::Zero;
        if input.nilai.is_zero() || input.nilai.sign() == bigdecimal::num_bigint::Sign::Minus {
            return Err(AppError::Validation("Nilai aset harus lebih dari 0".into()));
        }

        let item = sqlx::query!(
            r#"SELECT i.nama, k.kode AS kategori FROM items i
               JOIN kategori k ON k.id = i.kategori_id
               WHERE i.id = $1 AND i.is_active"#,
            input.item_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::Validation("Item tidak ditemukan atau nonaktif".into()))?;

        let ruangan = sqlx::query!(
            "SELECT kode, nama FROM ruangan WHERE id = $1 AND is_active",
            input.ruangan_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::Validation("Ruangan tidak ditemukan atau nonaktif".into()))?;

        let ctx = ConfigContext::global();
        let format = state
            .config
            .get_string("A05", &ctx)
            .await
            .unwrap_or_else(|_| "ALKES/{tahun}/{kode_ruangan}/{seq:04}".to_string());
        let format = format.replace("ALKES/", &format!("{}/", item.kategori));

        let tanggal = input
            .tgl_perolehan
            .unwrap_or_else(|| chrono::Utc::now().date_naive());
        let tahun = tanggal
            .format("%Y")
            .to_string()
            .parse::<i16>()
            .unwrap_or(2026);
        let kode_ruangan = input
            .kode_ruangan
            .clone()
            .unwrap_or_else(|| ruangan.kode.clone());

        let mut tx = state.pool.begin().await?;
        let mut seq = AsetRepository::next_seq(&mut tx, tahun).await?;
        let mut kode = Self::bentuk_kode(&format, tahun, &kode_ruangan, seq);
        // Hindari tabrakan kode.
        let mut putaran = 0;
        while AsetRepository::kode_aset_terpakai(&mut tx, &kode).await? && putaran < 100 {
            seq += 1;
            putaran += 1;
            kode = Self::bentuk_kode(&format, tahun, &kode_ruangan, seq);
        }

        let id = AsetRepository::insert(
            &mut tx,
            input.item_id,
            input.ruangan_id,
            &kode,
            input
                .no_seri
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            tanggal,
            &input.nilai,
            input.umur_ekonomis,
            input
                .kondisi
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            "REGISTERED",
        )
        .await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "aset_tetap",
            id,
            "CREATE",
            None,
            Some(&json!({
                "kode_aset": kode, "item": item.nama, "nilai": input.nilai.to_string(),
                "ruangan": ruangan.nama,
            })),
        )
        .await?;

        // Snapshot (§8): A05, G03, G05.
        let mut snap = ConfigContext::global();
        snap.ref_type = Some("aset_tetap".to_string());
        snap.ref_id = Some(id);
        let _ = state
            .config
            .snapshot("aset_tetap", id, &["A05", "G03", "G05"], &snap)
            .await;

        AsetRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Auto-draft aset dari serah terima yang CONFIRMED bila nilai ≥ threshold G03.
    pub async fn auto_draft_dari_serah(state: &AppState, serah_id: i64) -> AppResult<Option<i64>> {
        let ctx = ConfigContext::global();
        let auto = state.config.get_bool("G04", &ctx).await.unwrap_or(true);
        if !auto {
            return Ok(None);
        }
        let threshold: f64 = state
            .config
            .get_f64("G03", &ctx)
            .await
            .unwrap_or(1_000_000.0);

        let row = sqlx::query!(
            r#"SELECT st.qty_serah, st.ruangan_id, st.tanggal_serah,
                      d.item_id, d.harga_satuan, i.threshold_aset, k.kode AS kategori,
                      r.kode AS kode_ruangan
               FROM serah_terima st
               JOIN penerimaan pn ON pn.id = st.penerimaan_id
               JOIN po_detail d ON d.id = pn.po_detail_id
               JOIN items i ON i.id = d.item_id
               JOIN kategori k ON k.id = i.kategori_id
               JOIN ruangan r ON r.id = st.ruangan_id
               WHERE st.id = $1"#,
            serah_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Serah terima".into()))?;

        let nilai: BigDecimal = &row.qty_serah * &row.harga_satuan;
        let nilai_f64: f64 = nilai.to_string().parse().unwrap_or(0.0);

        // Threshold per item (bila ada) mengalahkan G03.
        let ambang = row
            .threshold_aset
            .as_ref()
            .and_then(|t| t.to_string().parse::<f64>().ok())
            .unwrap_or(threshold);

        if nilai_f64 < ambang {
            return Ok(None);
        }

        // Sudah pernah dibuat?
        let sudah: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM aset_tetap WHERE kode_aset LIKE $1")
                .bind(format!("%/{}%", row.tanggal_serah))
                .fetch_one(&state.pool)
                .await
                .unwrap_or(0);
        let _ = sudah;

        let format_cfg = state
            .config
            .get_string("A05", &ctx)
            .await
            .unwrap_or_else(|_| "{tahun}/{kode_ruangan}/{seq:04}".to_string());
        let format = format_cfg.replace("ALKES/", &format!("{}/", row.kategori));
        let tahun = row
            .tanggal_serah
            .format("%Y")
            .to_string()
            .parse::<i16>()
            .unwrap_or(2026);

        let mut tx = state.pool.begin().await?;
        let mut seq = AsetRepository::next_seq(&mut tx, tahun).await?;
        let mut kode = Self::bentuk_kode(&format, tahun, &row.kode_ruangan, seq);
        let mut putaran = 0;
        while AsetRepository::kode_aset_terpakai(&mut tx, &kode).await? && putaran < 100 {
            seq += 1;
            putaran += 1;
            kode = Self::bentuk_kode(&format, tahun, &row.kode_ruangan, seq);
        }

        let id = AsetRepository::insert(
            &mut tx,
            row.item_id,
            row.ruangan_id,
            &kode,
            None,
            row.tanggal_serah,
            &nilai,
            None,
            Some("Baik"),
            "REGISTERED",
        )
        .await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            1,
            "aset_tetap",
            id,
            "AUTO_DRAFT",
            None,
            Some(&json!({"kode_aset": kode, "dari_serah": serah_id, "nilai": nilai.to_string()})),
        )
        .await?;

        Ok(Some(id))
    }

    /// Mutasi aset antar ruangan (approval WADIR bila G06 — dicatat sebagai audit).
    pub async fn mutasi(
        state: &AppState,
        actor_id: i64,
        aset_id: i64,
        ke_ruangan_id: i64,
        alasan: Option<String>,
        tanggal: Option<NaiveDate>,
    ) -> AppResult<MutasiRow> {
        let aset = AsetRepository::find(&state.pool, aset_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Aset".into()))?;

        if aset.ruangan_id == ke_ruangan_id {
            return Err(AppError::Validation(
                "Ruangan tujuan sama dengan asal".into(),
            ));
        }
        let tujuan_ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM ruangan WHERE id = $1 AND is_active")
                .bind(ke_ruangan_id)
                .fetch_one(&state.pool)
                .await?;
        if tujuan_ok == 0 {
            return Err(AppError::Validation(
                "Ruangan tujuan tidak ditemukan/nonaktif".into(),
            ));
        }

        let tanggal = tanggal.unwrap_or_else(|| chrono::Utc::now().date_naive());
        let mut tx = state.pool.begin().await?;
        let mutasi_id = AsetRepository::insert_mutasi(
            &mut tx,
            aset_id,
            aset.ruangan_id,
            ke_ruangan_id,
            tanggal,
            alasan.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            actor_id,
        )
        .await?;
        sqlx::query!(
            "UPDATE aset_tetap SET ruangan_id = $2 WHERE id = $1",
            aset_id,
            ke_ruangan_id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "mutasi_aset",
            mutasi_id,
            "CREATE",
            Some(&json!({"ruangan_id": aset.ruangan_id, "kode_aset": aset.kode_aset})),
            Some(&json!({"ruangan_id": ke_ruangan_id, "alasan": alasan})),
        )
        .await?;

        let rows = AsetRepository::list_mutasi(&state.pool, aset_id).await?;
        rows.into_iter()
            .find(|m| m.id == mutasi_id)
            .ok_or(AppError::Internal)
    }

    pub async fn list(
        state: &AppState,
        page: i64,
        per_page: i64,
        search: Option<&str>,
        ruangan_id: Option<i64>,
        status: Option<&str>,
    ) -> AppResult<(Vec<AsetRow>, i64)> {
        AsetRepository::list(&state.pool, page, per_page, search, ruangan_id, status).await
    }

    pub async fn find(state: &AppState, id: i64) -> AppResult<AsetRow> {
        AsetRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Aset".into()))
    }

    pub async fn ubah_status(
        state: &AppState,
        actor_id: i64,
        id: i64,
        status: &str,
    ) -> AppResult<AsetRow> {
        let valid = ["REGISTERED", "IN_USE", "MAINTAINED", "DISPOSED"];
        if !valid.contains(&status) {
            return Err(AppError::Validation(format!(
                "Status '{status}' tidak valid (pilihan: {})",
                valid.join(", ")
            )));
        }
        let before = Self::find(state, id).await?;
        AsetRepository::set_status(&state.pool, id, status).await?;
        record_audit(
            &state.pool,
            actor_id,
            "aset_tetap",
            id,
            "UPDATE_STATUS",
            Some(&json!({"status": before.status})),
            Some(&json!({"status": status})),
        )
        .await?;
        Self::find(state, id).await
    }

    pub async fn riwayat_mutasi(state: &AppState, aset_id: i64) -> AppResult<Vec<MutasiRow>> {
        AsetRepository::list_mutasi(&state.pool, aset_id).await
    }

    /// Simpan path QR yang sudah di-generate (handler yang menulis file).
    pub async fn simpan_qr(state: &AppState, id: i64, path: &str) -> AppResult<()> {
        AsetRepository::set_qr(&state.pool, id, path).await
    }
}
