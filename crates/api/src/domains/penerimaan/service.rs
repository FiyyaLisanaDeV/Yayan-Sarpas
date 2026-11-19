//! Service penerimaan — validasi dokumen wajib (F07), toleransi qty (F05), partial (F04).

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use serde_json::json;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{PenerimaanRepository, PenerimaanRow};

/// Input pencatatan penerimaan.
pub struct TerimaInput {
    pub po_detail_id: i64,
    pub qty_terima: BigDecimal,
    pub qty_reject: BigDecimal,
    pub kondisi: Option<String>,
    pub no_do: Option<String>,
    pub no_invoice: Option<String>,
    pub no_faktur_pajak: Option<String>,
    pub catatan: Option<String>,
    pub tanggal: Option<NaiveDate>,
}

pub struct PenerimaanService;

impl PenerimaanService {
    /// Catat penerimaan barang. Menyesuaikan status PO:
    /// PARTIAL_RECEIVED bila masih ada sisa, CLOSED bila semua qty terpenuhi (F08).
    pub async fn terima(
        state: &AppState,
        actor_id: i64,
        input: TerimaInput,
        tahun: i16,
    ) -> AppResult<PenerimaanRow> {
        let ctx = ConfigContext::global();
        let partial_allowed = state.config.get_bool("F04", &ctx).await.unwrap_or(true);
        let toleransi = state.config.get_f64("F05", &ctx).await.unwrap_or(0.0);
        let dokumen_wajib: Vec<String> = state
            .config
            .get_json::<serde_json::Value>("F07", &ctx)
            .await
            .ok()
            .and_then(|v| {
                v.as_array().map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
            })
            .unwrap_or_else(|| vec!["DO".into(), "INVOICE".into(), "FAKTUR_PAJAK".into()]);

        if input.qty_terima.is_zero()
            || input.qty_terima.sign() == bigdecimal::num_bigint::Sign::Minus
        {
            return Err(AppError::Validation("Qty terima harus lebih dari 0".into()));
        }
        if input.qty_reject.sign() == bigdecimal::num_bigint::Sign::Minus {
            return Err(AppError::Validation(
                "Qty reject tidak boleh negatif".into(),
            ));
        }

        // Dokumen wajib (F07).
        for dok in &dokumen_wajib {
            let ada = match dok.as_str() {
                "DO" => input
                    .no_do
                    .as_deref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false),
                "INVOICE" => input
                    .no_invoice
                    .as_deref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false),
                "FAKTUR_PAJAK" => input
                    .no_faktur_pajak
                    .as_deref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false),
                _ => true,
            };
            if !ada {
                return Err(AppError::Validation(format!(
                    "Dokumen {dok} wajib diisi (config F07)"
                )));
            }
        }

        let mut tx = state.pool.begin().await?;

        let (_qty_po, sisa) =
            PenerimaanRepository::sisa_detail(&mut tx, input.po_detail_id).await?;

        // Cek sisa & toleransi.
        let qty_f64: f64 = input.qty_terima.to_string().parse().unwrap_or(0.0);
        let sisa_f64: f64 = sisa.to_string().parse().unwrap_or(0.0);
        let batas = sisa_f64 * (1.0 + toleransi / 100.0);

        if qty_f64 > batas + 1e-9 {
            return Err(AppError::Validation(format!(
                "Qty terima {qty_f64} melebihi sisa {sisa_f64} (toleransi {toleransi}%)"
            )));
        }

        if !partial_allowed && qty_f64 < sisa_f64 - 1e-9 {
            return Err(AppError::Conflict(
                "Penerimaan partial tidak diizinkan (config F04) — qty harus penuh".into(),
            ));
        }

        let format_no = state
            .config
            .get_string("J05", &ctx)
            .await
            .unwrap_or_else(|_| "BAP/{tahun}/{seq:04}".to_string());
        let no_terima = PenerimaanRepository::next_no_terima(&mut tx, &format_no, tahun).await?;
        let tanggal = input
            .tanggal
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        let id = PenerimaanRepository::insert(
            &mut tx,
            input.po_detail_id,
            &no_terima,
            tanggal,
            &input.qty_terima,
            &input.qty_reject,
            input.kondisi.as_deref(),
            input.no_do.as_deref(),
            input.no_invoice.as_deref(),
            input.no_faktur_pajak.as_deref(),
            actor_id,
            input.catatan.as_deref(),
        )
        .await?;

        // Status PO: cek sisa seluruh detail.
        let po_id = PenerimaanRepository::po_id_dari_detail(&mut tx, input.po_detail_id).await?;
        let ada_sisa =
            crate::domains::po::repository::PoRepository::ada_sisa_belum_terima(&mut tx, po_id)
                .await?;

        let status_baru = if ada_sisa && partial_allowed {
            "PARTIAL_RECEIVED"
        } else if ada_sisa {
            "SENT"
        } else {
            "CLOSED"
        };
        crate::domains::po::repository::PoRepository::set_status(&mut tx, po_id, status_baru)
            .await?;

        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "penerimaan",
            id,
            "CREATE",
            None,
            Some(&json!({
                "po_detail_id": input.po_detail_id, "no_terima": no_terima,
                "qty_terima": input.qty_terima.to_string(), "status_po": status_baru,
            })),
        )
        .await?;

        // Snapshot (§8): F04, F05, F07.
        let mut snap = ConfigContext::global();
        snap.ref_type = Some("penerimaan".to_string());
        snap.ref_id = Some(id);
        let _ = state
            .config
            .snapshot("penerimaan", id, &["F04", "F05", "F07"], &snap)
            .await;

        // Notifikasi ke pembuat PO.
        if let Ok(Some(po)) =
            crate::domains::po::repository::PoRepository::find(&state.pool, po_id).await
        {
            crate::notifikasi::kirim_ke_user(
                state,
                po.created_by,
                "BARANG_TIBA",
                Some(po_id),
                &format!(
                    "Barang untuk PO {} telah diterima ({})",
                    po.no_po, no_terima
                ),
            )
            .await?;
        }

        PenerimaanRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn list_by_po(state: &AppState, po_id: i64) -> AppResult<Vec<PenerimaanRow>> {
        PenerimaanRepository::list_by_po(&state.pool, po_id).await
    }

    pub async fn list(
        state: &AppState,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<PenerimaanRow>, i64)> {
        PenerimaanRepository::list(&state.pool, page, per_page).await
    }

    pub async fn detail_siap_terima(
        state: &AppState,
        po_id: i64,
    ) -> AppResult<Vec<serde_json::Value>> {
        PenerimaanRepository::detail_siap_terima(&state.pool, po_id).await
    }
}
