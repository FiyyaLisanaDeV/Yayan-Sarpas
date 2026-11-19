//! Repository PO.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PoHeaderRow {
    pub id: i64,
    pub no_po: String,
    pub tahun: i16,
    pub supplier_id: i64,
    pub supplier_nama: String,
    pub tanggal_po: NaiveDate,
    pub nilai_total: BigDecimal,
    pub ppn: BigDecimal,
    pub status: String,
    pub created_by: i64,
    pub approved_by: Option<i64>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub catatan: Option<String>,
    pub sla_deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub sla_status: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PoDetailRow {
    pub id: i64,
    pub po_header_id: i64,
    pub permintaan_id: Option<i64>,
    pub item_id: i64,
    pub item_nama: String,
    pub kategori: String,
    pub qty: BigDecimal,
    pub harga_satuan: BigDecimal,
    pub subtotal: BigDecimal,
    pub satuan_kode: String,
    pub qty_diterima: BigDecimal,
}

pub struct PoRepository;

const HEADER_COLS: &str = r#"
    SELECT p.id, p.no_po, p.tahun, p.supplier_id, s.nama AS supplier_nama,
           p.tanggal_po, p.nilai_total, p.ppn, p.status, p.created_by,
           p.approved_by, p.approved_at, p.catatan,
           p.sla_deadline, p.sla_status, p.updated_at
    FROM po_header p JOIN supplier s ON s.id = p.supplier_id
"#;

impl PoRepository {
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        tahun: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> AppResult<(Vec<PoHeaderRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as::<_, PoHeaderRow>(&format!(
            r#"{HEADER_COLS}
               WHERE ($1::smallint IS NULL OR p.tahun = $1)
                 AND ($2::text IS NULL OR p.status = $2)
                 AND ($3::text IS NULL OR lower(p.no_po) LIKE $3 OR lower(s.nama) LIKE $3)
               ORDER BY p.updated_at DESC, p.id DESC
               LIMIT $4 OFFSET $5"#
        ))
        .bind(tahun)
        .bind(status)
        .bind(pattern.clone())
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM po_header p JOIN supplier s ON s.id = p.supplier_id
               WHERE ($1::smallint IS NULL OR p.tahun = $1)
                 AND ($2::text IS NULL OR p.status = $2)
                 AND ($3::text IS NULL OR lower(p.no_po) LIKE $3 OR lower(s.nama) LIKE $3)"#,
        )
        .bind(tahun)
        .bind(status)
        .bind(pattern)
        .fetch_one(pool)
        .await?;

        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<PoHeaderRow>> {
        Ok(
            sqlx::query_as::<_, PoHeaderRow>(&format!("{HEADER_COLS} WHERE p.id = $1"))
                .bind(id)
                .fetch_optional(pool)
                .await?,
        )
    }

    pub async fn details(pool: &PgPool, po_id: i64) -> AppResult<Vec<PoDetailRow>> {
        Ok(sqlx::query_as::<_, PoDetailRow>(
            r#"SELECT d.id, d.po_header_id, d.permintaan_id, d.item_id,
                      i.nama AS item_nama, k.kode AS kategori,
                      d.qty, d.harga_satuan, d.subtotal, s.kode AS satuan_kode,
                      COALESCE((SELECT SUM(pn.qty_terima) FROM penerimaan pn
                                WHERE pn.po_detail_id = d.id), 0) AS qty_diterima
               FROM po_detail d
               JOIN items i ON i.id = d.item_id
               JOIN satuan s ON s.id = d.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE d.po_header_id = $1
               ORDER BY d.id"#,
        )
        .bind(po_id)
        .fetch_all(pool)
        .await?)
    }

    /// Nomor PO berikutnya: format dari F11, seq global per tahun.
    pub async fn next_no_po(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        format: &str,
        kategori: &str,
        tahun: i16,
    ) -> AppResult<String> {
        let seq: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) + 1 FROM po_header WHERE tahun = $1")
                .bind(tahun)
                .fetch_one(&mut **tx)
                .await?;

        Ok(format
            .replace("{kategori}", kategori)
            .replace("{tahun}", &tahun.to_string())
            .replace("{seq:04}", &format!("{seq:04}"))
            .replace("{seq}", &seq.to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_header(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        no_po: &str,
        tahun: i16,
        supplier_id: i64,
        tanggal_po: NaiveDate,
        nilai_total: &BigDecimal,
        ppn: &BigDecimal,
        created_by: i64,
        catatan: Option<&str>,
        sla_deadline: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO po_header
                 (no_po, tahun, supplier_id, tanggal_po, nilai_total, ppn, status,
                  created_by, catatan, sla_deadline)
               VALUES ($1,$2,$3,$4,$5,$6,'DRAFT',$7,$8,$9) RETURNING id"#,
            no_po,
            tahun,
            supplier_id,
            tanggal_po,
            nilai_total,
            ppn,
            created_by,
            catatan,
            sla_deadline
        )
        .fetch_one(&mut **tx)
        .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_detail(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        po_header_id: i64,
        permintaan_id: Option<i64>,
        item_id: i64,
        qty: &BigDecimal,
        harga_satuan: &BigDecimal,
        subtotal: &BigDecimal,
        satuan_id: i64,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"INSERT INTO po_detail (po_header_id, permintaan_id, item_id, qty, harga_satuan, subtotal, satuan_id)
               VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
            po_header_id,
            permintaan_id,
            item_id,
            qty,
            harga_satuan,
            subtotal,
            satuan_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn set_status(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        status: &str,
    ) -> AppResult<()> {
        sqlx::query!("UPDATE po_header SET status = $2 WHERE id = $1", id, status)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub async fn set_approved(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        approver: i64,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE po_header SET status='APPROVED', approved_by=$2, approved_at=now() WHERE id=$1",
            id,
            approver
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Tandai baris permintaan masuk PO (FINAL → IN_PO).
    pub async fn tandai_permintaan_in_po(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        permintaan_id: i64,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE permintaan SET status = 'IN_PO' WHERE id = $1 AND status = 'FINAL'",
            permintaan_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Baris permintaan FINAL yang belum masuk PO, dikelompokkan untuk dipilih.
    pub async fn permintaan_final(
        pool: &PgPool,
        search: Option<&str>,
    ) -> AppResult<Vec<serde_json::Value>> {
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));
        let rows = sqlx::query(
            r#"SELECT p.id, p.tahun, r.nama AS ruangan, i.id AS item_id, i.nama AS item,
                      k.kode AS kategori, p.jumlah, s.kode AS satuan, s.id AS satuan_id
               FROM permintaan p
               JOIN ruangan r ON r.id = p.ruangan_id
               JOIN items i ON i.id = p.item_id
               JOIN satuan s ON s.id = p.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE p.status = 'FINAL'
                 AND NOT EXISTS (SELECT 1 FROM po_detail d WHERE d.permintaan_id = p.id)
                 AND ($1::text IS NULL OR lower(i.nama) LIKE $1 OR lower(r.nama) LIKE $1)
               ORDER BY r.nama, i.nama"#,
        )
        .bind(pattern)
        .fetch_all(pool)
        .await?;

        use sqlx::Row;
        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "permintaan_id": r.get::<i64, _>("id"),
                    "tahun": r.get::<i16, _>("tahun"),
                    "ruangan": r.get::<String, _>("ruangan"),
                    "item_id": r.get::<i64, _>("item_id"),
                    "item": r.get::<String, _>("item"),
                    "kategori": r.get::<String, _>("kategori"),
                    "jumlah": r.get::<BigDecimal, _>("jumlah"),
                    "satuan": r.get::<String, _>("satuan"),
                    "satuan_id": r.get::<i64, _>("satuan_id"),
                })
            })
            .collect())
    }

    /// Apakah ada detail PO yang belum diterima penuh (untuk auto-close).
    pub async fn ada_sisa_belum_terima(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        po_id: i64,
    ) -> AppResult<bool> {
        let sisa: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM po_detail d
               WHERE d.po_header_id = $1
                 AND d.qty > COALESCE((SELECT SUM(pn.qty_terima) FROM penerimaan pn
                                       WHERE pn.po_detail_id = d.id), 0)"#,
        )
        .bind(po_id)
        .fetch_one(&mut **tx)
        .await?;
        Ok(sisa > 0)
    }
}
