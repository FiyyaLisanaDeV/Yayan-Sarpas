//! Repository penawaran.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PenawaranRow {
    pub id: i64,
    pub supplier_id: i64,
    pub supplier_nama: String,
    pub item_id: i64,
    pub item_nama: String,
    pub kategori: String,
    pub satuan_kode: String,
    pub harga_satuan: BigDecimal,
    pub tanggal_penawaran: NaiveDate,
    pub sumber: Option<String>,
    pub status: String,
    pub alasan_penunjukan: Option<String>,
    pub catatan: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Baris perbandingan: satu item dengan daftar penawarannya.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ItemPenawaranRow {
    pub item_id: i64,
    pub item_nama: String,
    pub kategori: String,
    pub satuan_kode: String,
    pub jumlah_penawaran: i64,
    pub harga_terendah: Option<BigDecimal>,
    pub ada_terpilih: bool,
}

/// Data penawaran baru.
pub struct PenawaranBaru<'a> {
    pub supplier_id: i64,
    pub item_id: i64,
    pub harga_satuan: &'a BigDecimal,
    pub tanggal: NaiveDate,
    pub sumber: Option<&'a str>,
    pub catatan: Option<&'a str>,
    pub created_by: i64,
}

pub struct PenawaranRepository;

impl PenawaranRepository {
    pub async fn create(pool: &PgPool, data: PenawaranBaru<'_>) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO penawaran
                 (supplier_id, item_id, harga_satuan, tanggal_penawaran, sumber, catatan, created_by)
               VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id"#,
            data.supplier_id,
            data.item_id,
            data.harga_satuan,
            data.tanggal,
            data.sumber,
            data.catatan,
            data.created_by
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn update(
        pool: &PgPool,
        id: i64,
        harga_satuan: &BigDecimal,
        tanggal: NaiveDate,
        sumber: Option<&str>,
        catatan: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE penawaran SET harga_satuan=$2, tanggal_penawaran=$3, sumber=$4, catatan=$5 WHERE id=$1",
            id,
            harga_satuan,
            tanggal,
            sumber,
            catatan
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<PenawaranRow>> {
        Ok(sqlx::query_as::<_, PenawaranRow>(
            r#"SELECT p.id, p.supplier_id, s.nama AS supplier_nama,
                      p.item_id, i.nama AS item_nama, k.kode AS kategori, sa.kode AS satuan_kode,
                      p.harga_satuan, p.tanggal_penawaran, p.sumber, p.status,
                      p.alasan_penunjukan, p.catatan, p.created_at
               FROM penawaran p
               JOIN supplier s ON s.id = p.supplier_id
               JOIN items i ON i.id = p.item_id
               JOIN satuan sa ON sa.id = i.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE p.id = $1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?)
    }

    pub async fn list_by_item(pool: &PgPool, item_id: i64) -> AppResult<Vec<PenawaranRow>> {
        Ok(sqlx::query_as::<_, PenawaranRow>(
            r#"SELECT p.id, p.supplier_id, s.nama AS supplier_nama,
                      p.item_id, i.nama AS item_nama, k.kode AS kategori, sa.kode AS satuan_kode,
                      p.harga_satuan, p.tanggal_penawaran, p.sumber, p.status,
                      p.alasan_penunjukan, p.catatan, p.created_at
               FROM penawaran p
               JOIN supplier s ON s.id = p.supplier_id
               JOIN items i ON i.id = p.item_id
               JOIN satuan sa ON sa.id = i.satuan_id
               JOIN kategori k ON k.id = i.kategori_id
               WHERE p.item_id = $1
               ORDER BY p.harga_satuan, p.tanggal_penawaran DESC"#,
        )
        .bind(item_id)
        .fetch_all(pool)
        .await?)
    }

    /// Daftar item yang punya penawaran, untuk halaman perbandingan.
    pub async fn list_items(
        pool: &PgPool,
        search: Option<&str>,
        only_dipilih: bool,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<ItemPenawaranRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as::<_, ItemPenawaranRow>(
            r#"SELECT i.id AS item_id, i.nama AS item_nama, k.kode AS kategori,
                      sa.kode AS satuan_kode,
                      COUNT(p.id) AS jumlah_penawaran,
                      MIN(p.harga_satuan) AS harga_terendah,
                      BOOL_OR(p.status = 'DIPILIH') AS ada_terpilih
               FROM penawaran p
               JOIN items i ON i.id = p.item_id
               JOIN kategori k ON k.id = i.kategori_id
               JOIN satuan sa ON sa.id = i.satuan_id
               WHERE ($1::text IS NULL OR lower(i.nama) LIKE $1)
               GROUP BY i.id, i.nama, k.kode, sa.kode
               HAVING (NOT $2::bool OR BOOL_OR(p.status = 'DIPILIH'))
               ORDER BY i.nama
               LIMIT $3 OFFSET $4"#,
        )
        .bind(pattern.clone())
        .bind(only_dipilih)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM (
                   SELECT i.id
                   FROM penawaran p JOIN items i ON i.id = p.item_id
                   WHERE ($1::text IS NULL OR lower(i.nama) LIKE $1)
                   GROUP BY i.id
                   HAVING (NOT $2::bool OR BOOL_OR(p.status = 'DIPILIH'))
               ) t"#,
        )
        .bind(pattern)
        .bind(only_dipilih)
        .fetch_one(pool)
        .await?;

        Ok((rows, total))
    }

    /// Tandai satu penawaran DIPILIH, sisanya TIDAK_DIPILIH untuk item yang sama.
    pub async fn pilih(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        penawaran_id: i64,
        item_id: i64,
        alasan: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE penawaran SET status = 'TIDAK_DIPILIH', alasan_penunjukan = NULL
             WHERE item_id = $1 AND id <> $2",
            item_id,
            penawaran_id
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            "UPDATE penawaran SET status = 'DIPILIH', alasan_penunjukan = $2 WHERE id = $1",
            penawaran_id,
            alasan
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &PgPool, id: i64) -> AppResult<u64> {
        Ok(sqlx::query!("DELETE FROM penawaran WHERE id = $1", id)
            .execute(pool)
            .await?
            .rows_affected())
    }

    /// Penawaran DIPILIH untuk satu item (dipakai saat membuat PO).
    pub async fn terpilih(pool: &PgPool, item_id: i64) -> AppResult<Option<(i64, BigDecimal)>> {
        Ok(sqlx::query_as::<_, (i64, BigDecimal)>(
            "SELECT supplier_id, harga_satuan FROM penawaran WHERE item_id = $1 AND status = 'DIPILIH' LIMIT 1",
        )
        .bind(item_id)
        .fetch_optional(pool)
        .await?)
    }
}
