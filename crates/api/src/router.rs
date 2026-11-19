//! Router utama — semua endpoint + layer middleware + static SPA.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};

use sarpras_shared::api::ApiResponse;

use crate::domains::auth::handler;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/auth/register", axum::routing::post(handler::register))
        .route("/auth/login", axum::routing::post(handler::login))
        .route("/auth/refresh", axum::routing::post(handler::refresh))
        .route("/auth/me", get(handler::me))
        .route("/auth/logout", axum::routing::post(handler::logout))
        .nest("/ruangan", crate::domains::ruangan::handler::router())
        .nest("/kategori", crate::domains::kategori::router())
        .nest("/satuan", crate::domains::satuan::router())
        .nest("/items", crate::domains::item::handler::router())
        .nest("/users", crate::domains::user::handler::router())
        .nest("/permintaan", crate::domains::permintaan::handler::router())
        .nest("/approval", crate::domains::approval::handler::router())
        .nest("/import", crate::domains::import::handler::router())
        .nest("/supplier", crate::domains::supplier::handler::router())
        .nest("/penawaran", crate::domains::penawaran::handler::router())
        .nest("/po", crate::domains::po::handler::router())
        .nest("/penerimaan", crate::domains::penerimaan::handler::router())
        .nest(
            "/serah-terima",
            crate::domains::serah_terima::handler::router(),
        )
        .nest("/aset", crate::domains::aset::handler::router())
        .nest("/dashboard", crate::domains::dashboard::handler::router())
        .nest("/notifikasi", crate::notifikasi::router())
        .nest("/config", crate::domains::config::handler::router())
        .nest("/audit-trail", crate::domains::auditlog::router())
        .nest("/uploads", crate::uploads::router());

    // Static SPA (jika sudah dibuild) — single binary deployment.
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../clients/web-spa/dist");
    if dist.exists() {
        let serve = tower_http::services::ServeDir::new(&dist).not_found_service(
            tower_http::services::ServeFile::new(dist.join("index.html")),
        );
        api.fallback_service(serve)
            .layer(tower_http::cors::CorsLayer::permissive())
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .with_state(state)
    } else {
        api.layer(tower_http::cors::CorsLayer::permissive())
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .with_state(state)
    }
}

async fn health(
    State(state): State<AppState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(ApiResponse::ok(serde_json::json!({
            "status": if db_ok { "ok" } else { "degraded" },
            "service": "sarpras-api",
            "version": env!("CARGO_PKG_VERSION"),
            "database": if db_ok { "ok" } else { "error" },
        }))),
    )
}
