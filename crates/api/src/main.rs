//! sarpras-api — binary tunggal: server axum + background jobs (worker).

use std::net::SocketAddr;

use sqlx::postgres::PgPoolOptions;
use tracing::info;

use sarpras_api::router;
use sarpras_api::state::{self, RateLimiter};
use sarpras_shared::app_config::AppConfig;
use sarpras_shared::config::ConfigLoader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sarpras_api=info,sqlx=warn,tower_http=info".into()),
        )
        .init();

    let cfg = AppConfig::load()?;
    if let Err(e) = cfg.validate() {
        tracing::warn!("{e}");
    }

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;
    info!("Terhubung ke database.");

    // Migrasi idempoten saat startup (single binary: setup tanpa langkah ekstra).
    sqlx::migrate!("../../migrations").run(&pool).await?;

    // Cache config: invalidate total saat startup (fresh cache).
    let loader = ConfigLoader::new(pool.clone());
    loader.invalidate_all();

    let app_state = state::AppState {
        pool: pool.clone(),
        cfg: std::sync::Arc::new(cfg.clone()),
        config: loader,
        rate: std::sync::Arc::new(RateLimiter::new()),
        push: sarpras_api::notifikasi::PushHub::new(),
    };

    // Background jobs dalam proses yang sama (tokio task).
    sarpras_worker::spawn_all(pool.clone()).await?;

    let app = router::build_router(app_state);
    let addr: SocketAddr = cfg.bind_addr.parse()?;
    info!("SARPRAS API berjalan di http://{addr} (binary tunggal api+worker)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
