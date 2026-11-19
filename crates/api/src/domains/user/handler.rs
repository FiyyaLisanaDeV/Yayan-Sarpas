//! Handler user.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use validator::Validate;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, parse_role, Role};

use crate::domains::user::repository::UserRepository;
use crate::domains::user::service::UserService;
use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/options", get(options))
        .route("/:id", get(get_one).put(update).delete(soft_delete))
        .route("/:id/reset-password", axum::routing::post(reset_password))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    q: Option<String>,
    role: Option<String>,
    include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OptionsQuery {
    q: Option<String>,
    role: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserBody {
    #[validate(length(min = 2, max = 200, message = "nama wajib 2-200 karakter"))]
    pub nama: String,
    #[validate(length(min = 3, max = 50, message = "nip wajib 3-50 karakter"))]
    pub nip: String,
    #[validate(email(message = "format email tidak valid"))]
    pub email: String,
    /// Username untuk login; bila kosong dipakai bagian email sebelum "@".
    #[validate(length(max = 50, message = "username maksimal 50 karakter"))]
    pub username: Option<String>,
    pub role: String,
    pub ruangan_id: Option<i64>,
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserBody {
    #[validate(length(min = 2, max = 200, message = "nama wajib 2-200 karakter"))]
    pub nama: String,
    pub role: String,
    pub ruangan_id: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordBody {
    pub password: String,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let (page, per_page) = crate::domains::norm_page(q.page, q.per_page);
    let (rows, total) = UserRepository::list(
        &state.pool,
        page,
        per_page,
        q.q.as_deref(),
        q.role.as_deref(),
        q.include_inactive.unwrap_or(false),
    )
    .await?;
    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.id, "nama": r.nama, "nip": r.nip, "email": r.email, "username": r.username, "role": r.role,
        "ruangan": r.ruangan_id.map(|rid| serde_json::json!({"id": rid, "kode": r.ruangan_kode, "nama": r.ruangan_nama})),
        "is_active": r.is_active, "created_at": r.created_at,
    })).collect();
    Ok(ApiResponse::ok_paged(items, page, per_page, total))
}

/// Daftar pengguna aktif untuk dropdown (mis. memilih penerima saat serah terima).
/// Sengaja dibatasi ke field minimal agar tidak membocorkan data pribadi.
async fn options(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<OptionsQuery>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::Auditor,
            Role::PetugasGudang,
            Role::KasiSarpras,
            Role::WadirYanmed,
        ],
    )?;
    let (page, per_page) = (1_i64, 500_i64);
    let (rows, _total) = UserRepository::list(
        &state.pool,
        page,
        per_page,
        q.q.as_deref(),
        q.role.as_deref(),
        false,
    )
    .await?;
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "nama": r.nama,
                "ruangan": r.ruangan_id.map(|rid| serde_json::json!({"id": rid, "nama": r.ruangan_nama})),
            })
        })
        .collect();
    Ok(ApiResponse::ok(items))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin, Role::Auditor])?;
    let r = UserRepository::find(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("User".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({
        "id": r.id, "nama": r.nama, "nip": r.nip, "email": r.email, "username": r.username, "role": r.role,
        "ruangan": r.ruangan_id.map(|rid| serde_json::json!({"id": rid, "kode": r.ruangan_kode, "nama": r.ruangan_nama})),
        "is_active": r.is_active, "created_at": r.created_at,
    })))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Json(body): Json<CreateUserBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let role = parse_role(&body.role)?;
    let r = UserService::create(
        &state,
        actor.id,
        crate::domains::user::service::CreateUserInput {
            nama: &body.nama,
            nip: &body.nip,
            email: &body.email,
            username: body.username.as_deref(),
            role,
            ruangan_id: body.ruangan_id,
            password: &body.password,
        },
    )
    .await?;
    Ok(ApiResponse::ok(
        serde_json::json!({"id": r.id, "nama": r.nama, "username": r.username, "email": r.email, "role": r.role}),
    ))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserBody>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let role = parse_role(&body.role)?;
    let r = UserService::update(
        &state,
        actor.id,
        id,
        &body.nama,
        role,
        body.ruangan_id,
        body.is_active,
    )
    .await?;
    Ok(ApiResponse::ok(
        serde_json::json!({"id": r.id, "nama": r.nama, "email": r.email, "role": r.role, "is_active": r.is_active}),
    ))
}

async fn reset_password(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<ResetPasswordBody>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    UserService::reset_password(&state, actor.id, id, &body.password).await?;
    Ok(ApiResponse::ok(()))
}

async fn soft_delete(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    ensure(actor.role, &[Role::Admin])?;
    UserService::soft_delete(&state, actor.id, id).await?;
    Ok(ApiResponse::ok(()))
}
