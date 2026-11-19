//! Repository item.

use serde::Serialize;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ItemRow {
    pub id: i64,
    pub nama: String,
    pub satuan_id: i64,
    pub satuan_kode: String,
    pub satuan_nama: String,
    pub kategori_id: i64,
    pub kategori_kode: String,
    pub kategori_nama: String,
    pub threshold_aset: Option<bigdecimal::BigDecimal>,
    pub is_active: bool,
}

pub struct ItemRepository;

impl ItemRepository {
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        search: Option<&str>,
        kategori_id: Option<i64>,
        include_inactive: bool,
    ) -> AppResult<(Vec<ItemRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as!(
            ItemRow,
            r#"SELECT i.id, i.nama, i.satuan_id, s.kode AS satuan_kode, s.nama AS satuan_nama,
                      i.kategori_id, k.kode AS kategori_kode, k.nama AS kategori_nama,
                      i.threshold_aset, i.is_active
               FROM items i
               JOIN satuan s ON s.id = i.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE ($3::text IS NULL OR lower(i.nama) LIKE $3)
                 AND ($4::bigint IS NULL OR i.kategori_id = $4)
                 AND ($5::bool OR i.is_active)
               ORDER BY i.nama
               LIMIT $1 OFFSET $2"#,
            per_page,
            offset,
            pattern,
            kategori_id,
            include_inactive
        )
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM items i
               WHERE ($1::text IS NULL OR lower(i.nama) LIKE $1)
                 AND ($2::bigint IS NULL OR i.kategori_id = $2)
                 AND ($3::bool OR i.is_active)"#,
            pattern,
            kategori_id,
            include_inactive
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<ItemRow>> {
        Ok(sqlx::query_as!(
            ItemRow,
            r#"SELECT i.id, i.nama, i.satuan_id, s.kode AS satuan_kode, s.nama AS satuan_nama,
                      i.kategori_id, k.kode AS kategori_kode, k.nama AS kategori_nama,
                      i.threshold_aset, i.is_active
               FROM items i
               JOIN satuan s ON s.id = i.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE i.id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await?)
    }

    pub async fn create(
        pool: &PgPool,
        nama: &str,
        satuan_id: i64,
        kategori_id: i64,
        threshold_aset: Option<bigdecimal::BigDecimal>,
    ) -> AppResult<i64> {
        let rec = sqlx::query!(
            "INSERT INTO items (nama, satuan_id, kategori_id, threshold_aset) VALUES ($1, $2, $3, $4) RETURNING id",
            nama,
            satuan_id,
            kategori_id,
            threshold_aset
        )
        .fetch_one(pool)
        .await?;
        Ok(rec.id)
    }

    pub async fn update(
        pool: &PgPool,
        id: i64,
        nama: &str,
        satuan_id: i64,
        kategori_id: i64,
        threshold_aset: Option<bigdecimal::BigDecimal>,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE items SET nama = $2, satuan_id = $3, kategori_id = $4, threshold_aset = $5 WHERE id = $1",
            id,
            nama,
            satuan_id,
            kategori_id,
            threshold_aset
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(pool: &PgPool, id: i64) -> AppResult<()> {
        sqlx::query!("UPDATE items SET is_active = FALSE WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
