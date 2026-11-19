//! Repository permintaan — akses DB (sqlx compile-time checked).

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sarpras_shared::error::AppResult;

/// Satu baris permintaan (satu item).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PermintaanRow {
    pub id: i64,
    pub batch_id: Option<Uuid>,
    pub tahun: i16,
    pub ruangan_id: i64,
    pub ruangan_nama: String,
    pub item_id: i64,
    pub item_nama: String,
    pub kategori_kode: String,
    pub jumlah: BigDecimal,
    pub satuan_id: i64,
    pub satuan_kode: String,
    pub keterangan: Option<String>,
    pub status: String,
    pub current_step: i16,
    pub revisi_ke: i32,
    pub sla_deadline: Option<DateTime<Utc>>,
    pub sla_status: Option<String>,
    pub created_by: i64,
    pub submitted_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

pub struct PermintaanRepository;

const SELECT_COLS: &str = r#"
    SELECT p.id, p.batch_id, p.tahun, p.ruangan_id,
           r.nama AS ruangan_nama,
           p.item_id, i.nama AS item_nama,
           k.kode AS kategori_kode,
           p.jumlah, p.satuan_id, s.kode AS satuan_kode,
           p.keterangan, p.status, p.current_step, p.revisi_ke,
           p.sla_deadline, p.sla_status,
           p.created_by, p.submitted_at, p.closed_at, p.updated_at
    FROM permintaan p
    JOIN ruangan r ON r.id = p.ruangan_id
    JOIN items i ON i.id = p.item_id
    JOIN satuan s ON s.id = p.satuan_id
    JOIN kategori k ON k.id = i.kategori_id
"#;

impl PermintaanRepository {
    /// Baris milik satu batch (dipakai setelah create/submit).
    pub async fn by_batch(pool: &PgPool, batch_id: Uuid) -> AppResult<Vec<PermintaanRow>> {
        Ok(sqlx::query_as::<_, PermintaanRow>(&format!(
            "{SELECT_COLS} WHERE p.batch_id = $1 ORDER BY p.id"
        ))
        .bind(batch_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<PermintaanRow>> {
        Ok(
            sqlx::query_as::<_, PermintaanRow>(&format!("{SELECT_COLS} WHERE p.id = $1"))
                .bind(id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Daftar permintaan berpaginasi.
    /// `ruangan_id` = Some → hanya milik ruangan itu (USER_RUANGAN).
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        tahun: Option<i16>,
        status: Option<&str>,
        ruangan_id: Option<i64>,
        search: Option<&str>,
    ) -> AppResult<(Vec<PermintaanRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as::<_, PermintaanRow>(&format!(
            r#"{SELECT_COLS}
               WHERE ($3::smallint IS NULL OR p.tahun = $3)
                 AND ($4::text IS NULL OR p.status = $4)
                 AND ($5::bigint IS NULL OR p.ruangan_id = $5)
                 AND ($6::text IS NULL OR lower(i.nama) LIKE $6)
               ORDER BY p.updated_at DESC, p.id DESC
               LIMIT $1 OFFSET $2"#
        ))
        .bind(per_page)
        .bind(offset)
        .bind(tahun)
        .bind(status)
        .bind(ruangan_id)
        .bind(pattern.clone())
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM permintaan p JOIN items i ON i.id = p.item_id
               WHERE ($1::smallint IS NULL OR p.tahun = $1)
                 AND ($2::text IS NULL OR p.status = $2)
                 AND ($3::bigint IS NULL OR p.ruangan_id = $3)
                 AND ($4::text IS NULL OR lower(i.nama) LIKE $4)"#,
        )
        .bind(tahun)
        .bind(status)
        .bind(ruangan_id)
        .bind(pattern)
        .fetch_one(pool)
        .await?;

        Ok((rows, total))
    }

    /// Simpan satu baris permintaan dalam transaksi (dipanggil per item).
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_row(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        batch_id: Uuid,
        tahun: i16,
        ruangan_id: i64,
        item_id: i64,
        jumlah: &BigDecimal,
        satuan_id: i64,
        keterangan: Option<&str>,
        created_by: i64,
    ) -> AppResult<i64> {
        let rec = sqlx::query_scalar!(
            r#"INSERT INTO permintaan
                 (batch_id, tahun, ruangan_id, item_id, jumlah, satuan_id, keterangan, status, created_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'DRAFT', $8)
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

    /// Tandai satu baris sebagai SUBMITTED.
    pub async fn mark_submitted(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        status: &str,
        sla_deadline: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"UPDATE permintaan
               SET status = $2, submitted_at = now(), sla_deadline = $3, current_step = 1
               WHERE id = $1"#,
            id,
            status,
            sla_deadline
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Submit ulang setelah penolakan: revisi_ke naik, step kembali ke 1.
    pub async fn mark_resubmitted(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        sla_deadline: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"UPDATE permintaan
               SET status = 'SUBMITTED', submitted_at = now(), sla_deadline = $2,
                   current_step = 1, revisi_ke = revisi_ke + 1
               WHERE id = $1"#,
            id,
            sla_deadline
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Update baris saat masih DRAFT.
    pub async fn update_draft(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        jumlah: &BigDecimal,
        satuan_id: i64,
        keterangan: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE permintaan SET jumlah = $2, satuan_id = $3, keterangan = $4 WHERE id = $1",
            id,
            jumlah,
            satuan_id,
            keterangan
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Hapus baris DRAFT (satu atau satu batch).
    pub async fn delete_draft_batch(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        batch_id: Uuid,
        owner_id: i64,
    ) -> AppResult<u64> {
        let n = sqlx::query!(
            "DELETE FROM permintaan WHERE batch_id = $1 AND created_by = $2 AND status = 'DRAFT'",
            batch_id,
            owner_id
        )
        .execute(&mut **tx)
        .await?
        .rows_affected();
        Ok(n)
    }

    /// Baris yang siap di-PO (FINAL) untuk dropdown Buat PO (Fase 8).
    pub async fn count_final(pool: &PgPool, ruangan_id: Option<i64>) -> AppResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM permintaan WHERE status = 'FINAL' AND ($1::bigint IS NULL OR ruangan_id = $1)",
        )
        .bind(ruangan_id)
        .fetch_one(pool)
        .await?)
    }
}
