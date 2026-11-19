//! Service PO — grouping supplier, PPN, prasyarat penawaran, approval threshold.

use std::collections::HashMap;

use bigdecimal::{BigDecimal, Zero};
use chrono::{NaiveDate, Utc};
use serde_json::json;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};
use sarpras_shared::rbac::Role;

use crate::domains::approval::service::hitung_hari_kerja;
use crate::domains::penawaran::service::PenawaranService;
use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{PoDetailRow, PoHeaderRow, PoRepository};

pub struct PoService;

/// Satu baris input pembuatan PO.
#[derive(Debug, Clone)]
pub struct DetailInput {
    pub permintaan_id: i64,
    pub item_id: i64,
    pub qty: BigDecimal,
    pub harga_satuan: BigDecimal,
    pub satuan_id: i64,
}

impl PoService {
    pub async fn list(
        state: &AppState,
        page: i64,
        per_page: i64,
        tahun: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> AppResult<(Vec<PoHeaderRow>, i64)> {
        PoRepository::list(&state.pool, page, per_page, tahun, status, search).await
    }

    /// Buat PO dari baris permintaan FINAL.
    ///
    /// Aturan:
    /// - Semua baris harus milik **satu supplier** (grouping F03 = SUPPLIER).
    /// - Tiap item wajib punya penawaran DIPILIH (prasyarat F12).
    /// - Harga diambil dari penawaran terpilih bila tidak diisi.
    pub async fn create(
        state: &AppState,
        actor_id: i64,
        supplier_id: i64,
        tanggal_po: NaiveDate,
        catatan: Option<String>,
        input: Vec<DetailInput>,
        tahun: i16,
    ) -> AppResult<PoHeaderRow> {
        if input.is_empty() {
            return Err(AppError::Validation(
                "PO harus memiliki minimal satu baris".into(),
            ));
        }

        let supplier_ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM supplier WHERE id = $1 AND is_active")
                .bind(supplier_id)
                .fetch_one(&state.pool)
                .await?;
        if supplier_ok == 0 {
            return Err(AppError::Validation(
                "Supplier tidak ditemukan atau nonaktif".into(),
            ));
        }

        let ctx = ConfigContext::global();
        let ppn_persen = state.config.get_f64("F01", &ctx).await.unwrap_or(11.0);
        let format_no = state
            .config
            .get_string("F11", &ctx)
            .await
            .unwrap_or_else(|_| "PO/{kategori}/{tahun}/{seq:04}".to_string());
        let sla_hari = state.config.get_i64("C01", &ctx).await.unwrap_or(5);

        // Validasi tiap baris: permintaan FINAL, item cocok, penawaran dipilih.
        let mut kategori_pertama: Option<String> = None;
        let mut subtotal_total = BigDecimal::zero();
        let mut baris_siap: Vec<(DetailInput, BigDecimal, BigDecimal)> = Vec::new();

        for d in input {
            let row = sqlx::query!(
                r#"SELECT p.status, p.item_id, p.tahun
                   FROM permintaan p WHERE p.id = $1"#,
                d.permintaan_id
            )
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Permintaan #{}", d.permintaan_id)))?;

            if row.status != "FINAL" {
                return Err(AppError::Conflict(format!(
                    "Permintaan #{} berstatus {} — hanya FINAL yang dapat di-PO",
                    d.permintaan_id, row.status
                )));
            }
            if row.item_id != d.item_id {
                return Err(AppError::Validation(format!(
                    "Item pada permintaan #{} tidak cocok",
                    d.permintaan_id
                )));
            }

            let kategori = sqlx::query_scalar!(
                "SELECT k.kode FROM items i JOIN kategori k ON k.id = i.kategori_id WHERE i.id = $1",
                d.item_id
            )
            .fetch_one(&state.pool)
            .await?;
            if kategori_pertama.is_none() {
                kategori_pertama = Some(kategori.clone());
            }

            // Prasyarat penawaran (F12) + supplier harus supplier terpilih.
            let (jumlah_penawaran, min_penawaran, supplier_terpilih) =
                PenawaranService::cek_prasyarat_po(state, d.item_id).await?;
            if jumlah_penawaran < min_penawaran {
                return Err(AppError::Validation(format!(
                    "Item #{} baru punya {} penawaran (minimal {} sesuai config F12)",
                    d.item_id, jumlah_penawaran, min_penawaran
                )));
            }
            match supplier_terpilih {
                None => {
                    return Err(AppError::Validation(format!(
                        "Item #{} belum ditunjuk supplier pemenangnya",
                        d.item_id
                    )))
                }
                Some(s) if s != supplier_id => {
                    return Err(AppError::Conflict(format!(
                        "Item #{} ditunjuk ke supplier lain (id {s}), bukan {supplier_id}",
                        d.item_id
                    )))
                }
                Some(_) => {}
            }

            if d.qty.is_zero() || d.qty.sign() == bigdecimal::num_bigint::Sign::Minus {
                return Err(AppError::Validation(format!(
                    "Qty item #{} harus lebih dari 0",
                    d.item_id
                )));
            }

            let subtotal = &d.qty * &d.harga_satuan;
            subtotal_total += subtotal.clone();
            baris_siap.push((d, subtotal, BigDecimal::zero()));
        }

        // PPN: persen dari subtotal (F01, mis. 11.00 → 11%).
        let ppn_nilai = {
            use bigdecimal::FromPrimitive;
            use std::str::FromStr;
            let persen = BigDecimal::from_f64(ppn_persen).unwrap_or_else(|| BigDecimal::from(0));
            // subtotal × persen / 100 — hindari notasi ilmiah pada f64 besar.
            let seratus = BigDecimal::from(100);
            let hasil = (&subtotal_total * &persen) / &seratus;
            BigDecimal::from_str(&format!(
                "{:.2}",
                hasil.to_string().parse::<f64>().unwrap_or(0.0)
            ))
            .unwrap_or(hasil)
        };
        let nilai_total = &subtotal_total + &ppn_nilai;

        let kategori = kategori_pertama.unwrap_or_else(|| "ALKES".to_string());
        let sla_deadline = Some(
            hitung_hari_kerja(&state.pool, Utc::now(), sla_hari)
                .await
                .unwrap_or_else(|_| Utc::now()),
        );

        let mut tx = state.pool.begin().await?;
        let no_po = PoRepository::next_no_po(&mut tx, &format_no, &kategori, tahun).await?;
        let po_id = PoRepository::insert_header(
            &mut tx,
            &no_po,
            tahun,
            supplier_id,
            tanggal_po,
            &nilai_total,
            &ppn_nilai,
            actor_id,
            catatan.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            sla_deadline,
        )
        .await?;

        for (d, subtotal, _) in &baris_siap {
            PoRepository::insert_detail(
                &mut tx,
                po_id,
                Some(d.permintaan_id),
                d.item_id,
                &d.qty,
                &d.harga_satuan,
                subtotal,
                d.satuan_id,
            )
            .await?;
            PoRepository::tandai_permintaan_in_po(&mut tx, d.permintaan_id).await?;
        }
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "po_header",
            po_id,
            "CREATE",
            None,
            Some(&json!({
                "no_po": no_po, "supplier_id": supplier_id,
                "nilai_total": nilai_total.to_string(), "jumlah_baris": baris_siap.len(),
            })),
        )
        .await?;

        // Snapshot config saat buat PO (§8): F01, F03, F11.
        let mut snap_ctx = ConfigContext::global();
        snap_ctx.ref_type = Some("po_header".to_string());
        snap_ctx.ref_id = Some(po_id);
        let _ = state
            .config
            .snapshot("po_header", po_id, &["F01", "F03", "F11"], &snap_ctx)
            .await;

        PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Setujui PO. Bila nilai ≥ F02, hanya KABAG/ADMIN/WADIR yang boleh.
    pub async fn approve(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        po_id: i64,
    ) -> AppResult<PoHeaderRow> {
        let po = PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or_else(|| AppError::NotFound("PO".into()))?;

        if po.status != "DRAFT" {
            return Err(AppError::Conflict(format!(
                "PO berstatus {} — hanya DRAFT yang dapat disetujui",
                po.status
            )));
        }

        let threshold = state
            .config
            .get_f64("F02", &ConfigContext::global())
            .await
            .unwrap_or(0.0);
        let nilai: f64 = po.nilai_total.to_string().parse().unwrap_or(0.0);

        if nilai >= threshold
            && !matches!(
                actor.role,
                Role::KabagKeuangan | Role::Admin | Role::WadirYanmed
            )
        {
            return Err(AppError::Forbidden);
        }

        let mut tx = state.pool.begin().await?;
        PoRepository::set_approved(&mut tx, po_id, actor.id).await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor.id,
            "po_header",
            po_id,
            "APPROVE",
            Some(&json!({"status": "DRAFT"})),
            Some(&json!({"status": "APPROVED", "nilai": po.nilai_total.to_string()})),
        )
        .await?;

        crate::notifikasi::kirim_ke_user(
            state,
            po.created_by,
            "PO_APPROVED",
            Some(po_id),
            &format!(
                "PO {} senilai Rp {} telah disetujui",
                po.no_po, po.nilai_total
            ),
        )
        .await?;

        PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Tolak PO → kembali DRAFT.
    pub async fn reject(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        po_id: i64,
        catatan: Option<&str>,
    ) -> AppResult<PoHeaderRow> {
        let po = PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or_else(|| AppError::NotFound("PO".into()))?;
        if po.status != "DRAFT" {
            return Err(AppError::Conflict(format!(
                "PO berstatus {} — hanya DRAFT yang dapat ditolak",
                po.status
            )));
        }

        sqlx::query!(
            "UPDATE po_header SET status='DRAFT', catatan = COALESCE($2, catatan) WHERE id=$1",
            po_id,
            catatan
        )
        .execute(&state.pool)
        .await?;

        record_audit(
            &state.pool,
            actor.id,
            "po_header",
            po_id,
            "REJECT",
            None,
            Some(&json!({"catatan": catatan})),
        )
        .await?;

        PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Kirim PO ke supplier (APPROVED → SENT).
    pub async fn kirim(state: &AppState, actor_id: i64, po_id: i64) -> AppResult<PoHeaderRow> {
        let po = PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or_else(|| AppError::NotFound("PO".into()))?;
        if po.status != "APPROVED" {
            return Err(AppError::Conflict(
                "Hanya PO APPROVED yang dapat dikirim".into(),
            ));
        }

        let mut tx = state.pool.begin().await?;
        PoRepository::set_status(&mut tx, po_id, "SENT").await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "po_header",
            po_id,
            "SEND",
            None,
            Some(&json!({"status": "SENT"})),
        )
        .await?;

        PoRepository::find(&state.pool, po_id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn details(state: &AppState, po_id: i64) -> AppResult<Vec<PoDetailRow>> {
        PoRepository::details(&state.pool, po_id).await
    }

    /// Rekap realisasi: total per supplier & status.
    pub async fn realisasi(state: &AppState, tahun: i16) -> AppResult<serde_json::Value> {
        let per_status = sqlx::query!(
            "SELECT status, COUNT(*) AS jumlah, COALESCE(SUM(nilai_total),0) AS nilai
             FROM po_header WHERE tahun = $1 GROUP BY status ORDER BY status",
            tahun
        )
        .fetch_all(&state.pool)
        .await?;

        let per_supplier = sqlx::query!(
            r#"SELECT s.nama, COUNT(p.id) AS jumlah, COALESCE(SUM(p.nilai_total),0) AS nilai
               FROM po_header p JOIN supplier s ON s.id = p.supplier_id
               WHERE p.tahun = $1
               GROUP BY s.nama ORDER BY SUM(p.nilai_total) DESC NULLS LAST LIMIT 20"#,
            tahun
        )
        .fetch_all(&state.pool)
        .await?;

        let menunggu_po: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM permintaan WHERE status='FINAL'")
                .fetch_one(&state.pool)
                .await?;

        Ok(json!({
            "tahun": tahun,
            "per_status": per_status.iter().map(|r| json!({
                "status": r.status, "jumlah": r.jumlah, "nilai": r.nilai
            })).collect::<Vec<_>>(),
            "per_supplier": per_supplier.iter().map(|r| json!({
                "supplier": r.nama, "jumlah_po": r.jumlah, "nilai": r.nilai
            })).collect::<Vec<_>>(),
            "permintaan_final_belum_po": menunggu_po,
        }))
    }
}

/// Kelompokkan baris input per supplier (untuk UI grouping F03).
pub fn kelompokkan_per_supplier(rows: Vec<(i64, DetailInput)>) -> HashMap<i64, Vec<DetailInput>> {
    let mut map: HashMap<i64, Vec<DetailInput>> = HashMap::new();
    for (supplier_id, d) in rows {
        map.entry(supplier_id).or_default().push(d);
    }
    map
}
