//! Repository supplier — master + katalog supplier-item.

use bigdecimal::BigDecimal;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct SupplierRow {
    pub id: i64,
    pub nama: String,
    pub npwp: Option<String>,
    pub kontak: Option<String>,
    pub alamat: Option<String>,
    pub is_active: bool,
    pub jumlah_item: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct SupplierItemRow {
    pub id: i64,
    pub supplier_id: i64,
    pub item_id: i64,
    pub item_nama: String,
    pub kategori: String,
    pub satuan_kode: String,
    pub harga_referensi: Option<BigDecimal>,
    pub catatan: Option<String>,
    pub is_active: bool,
}

pub struct SupplierRepository;

impl SupplierRepository {
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        search: Option<&str>,
        include_inactive: bool,
    ) -> AppResult<(Vec<SupplierRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as!(
            SupplierRow,
            r#"SELECT s.id, s.nama, s.npwp, s.kontak, s.alamat, s.is_active,
                      (SELECT COUNT(*) FROM supplier_item si
                       WHERE si.supplier_id = s.id AND si.is_active) AS jumlah_item
               FROM supplier s
               WHERE ($1::text IS NULL OR lower(s.nama) LIKE $1 OR lower(COALESCE(s.npwp,'')) LIKE $1)
                 AND ($4::bool OR s.is_active)
               ORDER BY s.nama
               LIMIT $2 OFFSET $3"#,
            pattern,
            per_page,
            offset,
            include_inactive
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM supplier s
               WHERE ($1::text IS NULL OR lower(s.nama) LIKE $1 OR lower(COALESCE(s.npwp,'')) LIKE $1)
                 AND ($2::bool OR s.is_active)"#,
        )
        .bind(pattern)
        .bind(include_inactive)
        .fetch_one(pool)
        .await?;

        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<SupplierRow>> {
        Ok(sqlx::query_as!(
            SupplierRow,
            r#"SELECT s.id, s.nama, s.npwp, s.kontak, s.alamat, s.is_active,
                      (SELECT COUNT(*) FROM supplier_item si
                       WHERE si.supplier_id = s.id AND si.is_active) AS jumlah_item
               FROM supplier s WHERE s.id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await?)
    }

    pub async fn create(
        pool: &PgPool,
        nama: &str,
        npwp: Option<&str>,
        kontak: Option<&str>,
        alamat: Option<&str>,
    ) -> AppResult<i64> {
        let rec = sqlx::query_scalar!(
            "INSERT INTO supplier (nama, npwp, kontak, alamat) VALUES ($1,$2,$3,$4) RETURNING id",
            nama,
            npwp,
            kontak,
            alamat
        )
        .fetch_one(pool)
        .await?;
        Ok(rec)
    }

    pub async fn update(
        pool: &PgPool,
        id: i64,
        nama: &str,
        npwp: Option<&str>,
        kontak: Option<&str>,
        alamat: Option<&str>,
        is_active: bool,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE supplier SET nama=$2, npwp=$3, kontak=$4, alamat=$5, is_active=$6 WHERE id=$1",
            id,
            nama,
            npwp,
            kontak,
            alamat,
            is_active
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(pool: &PgPool, id: i64) -> AppResult<()> {
        sqlx::query!("UPDATE supplier SET is_active = FALSE WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // ---------------------------------------------------------- katalog

    pub async fn list_items(pool: &PgPool, supplier_id: i64) -> AppResult<Vec<SupplierItemRow>> {
        Ok(sqlx::query_as!(
            SupplierItemRow,
            r#"SELECT si.id, si.supplier_id, si.item_id,
                      i.nama AS item_nama, k.kode AS kategori, s.kode AS satuan_kode,
                      si.harga_referensi, si.catatan, si.is_active
               FROM supplier_item si
               JOIN items i ON i.id = si.item_id
               JOIN satuan s ON s.id = i.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE si.supplier_id = $1
               ORDER BY i.nama"#,
            supplier_id
        )
        .fetch_all(pool)
        .await?)
    }

    pub async fn add_item(
        pool: &PgPool,
        supplier_id: i64,
        item_id: i64,
        harga_referensi: Option<&BigDecimal>,
        catatan: Option<&str>,
    ) -> AppResult<i64> {
        let rec = sqlx::query_scalar!(
            r#"INSERT INTO supplier_item (supplier_id, item_id, harga_referensi, catatan)
               VALUES ($1,$2,$3,$4)
               ON CONFLICT (supplier_id, item_id)
               DO UPDATE SET harga_referensi = EXCLUDED.harga_referensi,
                             catatan = EXCLUDED.catatan,
                             is_active = TRUE
               RETURNING id"#,
            supplier_id,
            item_id,
            harga_referensi,
            catatan
        )
        .fetch_one(pool)
        .await?;
        Ok(rec)
    }

    pub async fn remove_item(pool: &PgPool, id: i64) -> AppResult<u64> {
        let n = sqlx::query!("DELETE FROM supplier_item WHERE id = $1", id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(n)
    }

    /// Supplier aktif yang menyediakan sebuah item (untuk dropdown penawaran).
    pub async fn supplier_untuk_item(pool: &PgPool, item_id: i64) -> AppResult<Vec<(i64, String)>> {
        Ok(sqlx::query_as::<_, (i64, String)>(
            r#"SELECT s.id, s.nama
               FROM supplier_item si JOIN supplier s ON s.id = si.supplier_id
               WHERE si.item_id = $1 AND si.is_active AND s.is_active
               ORDER BY s.nama"#,
        )
        .bind(item_id)
        .fetch_all(pool)
        .await?)
    }
}
