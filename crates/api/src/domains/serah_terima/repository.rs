//! Repository serah terima.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SerahRow {
    pub id: i64,
    pub penerimaan_id: i64,
    pub ruangan_id: i64,
    pub ruangan_nama: String,
    pub tanggal_serah: NaiveDate,
    pub qty_serah: BigDecimal,
    pub penerima_ruangan_id: i64,
    pub penerima_nama: String,
    pub pemberi_id: i64,
    pub pemberi_nama: String,
    pub no_ba: Option<String>,
    pub file_ba: Option<String>,
    pub catatan: Option<String>,
    pub status_konfirmasi: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub item_nama: String,
    pub no_po: String,
    pub no_terima: String,
}

pub struct SerahRepository;

impl SerahRepository {
    const COLS: &'static str = r#"
        SELECT st.id, st.penerimaan_id, st.ruangan_id, r.nama AS ruangan_nama,
               st.tanggal_serah, st.qty_serah,
               st.penerima_ruangan_id, ur.nama AS penerima_nama,
               st.pemberi_id, up.nama AS pemberi_nama,
               st.no_ba, st.file_ba, st.catatan, st.status_konfirmasi, st.updated_at,
               i.nama AS item_nama, ph.no_po, pn.no_terima
        FROM serah_terima st
        JOIN ruangan r ON r.id = st.ruangan_id
        JOIN users ur ON ur.id = st.penerima_ruangan_id
        JOIN users up ON up.id = st.pemberi_id
        JOIN penerimaan pn ON pn.id = st.penerimaan_id
        JOIN po_detail d ON d.id = pn.po_detail_id
        JOIN po_header ph ON ph.id = d.po_header_id
        JOIN items i ON i.id = d.item_id
    "#;

    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        status: Option<&str>,
        ruangan_id: Option<i64>,
    ) -> AppResult<(Vec<SerahRow>, i64)> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query_as::<_, SerahRow>(&format!(
            r#"{}
               WHERE ($1::text IS NULL OR st.status_konfirmasi = $1)
                 AND ($2::bigint IS NULL OR st.ruangan_id = $2)
               ORDER BY st.id DESC LIMIT $3 OFFSET $4"#,
            Self::COLS
        ))
        .bind(status)
        .bind(ruangan_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM serah_terima st
             WHERE ($1::text IS NULL OR st.status_konfirmasi = $1)
               AND ($2::bigint IS NULL OR st.ruangan_id = $2)",
        )
        .bind(status)
        .bind(ruangan_id)
        .fetch_one(pool)
        .await?;
        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<SerahRow>> {
        Ok(
            sqlx::query_as::<_, SerahRow>(&format!("{} WHERE st.id = $1", Self::COLS))
                .bind(id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Penerimaan yang siap diserahkan (belum punya serah terima).
    pub async fn penerimaan_siap(pool: &PgPool) -> AppResult<Vec<serde_json::Value>> {
        let rows = sqlx::query!(
            r#"SELECT pn.id, pn.no_terima, pn.qty_terima, pn.tanggal_terima,
                      i.nama AS item_nama, d.item_id, d.po_header_id, ph.no_po,
                      ph.supplier_id, d.harga_satuan
               FROM penerimaan pn
               JOIN po_detail d ON d.id = pn.po_detail_id
               JOIN po_header ph ON ph.id = d.po_header_id
               JOIN items i ON i.id = d.item_id
               WHERE NOT EXISTS (SELECT 1 FROM serah_terima st WHERE st.penerimaan_id = pn.id)
               ORDER BY pn.id"#
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "penerimaan_id": r.id, "no_terima": r.no_terima,
                    "item": r.item_nama, "item_id": r.item_id,
                    "qty": r.qty_terima, "tanggal_terima": r.tanggal_terima,
                    "no_po": r.no_po, "harga_satuan": r.harga_satuan,
                    "nilai": &r.qty_terima * &r.harga_satuan,
                })
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        penerimaan_id: i64,
        ruangan_id: i64,
        tanggal: NaiveDate,
        qty: &BigDecimal,
        penerima_ruangan_id: i64,
        pemberi_id: i64,
        no_ba: &str,
        catatan: Option<&str>,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO serah_terima
                 (penerimaan_id, ruangan_id, tanggal_serah, qty_serah,
                  penerima_ruangan_id, pemberi_id, no_ba, catatan)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id"#,
            penerimaan_id,
            ruangan_id,
            tanggal,
            qty,
            penerima_ruangan_id,
            pemberi_id,
            no_ba,
            catatan
        )
        .fetch_one(&mut **tx)
        .await?)
    }

    pub async fn next_no_ba(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        format: &str,
        tahun: i16,
    ) -> AppResult<String> {
        let seq: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) + 1 FROM serah_terima st
             JOIN penerimaan pn ON pn.id = st.penerimaan_id
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

    pub async fn set_status(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        status: &str,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE serah_terima SET status_konfirmasi = $2 WHERE id = $1",
            id,
            status
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn set_file_ba(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        path: &str,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE serah_terima SET file_ba = $2 WHERE id = $1",
            id,
            path
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
