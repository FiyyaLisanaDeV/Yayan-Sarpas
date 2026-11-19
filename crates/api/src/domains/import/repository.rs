//! Repository import — batch, row log, dan lookup master untuk pencocokan.

use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BatchRow {
    pub id: i64,
    pub tipe: String,
    pub file_path: String,
    pub total_row: i32,
    pub sukses: i32,
    pub gagal: i32,
    pub created_by: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RowLogRow {
    pub id: i64,
    pub batch_id: i64,
    pub row_no: i32,
    pub status: String,
    pub error_msg: Option<String>,
    pub raw_json: Option<serde_json::Value>,
}

pub struct ImportRepository;

impl ImportRepository {
    pub async fn create_batch(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tipe: &str,
        file_path: &str,
        total_row: i32,
        created_by: i64,
    ) -> AppResult<i64> {
        let rec = sqlx::query_scalar!(
            "INSERT INTO import_batch (tipe, file_path, total_row, created_by)
             VALUES ($1, $2, $3, $4) RETURNING id",
            tipe,
            file_path,
            total_row,
            created_by
        )
        .fetch_one(&mut **tx)
        .await?;
        Ok(rec)
    }

    pub async fn update_batch_counts(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        batch_id: i64,
        sukses: i32,
        gagal: i32,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE import_batch SET sukses = $2, gagal = $3 WHERE id = $1",
            batch_id,
            sukses,
            gagal
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn log_row(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        batch_id: i64,
        row_no: i32,
        status: &str,
        error_msg: Option<&str>,
        raw_json: Option<&serde_json::Value>,
    ) -> AppResult<()> {
        sqlx::query!(
            "INSERT INTO import_row_log (batch_id, row_no, status, error_msg, raw_json)
             VALUES ($1, $2, $3, $4, $5)",
            batch_id,
            row_no,
            status,
            error_msg,
            raw_json
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn list_batch(
        pool: &PgPool,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<BatchRow>, i64)> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query_as!(
            BatchRow,
            "SELECT id, tipe, file_path, total_row, sukses, gagal, created_by, created_at
             FROM import_batch ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            per_page,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_batch")
            .fetch_one(pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn find_batch(pool: &PgPool, id: i64) -> AppResult<Option<BatchRow>> {
        Ok(sqlx::query_as!(
            BatchRow,
            "SELECT id, tipe, file_path, total_row, sukses, gagal, created_by, created_at
             FROM import_batch WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?)
    }

    pub async fn list_row_log(
        pool: &PgPool,
        batch_id: i64,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<RowLogRow>, i64)> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query_as!(
            RowLogRow,
            "SELECT id, batch_id, row_no, status, error_msg, raw_json
             FROM import_row_log WHERE batch_id = $1 ORDER BY row_no LIMIT $2 OFFSET $3",
            batch_id,
            per_page,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM import_row_log WHERE batch_id = $1")
                .bind(batch_id)
                .fetch_one(pool)
                .await?;
        Ok((rows, total))
    }

    /// Seluruh ruangan aktif: (id, nama, kunci kanonik).
    pub async fn ruangan_map(pool: &PgPool) -> AppResult<Vec<(i64, String)>> {
        Ok(
            sqlx::query_as::<_, (i64, String)>("SELECT id, nama FROM ruangan WHERE is_active")
                .fetch_all(pool)
                .await?,
        )
    }

    /// Item aktif beserta kategori & satuan default-nya.
    pub async fn item_map(pool: &PgPool) -> AppResult<Vec<(i64, String, String, i64)>> {
        Ok(sqlx::query_as::<_, (i64, String, String, i64)>(
            "SELECT i.id, i.nama, k.kode, i.satuan_id
             FROM items i JOIN kategori k ON k.id = i.kategori_id
             WHERE i.is_active",
        )
        .fetch_all(pool)
        .await?)
    }

    pub async fn satuan_map(pool: &PgPool) -> AppResult<Vec<(String, i64)>> {
        Ok(
            sqlx::query_as::<_, (String, i64)>("SELECT kode, id FROM satuan")
                .fetch_all(pool)
                .await?,
        )
    }

    /// Cek apakah baris historis identik sudah pernah diimpor.
    /// Mencegah duplikasi saat import dijalankan berulang.
    #[allow(clippy::too_many_arguments)]
    pub async fn historis_sudah_ada(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tahun: i16,
        ruangan_id: i64,
        item_id: i64,
        jumlah: &bigdecimal::BigDecimal,
        satuan_id: i64,
    ) -> AppResult<bool> {
        let n: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM permintaan
               WHERE client_id = 'IMPORT'
                 AND tahun = $1 AND ruangan_id = $2 AND item_id = $3
                 AND jumlah = $4 AND satuan_id = $5"#,
        )
        .bind(tahun)
        .bind(ruangan_id)
        .bind(item_id)
        .bind(jumlah)
        .bind(satuan_id)
        .fetch_one(&mut **tx)
        .await?;
        Ok(n > 0)
    }

    /// Insert satu permintaan historis (status CLOSED, tanpa SLA).
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_historis(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        batch_id: uuid::Uuid,
        tahun: i16,
        ruangan_id: i64,
        item_id: i64,
        jumlah: &bigdecimal::BigDecimal,
        satuan_id: i64,
        keterangan: Option<&str>,
        created_by: i64,
    ) -> AppResult<i64> {
        let rec = sqlx::query_scalar!(
            r#"INSERT INTO permintaan
                 (batch_id, tahun, ruangan_id, item_id, jumlah, satuan_id, keterangan,
                  status, current_step, created_by, submitted_at, closed_at, client_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'CLOSED', 0, $8, now(), now(), 'IMPORT')
               RETURNING id"#,
            Some(batch_id),
            tahun,
            ruangan_id,
            item_id,
            jumlah,
            satuan_id,
            keterangan,
            created_by
        )
        .fetch_one(&mut **tx)
        .await?;
        Ok(rec)
    }
}
