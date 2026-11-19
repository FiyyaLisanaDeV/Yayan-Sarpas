//! Repository approval — inbox, approval_log, sla_tracking.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

/// Satu baris permintaan dalam konteks inbox approval.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InboxRow {
    pub id: i64,
    pub batch_id: Option<uuid::Uuid>,
    pub tahun: i16,
    pub ruangan_nama: String,
    pub item_nama: String,
    pub kategori_kode: String,
    pub jumlah: bigdecimal::BigDecimal,
    pub satuan_kode: String,
    pub keterangan: Option<String>,
    pub status: String,
    pub current_step: i16,
    pub sla_deadline: Option<DateTime<Utc>>,
    pub created_by: i64,
    pub pemohon_nama: String,
    pub submitted_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

const SELECT_COLS: &str = r#"
    SELECT p.id, p.batch_id, p.tahun,
           r.nama AS ruangan_nama,
           i.nama AS item_nama,
           k.kode AS kategori_kode,
           p.jumlah, s.kode AS satuan_kode,
           p.keterangan, p.status, p.current_step,
           p.sla_deadline, p.created_by,
           u.nama AS pemohon_nama,
           p.submitted_at, p.updated_at
    FROM permintaan p
    JOIN ruangan r ON r.id = p.ruangan_id
    JOIN items i ON i.id = p.item_id
    JOIN satuan s ON s.id = p.satuan_id
    JOIN kategori k ON k.id = i.kategori_id
    JOIN users u ON u.id = p.created_by
"#;

/// Satu entri riwayat approval.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LogRow {
    pub id: i64,
    pub permintaan_id: i64,
    pub step: i16,
    pub actor_id: i64,
    pub actor_nama: String,
    pub actor_role: String,
    pub aksi: String,
    pub catatan: Option<String>,
    pub timestamp: DateTime<Utc>,
}

pub struct ApprovalRepository;

impl ApprovalRepository {
    /// Baris yang menunggu aksi `role` pada `step` saat ini.
    pub async fn inbox(
        pool: &PgPool,
        role: &str,
        status_list: &[String],
        search: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<InboxRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as::<_, InboxRow>(&format!(
            r#"{SELECT_COLS}
               WHERE p.status = ANY($1)
                 AND ($2::text IS NULL OR lower(i.nama) LIKE $2 OR lower(r.nama) LIKE $2)
               ORDER BY p.sla_deadline NULLS LAST, p.updated_at DESC
               LIMIT $3 OFFSET $4"#
        ))
        .bind(status_list)
        .bind(pattern.clone())
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM permintaan p
               JOIN items i ON i.id = p.item_id
               JOIN ruangan r ON r.id = p.ruangan_id
               WHERE p.status = ANY($1)
                 AND ($2::text IS NULL OR lower(i.nama) LIKE $2 OR lower(r.nama) LIKE $2)"#,
        )
        .bind(status_list)
        .bind(pattern)
        .fetch_one(pool)
        .await?;

        let _ = role;
        Ok((rows, total))
    }

    /// Riwayat approval satu baris permintaan.
    pub async fn logs(pool: &PgPool, permintaan_id: i64) -> AppResult<Vec<LogRow>> {
        Ok(sqlx::query_as::<_, LogRow>(
            r#"SELECT al.id, al.permintaan_id, al.step, al.actor_id,
                      u.nama AS actor_nama, u.role AS actor_role,
                      al.aksi, al.catatan, al.timestamp
               FROM approval_log al JOIN users u ON u.id = al.actor_id
               WHERE al.permintaan_id = $1
               ORDER BY al.id"#,
        )
        .bind(permintaan_id)
        .fetch_all(pool)
        .await?)
    }

    /// Tambah entri riwayat.
    pub async fn insert_log(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        permintaan_id: i64,
        step: i16,
        actor_id: i64,
        aksi: &str,
        catatan: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query!(
            "INSERT INTO approval_log (permintaan_id, step, actor_id, aksi, catatan)
             VALUES ($1, $2, $3, $4, $5)",
            permintaan_id,
            step,
            actor_id,
            aksi,
            catatan
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Ubah status + step baris permintaan.
    pub async fn set_status_step(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        status: &str,
        step: i16,
        deadline: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE permintaan SET status = $2, current_step = $3, sla_deadline = $4 WHERE id = $1",
            id,
            status,
            step,
            deadline
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Tutup semua tracking SLA untuk satu baris (dipanggil saat keluar step).
    pub async fn close_sla(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ref_id: i64,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE sla_tracking SET closed_at = now()
             WHERE ref_type = 'permintaan' AND ref_id = $1 AND closed_at IS NULL",
            ref_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Buka tracking SLA untuk step yang baru dimasuki.
    pub async fn open_sla(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ref_id: i64,
        step_label: &str,
        deadline: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query!(
            "INSERT INTO sla_tracking (ref_type, ref_id, step, deadline)
             VALUES ('permintaan', $1, $2, $3)",
            ref_id,
            step_label,
            deadline
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Semua baris dalam batch (agar approval berlaku satu kesatuan).
    pub async fn batch_ids(pool: &PgPool, batch_id: uuid::Uuid) -> AppResult<Vec<i64>> {
        Ok(
            sqlx::query_scalar("SELECT id FROM permintaan WHERE batch_id = $1 ORDER BY id")
                .bind(batch_id)
                .fetch_all(pool)
                .await?,
        )
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<InboxRow>> {
        Ok(
            sqlx::query_as::<_, InboxRow>(&format!("{SELECT_COLS} WHERE p.id = $1"))
                .bind(id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Jumlah baris yang menunggu role (untuk badge).
    pub async fn count_pending(pool: &PgPool, status_list: &[String]) -> AppResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM permintaan WHERE status = ANY($1)")
                .bind(status_list)
                .fetch_one(pool)
                .await?,
        )
    }
}
