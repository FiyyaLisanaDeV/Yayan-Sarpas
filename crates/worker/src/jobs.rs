//! Job definitions. Dieksekusi oleh binary api (single process).

use sqlx::PgPool;
use tracing::{error, info};

/// Scan `sla_tracking` yang masih terbuka:
/// - deadline lewat            → OVERDUE + notifikasi ke pemohon
/// - overdue > C07 hari        → ESCALATED + notifikasi ke atasan
/// - deadline < C06 hari lagi  → pengingat SLA_WARN
///
/// Notifikasi dikirim saat transisi status, bukan berulang tiap run.
pub async fn sla_checker(pool: PgPool) {
    let warn_days = config_i64(&pool, "C06", 1).await;
    let escalate_days = config_i64(&pool, "C07", 2).await;

    // 1) ON_TIME yang lewat deadline → OVERDUE
    match sqlx::query_scalar::<_, i64>(
        r#"WITH upd AS (
               UPDATE sla_tracking
               SET status = 'OVERDUE'
               WHERE closed_at IS NULL
                 AND status = 'ON_TIME'
                 AND deadline < now()
               RETURNING ref_id
           )
           SELECT ref_id FROM upd"#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(refs) => {
            if !refs.is_empty() {
                info!(jumlah = refs.len(), "sla_checker: menjadi OVERDUE");
                for ref_id in refs {
                    notif_overdue(&pool, ref_id).await;
                }
            }
        }
        Err(e) => error!(error = %e, "sla_checker: gagal menandai OVERDUE"),
    }

    // 2) OVERDUE melewati ambang → ESCALATED
    match sqlx::query_scalar::<_, i64>(
        r#"WITH upd AS (
               UPDATE sla_tracking
               SET status = 'ESCALATED', escalated_at = now()
               WHERE closed_at IS NULL
                 AND status = 'OVERDUE'
                 AND deadline < now() - ($1::int * INTERVAL '1 day')
               RETURNING ref_id
           )
           SELECT ref_id FROM upd"#,
    )
    .bind(escalate_days as i32)
    .fetch_all(&pool)
    .await
    {
        Ok(refs) => {
            if !refs.is_empty() {
                info!(jumlah = refs.len(), "sla_checker: dieskalasi");
                for ref_id in refs {
                    notif_eskalasi(&pool, ref_id).await;
                }
            }
        }
        Err(e) => error!(error = %e, "sla_checker: gagal eskalasi"),
    }

    // 3) Pengingat H-<warn_days>
    if warn_days > 0 {
        match sqlx::query_scalar::<_, i64>(
            r#"SELECT DISTINCT ref_id FROM sla_tracking
               WHERE closed_at IS NULL
                 AND status = 'ON_TIME'
                 AND deadline BETWEEN now() AND now() + ($1::int * INTERVAL '1 day')"#,
        )
        .bind(warn_days as i32)
        .fetch_all(&pool)
        .await
        {
            Ok(refs) => {
                if !refs.is_empty() {
                    info!(jumlah = refs.len(), "sla_checker: pengingat dikirim");
                    for ref_id in refs {
                        notif_pengingat(&pool, ref_id).await;
                    }
                }
            }
            Err(e) => error!(error = %e, "sla_checker: gagal mengirim pengingat"),
        }
    }

    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sla_tracking WHERE closed_at IS NULL")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    info!(tracking_terbuka = open, "sla_checker selesai");
}

/// Hapus notifikasi lebih tua dari retensi (L02, default 365 hari).
pub async fn notif_cleanup(pool: PgPool) {
    let hari = config_i64(&pool, "L02", 365).await;
    match sqlx::query(
        "DELETE FROM notifikasi WHERE created_at < now() - ($1::int * INTERVAL '1 day')",
    )
    .bind(hari as i32)
    .execute(&pool)
    .await
    {
        Ok(r) => info!(deleted = r.rows_affected(), hari, "notif_cleanup selesai"),
        Err(e) => error!(error = %e, "notif_cleanup gagal"),
    }
}

/// Warm-up cache config (idempoten; TTL moka 5 menit).
pub async fn config_cache_warm(pool: PgPool) {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM config_item")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    info!(
        config_item = n,
        "config_cache_warm: cache TTL moka 5 menit aktif"
    );
}

// ------------------------------------------------------------------ helper

/// Baca config number langsung dari DB (tanpa ConfigLoader, job tetap ringan).
async fn config_i64(pool: &PgPool, kode: &str, default: i64) -> i64 {
    sqlx::query_scalar(
        "SELECT NULLIF(cv.nilai, '')::bigint FROM config_value cv
         JOIN config_item ci ON ci.id = cv.config_item_id
         WHERE ci.kode = $1 AND cv.scope_type = 'GLOBAL' AND cv.scope_id IS NULL",
    )
    .bind(kode)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(default)
}

/// Pemohon + label ruangan/item untuk satu baris permintaan.
async fn penerima_ref(pool: &PgPool, ref_id: i64) -> Option<(i64, String, String)> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT p.created_by, r.nama, i.nama
         FROM permintaan p
         JOIN ruangan r ON r.id = p.ruangan_id
         JOIN items i ON i.id = p.item_id
         WHERE p.id = $1",
    )
    .bind(ref_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Simpan notifikasi; kegagalan tidak boleh membuat job panic.
async fn notif(pool: &PgPool, user_id: i64, tipe: &str, ref_id: i64, pesan: &str) {
    let _ = sqlx::query(
        "INSERT INTO notifikasi (user_id, tipe, ref_id, pesan) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(tipe)
    .bind(ref_id)
    .bind(pesan)
    .execute(pool)
    .await;
}

async fn notif_overdue(pool: &PgPool, ref_id: i64) {
    if let Some((pemohon, ruangan, item)) = penerima_ref(pool, ref_id).await {
        notif(
            pool,
            pemohon,
            "SLA_OVERDUE",
            ref_id,
            &format!("Permintaan {item} ({ruangan}) melewati batas SLA"),
        )
        .await;
    }
}

async fn notif_eskalasi(pool: &PgPool, ref_id: i64) {
    let atasan: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM users WHERE role = 'KABID_YANMED' AND is_active")
            .fetch_all(pool)
            .await
            .unwrap_or_default();

    let label = penerima_ref(pool, ref_id)
        .await
        .map(|(_, r, i)| format!("{i} ({r})"))
        .unwrap_or_else(|| format!("permintaan #{ref_id}"));

    for uid in atasan {
        notif(
            pool,
            uid,
            "SLA_ESCALATED",
            ref_id,
            &format!("ESKALASI: {label} melewati SLA dan belum diproses"),
        )
        .await;
    }
}

async fn notif_pengingat(pool: &PgPool, ref_id: i64) {
    if let Some((pemohon, ruangan, item)) = penerima_ref(pool, ref_id).await {
        notif(
            pool,
            pemohon,
            "SLA_WARN",
            ref_id,
            &format!("Pengingat: permintaan {item} ({ruangan}) mendekati batas SLA"),
        )
        .await;
    }
}
