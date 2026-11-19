//! Repository penerimaan.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PenerimaanRow {
    pub id: i64,
    pub po_detail_id: i64,
    pub no_terima: String,
    pub tanggal_terima: NaiveDate,
    pub qty_terima: BigDecimal,
    pub qty_reject: BigDecimal,
    pub kondisi: Option<String>,
    pub no_do: Option<String>,
    pub no_invoice: Option<String>,
    pub no_faktur_pajak: Option<String>,
    pub diterima_oleh: i64,
    pub catatan: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub no_po: String,
    pub item_nama: String,
    pub qty_po: BigDecimal,
}

pub struct PenerimaanRepository;

impl PenerimaanRepository {
    const COLS: &'static str = r#"
        SELECT pn.id, pn.po_detail_id, pn.no_terima, pn.tanggal_terima,
               pn.qty_terima, pn.qty_reject, pn.kondisi,
               pn.no_do, pn.no_invoice, pn.no_faktur_pajak,
               pn.diterima_oleh, pn.catatan, pn.updated_at,
               ph.no_po, i.nama AS item_nama, d.qty AS qty_po
        FROM penerimaan pn
        JOIN po_detail d ON d.id = pn.po_detail_id
        JOIN po_header ph ON ph.id = d.po_header_id
        JOIN items i ON i.id = d.item_id
    "#;

    pub async fn list_by_po(pool: &PgPool, po_id: i64) -> AppResult<Vec<PenerimaanRow>> {
        Ok(sqlx::query_as::<_, PenerimaanRow>(&format!(
            "{} WHERE ph.id = $1 ORDER BY pn.id",
            Self::COLS
        ))
        .bind(po_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<PenerimaanRow>, i64)> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query_as::<_, PenerimaanRow>(&format!(
            "{} ORDER BY pn.id DESC LIMIT $1 OFFSET $2",
            Self::COLS
        ))
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM penerimaan")
            .fetch_one(pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<PenerimaanRow>> {
        Ok(
            sqlx::query_as::<_, PenerimaanRow>(&format!("{} WHERE pn.id = $1", Self::COLS))
                .bind(id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Detail PO yang bisa diterima (beserta sisa).
    pub async fn detail_siap_terima(
        pool: &PgPool,
        po_id: i64,
    ) -> AppResult<Vec<serde_json::Value>> {
        let rows = sqlx::query!(
            r#"SELECT d.id, d.item_id, i.nama AS item_nama, s.kode AS satuan,
                      d.qty, d.harga_satuan,
                      COALESCE((SELECT SUM(pn.qty_terima) FROM penerimaan pn WHERE pn.po_detail_id = d.id),0) AS "sudah_terima!"
               FROM po_detail d
               JOIN items i ON i.id = d.item_id
               JOIN satuan s ON s.id = d.satuan_id
               WHERE d.po_header_id = $1
               ORDER BY d.id"#,
            po_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let sisa: BigDecimal = &r.qty - &r.sudah_terima;
                serde_json::json!({
                    "po_detail_id": r.id, "item_id": r.item_id, "item": r.item_nama,
                    "satuan": r.satuan, "qty_po": r.qty, "sudah_terima": r.sudah_terima,
                    "sisa": sisa, "harga_satuan": r.harga_satuan,
                })
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        po_detail_id: i64,
        no_terima: &str,
        tanggal: NaiveDate,
        qty_terima: &BigDecimal,
        qty_reject: &BigDecimal,
        kondisi: Option<&str>,
        no_do: Option<&str>,
        no_invoice: Option<&str>,
        no_faktur: Option<&str>,
        diterima_oleh: i64,
        catatan: Option<&str>,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO penerimaan
                 (po_detail_id, no_terima, tanggal_terima, qty_terima, qty_reject, kondisi,
                  no_do, no_invoice, no_faktur_pajak, diterima_oleh, catatan)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id"#,
            po_detail_id,
            no_terima,
            tanggal,
            qty_terima,
            qty_reject,
            kondisi,
            no_do,
            no_invoice,
            no_faktur,
            diterima_oleh,
            catatan
        )
        .fetch_one(&mut **tx)
        .await?)
    }

    /// Nomor penerimaan berikutnya dari format J05.
    pub async fn next_no_terima(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        format: &str,
        tahun: i16,
    ) -> AppResult<String> {
        let seq: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) + 1 FROM penerimaan pn
             JOIN po_detail d ON d.id = pn.po_detail_id
             JOIN po_header ph ON ph.id = d.po_header_id
             WHERE ph.tahun = $1",
        )
        .bind(tahun)
        .fetch_one(&mut **tx)
        .await?;

        Ok(format
            .replace("{tahun}", &tahun.to_string())
            .replace("{seq:04}", &format!("{seq:04}"))
            .replace("{seq}", &seq.to_string()))
    }

    /// Total diterima vs qty PO untuk satu detail.
    pub async fn sisa_detail(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        po_detail_id: i64,
    ) -> AppResult<(BigDecimal, BigDecimal)> {
        let row = sqlx::query!(
            r#"SELECT d.qty,
                      COALESCE((SELECT SUM(pn.qty_terima) FROM penerimaan pn
                                WHERE pn.po_detail_id = d.id),0) AS "sudah!"
               FROM po_detail d WHERE d.id = $1"#,
            po_detail_id
        )
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| sarpras_shared::error::AppError::NotFound("Detail PO".into()))?;

        let sisa = &row.qty - &row.sudah;
        Ok((row.qty, sisa))
    }

    pub async fn po_id_dari_detail(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        po_detail_id: i64,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            "SELECT po_header_id FROM po_detail WHERE id = $1",
            po_detail_id
        )
        .fetch_one(&mut **tx)
        .await?)
    }
}
