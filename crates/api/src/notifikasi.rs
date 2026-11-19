//! Notifikasi — penyimpanan + push WebSocket (IN_APP).
//!
//! Fase 2: kirim notifikasi in-app untuk event SUBMIT.
//! WebSocket broadcast disiapkan di sini agar Fase 3/9 tinggal memakai.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use sarpras_shared::error::{AppError, AppResult};

use crate::state::AppState;

/// Pesan yang dikirim ke klien via WebSocket.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PushMessage {
    pub tipe: String,
    pub ref_id: Option<i64>,
    pub pesan: String,
}

/// Registry channel per user_id (single instance; cukup untuk 1 proses).
#[derive(Clone, Default)]
pub struct PushHub {
    inner: Arc<std::sync::Mutex<HashMap<i64, broadcast::Sender<PushMessage>>>>,
}

impl PushHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ambil/ciptakan channel untuk user.
    pub fn channel(&self, user_id: i64) -> broadcast::Sender<PushMessage> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(user_id)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(64);
                tx
            })
            .clone()
    }

    pub fn subscribe(&self, user_id: i64) -> broadcast::Receiver<PushMessage> {
        self.channel(user_id).subscribe()
    }

    pub fn push(&self, user_id: i64, msg: PushMessage) {
        // Tidak ada subscriber = tidak masalah (klien offline).
        let _ = self.channel(user_id).send(msg);
    }
}

/// Simpan satu notifikasi ke DB.
pub async fn simpan(
    state: &AppState,
    user_id: i64,
    tipe: &str,
    ref_id: Option<i64>,
    pesan: &str,
) -> AppResult<()> {
    sqlx::query!(
        "INSERT INTO notifikasi (user_id, tipe, ref_id, pesan) VALUES ($1, $2, $3, $4)",
        user_id,
        tipe,
        ref_id,
        pesan
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// Kirim notifikasi ke satu user tertentu.
pub async fn kirim_ke_user(
    state: &AppState,
    user_id: i64,
    tipe: &str,
    ref_id: Option<i64>,
    pesan: &str,
) -> AppResult<()> {
    simpan(state, user_id, tipe, ref_id, pesan).await?;
    state.push.push(
        user_id,
        PushMessage {
            tipe: tipe.to_string(),
            ref_id,
            pesan: pesan.to_string(),
        },
    );
    Ok(())
}

/// Kirim notifikasi ke semua user aktif dengan role tertentu.
pub async fn kirim_ke_role(
    state: &AppState,
    role: &str,
    tipe: &str,
    ref_id: Option<i64>,
    pesan: &str,
) -> AppResult<u32> {
    let users: Vec<i64> = sqlx::query_scalar("SELECT id FROM users WHERE role = $1 AND is_active")
        .bind(role)
        .fetch_all(&state.pool)
        .await?;

    let mut terkirim = 0u32;
    for uid in users {
        simpan(state, uid, tipe, ref_id, pesan).await?;
        state.push.push(
            uid,
            PushMessage {
                tipe: tipe.to_string(),
                ref_id,
                pesan: pesan.to_string(),
            },
        );
        terkirim += 1;
    }
    Ok(terkirim)
}

// ---------------------------------------------------------------- endpoint

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list))
        .route("/unread-count", get(unread_count))
        .route("/:id/read", post(mark_read))
        .route("/read-all", post(mark_all_read))
        .route("/ws", get(ws_handler))
}

#[derive(Debug, serde::Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    only_unread: Option<bool>,
}

async fn list(
    axum::extract::State(state): axum::extract::State<AppState>,
    crate::middleware::auth::AuthUser(actor): crate::middleware::auth::AuthUser,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<sarpras_shared::api::ApiResponse<Vec<serde_json::Value>>, AppError> {
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let offset = (page - 1) * per_page;
    let only_unread = q.only_unread.unwrap_or(false);

    let rows = sqlx::query!(
        r#"SELECT id, tipe, ref_id, pesan, is_read, created_at
           FROM notifikasi
           WHERE user_id = $1 AND ($4::bool = false OR is_read = false)
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
        actor.id,
        per_page,
        offset,
        only_unread
    )
    .fetch_all(&state.pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifikasi WHERE user_id = $1 AND ($2::bool = false OR is_read = false)",
    )
    .bind(actor.id)
    .bind(only_unread)
    .fetch_one(&state.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "tipe": r.tipe, "ref_id": r.ref_id,
                "pesan": r.pesan, "is_read": r.is_read, "created_at": r.created_at,
            })
        })
        .collect();

    Ok(sarpras_shared::api::ApiResponse::ok_paged(
        items, page, per_page, total,
    ))
}

async fn unread_count(
    axum::extract::State(state): axum::extract::State<AppState>,
    crate::middleware::auth::AuthUser(actor): crate::middleware::auth::AuthUser,
) -> Result<sarpras_shared::api::ApiResponse<serde_json::Value>, AppError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifikasi WHERE user_id = $1 AND is_read = false",
    )
    .bind(actor.id)
    .fetch_one(&state.pool)
    .await?;
    Ok(sarpras_shared::api::ApiResponse::ok(
        serde_json::json!({ "unread": n }),
    ))
}

async fn mark_read(
    axum::extract::State(state): axum::extract::State<AppState>,
    crate::middleware::auth::AuthUser(actor): crate::middleware::auth::AuthUser,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<sarpras_shared::api::ApiResponse<()>, AppError> {
    let n = sqlx::query!(
        "UPDATE notifikasi SET is_read = true WHERE id = $1 AND user_id = $2",
        id,
        actor.id
    )
    .execute(&state.pool)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound("Notifikasi".into()));
    }
    Ok(sarpras_shared::api::ApiResponse::ok(()))
}

async fn mark_all_read(
    axum::extract::State(state): axum::extract::State<AppState>,
    crate::middleware::auth::AuthUser(actor): crate::middleware::auth::AuthUser,
) -> Result<sarpras_shared::api::ApiResponse<serde_json::Value>, AppError> {
    let n = sqlx::query!(
        "UPDATE notifikasi SET is_read = true WHERE user_id = $1 AND is_read = false",
        actor.id
    )
    .execute(&state.pool)
    .await?
    .rows_affected();
    Ok(sarpras_shared::api::ApiResponse::ok(
        serde_json::json!({ "ditandai": n }),
    ))
}

/// WebSocket: klien menerima push untuk user-nya sendiri.
async fn ws_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    crate::middleware::auth::AuthUser(actor): crate::middleware::auth::AuthUser,
    ws: axum::extract::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let mut rx = state.push.subscribe(actor.id);
        let (mut sender, mut receiver) = socket.split();

        use futures_util::{SinkExt, StreamExt};

        let mut kirim = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let teks = serde_json::to_string(&msg).unwrap_or_default();
                if sender
                    .send(axum::extract::ws::Message::Text(teks))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        // Abaikan pesan masuk; cukup deteksi penutupan.
        let mut terima =
            tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

        tokio::select! {
            _ = &mut kirim => terima.abort(),
            _ = &mut terima => kirim.abort(),
        }
    })
}
