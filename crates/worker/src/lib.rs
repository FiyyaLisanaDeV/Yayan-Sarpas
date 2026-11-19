//! sarpras-worker — background jobs (SLA cek, notif cleanup, cache warm).
//! Berjalan sebagai tokio task dalam SATU proses bersama api.
//!
//! Format cron tokio-cron-scheduler: 6 atau 7 field
//! (detik menit jam hari-bulan bulan hari-minggu [tahun]).

pub mod jobs;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Jadwal:
/// - sla_checker   : setiap hari 08:00
/// - notif_cleanup : setiap hari 02:00
/// - cache_warm    : setiap 5 menit
pub async fn spawn_all(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = JobScheduler::new().await?;

    {
        let pool = pool.clone();
        scheduler
            .add(Job::new_async("0 0 8 * * *", move |_id, _lock| {
                let pool = pool.clone();
                Box::pin(async move {
                    jobs::sla_checker(pool).await;
                })
            })?)
            .await?;
    }

    {
        let pool = pool.clone();
        scheduler
            .add(Job::new_async("0 0 2 * * *", move |_id, _lock| {
                let pool = pool.clone();
                Box::pin(async move {
                    jobs::notif_cleanup(pool).await;
                })
            })?)
            .await?;
    }

    {
        let pool = pool.clone();
        scheduler
            .add(Job::new_async("0 */5 * * * *", move |_id, _lock| {
                let pool = pool.clone();
                Box::pin(async move {
                    jobs::config_cache_warm(pool).await;
                })
            })?)
            .await?;
    }

    scheduler.start().await?;
    tracing::info!(
        "Job scheduler aktif (sla_checker 08:00, notif_cleanup 02:00, cache_warm tiap 5m)"
    );

    // Scheduler tetap hidup selama proses berjalan.
    tokio::spawn(async move {
        let _scheduler = scheduler;
        std::future::pending::<()>().await;
    });

    Ok(())
}
