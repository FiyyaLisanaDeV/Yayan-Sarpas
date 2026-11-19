//! Domain satuan — CRUD ringkas, 3 layer dalam satu modul.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::{AppError, AppResult};
use sarpras_shared::rbac::{ensure, Role};

use crate::domains::PaginationQuery;
use crate::middleware::auth::AuthUser;
use crate::middleware::record_audit;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", get(get_one).put(update))
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SatuanRow {
    pub id: i64,
    pub kode: String,
    pub nama: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SatuanBody {
    #[validate(length(min = 1, max = 20))]
    pub kode: String,
    #[validate(length(min = 1, max = 50))]
    pub nama: String,
}

// ---- Repository ----
struct Repo;

impl Repo {
    async fn list(
        pool: &sqlx::PgPool,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<SatuanRow>, i64)> {
        let rows = sqlx::query_as!(
            SatuanRow,
            "SELECT id, kode, nama FROM satuan ORDER BY id LIMIT $1 OFFSET $2",
            per_page,
            (page - 1) * per_page
        )
        .fetch_all(pool)
        .await?;
        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM satuan")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
        Ok((rows, total))
    }

    async fn find(pool: &sqlx::PgPool, id: i64) -> AppResult<Option<SatuanRow>> {
        Ok(sqlx::query_as!(
            SatuanRow,
            "SELECT id, kode, nama FROM satuan WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?)
    }

    async fn create(pool: &sqlx::PgPool, kode: &str, nama: &str) -> AppResult<i64> {
        let rec = sqlx::query!(
            "INSERT INTO satuan (kode, nama) VALUES ($1, $2) RETURNING id",
            kode,
            nama
        )
        .fetch_one(pool)
        .await?;
        Ok(rec.id)
    }

    async fn update(pool: &sqlx::PgPool, id: i64, nama: &str) -> AppResult<()> {
        sqlx::query!("UPDATE satuan SET nama = $2 WHERE id = $1", id, nama)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// ---- Service ----
struct Svc;

impl Svc {
    async fn create(
        state: &AppState,
        actor_id: i64,
        kode: &str,
        nama: &str,
    ) -> AppResult<SatuanRow> {
        let kode = kode.trim().to_uppercase();
        let id = crate::db_util::map_pg_error(
            Repo::create(&state.pool, &kode, nama.trim()).await,
            "Satuan",
        )?;
        record_audit(
            &state.pool,
            actor_id,
            "satuan",
            id,
            "CREATE",
            None,
            Some(&json!({"kode": kode, "nama": nama.trim()})),
        )
        .await?;
        Repo::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    async fn update(state: &AppState, actor_id: i64, id: i64, nama: &str) -> AppResult<SatuanRow> {
        let before = Repo::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Satuan".into()))?;
        Repo::update(&state.pool, id, nama.trim()).await?;
        record_audit(
            &state.pool,
            actor_id,
            "satuan",
            id,
            "UPDATE",
            Some(&json!({"nama": before.nama})),
            Some(&json!({"nama": nama.trim()})),
        )
        .await?;
        Repo::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }
}

// ---- Handler ----
async fn list(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<PaginationQuery>,
) -> Result<ApiResponse<Vec<SatuanRow>>, AppError> {
    let (page, per_page, _) = q.norm();
    let (rows, total) = Repo::list(&state.pool, page, per_page).await?;
    Ok(ApiResponse::ok_paged(rows, page, per_page, total))
}

async fn get_one(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<SatuanRow>, AppError> {
    let row = Repo::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Satuan".into()))?;
    Ok(ApiResponse::ok(row))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<SatuanBody>,
) -> Result<ApiResponse<SatuanRow>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let row = Svc::create(&state, actor.id, &body.kode, &body.nama).await?;
    Ok(ApiResponse::ok(row))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<SatuanBody>,
) -> Result<ApiResponse<SatuanRow>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let row = Svc::update(&state, actor.id, id, &body.nama).await?;
    Ok(ApiResponse::ok(row))
}
