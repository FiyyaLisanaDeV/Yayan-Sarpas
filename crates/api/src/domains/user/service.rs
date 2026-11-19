//! Service user — kebijakan password dari config (K03/K04), audit.

use serde_json::json;

use sarpras_shared::auth::password::{hash_password, validate_policy};
use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};
use sarpras_shared::rbac::Role;

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{UserRepository, UserRow};

pub struct UserService;

/// Parameter pembuatan user (menghindari fungsi dengan terlalu banyak argumen).
pub struct CreateUserInput<'a> {
    pub nama: &'a str,
    pub nip: &'a str,
    pub email: &'a str,
    pub username: Option<&'a str>,
    pub role: Role,
    pub ruangan_id: Option<i64>,
    pub password: &'a str,
}

impl UserService {
    pub async fn create(
        state: &AppState,
        actor_id: i64,
        input: CreateUserInput<'_>,
    ) -> AppResult<UserRow> {
        let CreateUserInput {
            nama,
            nip,
            email,
            username,
            role,
            ruangan_id,
            password,
        } = input;
        Self::cek_policy_password(state, password).await?;
        Self::validasi_ruangan(state, ruangan_id).await?;

        let email = email.trim().to_lowercase();
        // Username default: bagian sebelum "@" pada email.
        let username = username
            .map(|u| u.trim().to_lowercase())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_lowercase());
        let hash = hash_password(password)?;
        let id = crate::db_util::map_pg_error(
            UserRepository::create(
                &state.pool,
                nama.trim(),
                nip.trim(),
                &email,
                &username,
                &hash,
                role.as_str(),
                ruangan_id,
            )
            .await,
            "User (nip/email/username)",
        )?;

        record_audit(
            &state.pool,
            actor_id,
            "users",
            id,
            "CREATE",
            None,
            Some(&json!({"nama": nama.trim(), "nip": nip.trim(), "email": email, "username": username, "role": role.as_str(), "ruangan_id": ruangan_id})),
        )
        .await?;
        UserRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    pub async fn update(
        state: &AppState,
        actor_id: i64,
        id: i64,
        nama: &str,
        role: Role,
        ruangan_id: Option<i64>,
        is_active: bool,
    ) -> AppResult<UserRow> {
        Self::validasi_ruangan(state, ruangan_id).await?;

        let before = UserRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("User".into()))?;

        // Perlindungan admin terakhir: tidak boleh menonaktifkan
        // atau menurunkan role admin aktif terakhir.
        if before.role == "ADMIN" && before.is_active && (!is_active || role != Role::Admin) {
            let active_admins = UserRepository::count_active_admin(&state.pool).await?;
            if active_admins <= 1 {
                return Err(AppError::Conflict(
                    "Tidak bisa menonaktifkan/menurunkan admin aktif terakhir".into(),
                ));
            }
        }

        UserRepository::update(
            &state.pool,
            id,
            nama.trim(),
            role.as_str(),
            ruangan_id,
            is_active,
        )
        .await?;
        record_audit(
            &state.pool,
            actor_id,
            "users",
            id,
            "UPDATE",
            Some(&json!({"nama": before.nama, "role": before.role, "ruangan_id": before.ruangan_id, "is_active": before.is_active})),
            Some(&json!({"nama": nama.trim(), "role": role.as_str(), "ruangan_id": ruangan_id, "is_active": is_active})),
        )
        .await?;
        UserRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    pub async fn reset_password(
        state: &AppState,
        actor_id: i64,
        id: i64,
        password: &str,
    ) -> AppResult<()> {
        Self::cek_policy_password(state, password).await?;
        let before = UserRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("User".into()))?;
        let hash = hash_password(password)?;
        UserRepository::update_password(&state.pool, id, &hash).await?;
        record_audit(
            &state.pool,
            actor_id,
            "users",
            id,
            "RESET_PASSWORD",
            None,
            None,
        )
        .await?;
        tracing::info!(
            actor = actor_id,
            target = id,
            "password user {} direset",
            before.email
        );
        Ok(())
    }

    pub async fn soft_delete(state: &AppState, actor_id: i64, id: i64) -> AppResult<()> {
        if actor_id == id {
            return Err(AppError::Conflict(
                "Tidak bisa menonaktifkan akun sendiri".into(),
            ));
        }
        let before = UserRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("User".into()))?;
        if !before.is_active {
            return Err(AppError::Conflict("User sudah nonaktif".into()));
        }
        if before.role == "ADMIN" {
            let active_admins = UserRepository::count_active_admin(&state.pool).await?;
            if active_admins <= 1 {
                return Err(AppError::Conflict(
                    "Tidak bisa menonaktifkan admin aktif terakhir".into(),
                ));
            }
        }
        UserRepository::update(
            &state.pool,
            id,
            &before.nama,
            &before.role,
            before.ruangan_id,
            false,
        )
        .await?;
        record_audit(
            &state.pool,
            actor_id,
            "users",
            id,
            "DELETE",
            Some(&json!({"nama": before.nama, "role": before.role})),
            None,
        )
        .await?;
        Ok(())
    }

    async fn cek_policy_password(state: &AppState, password: &str) -> AppResult<()> {
        let ctx = ConfigContext::global();
        let min_length = state.config.get_i64("K03", &ctx).await? as usize;
        let require_complex = state.config.get_bool("K04", &ctx).await?;
        validate_policy(password, min_length, require_complex)?;
        Ok(())
    }

    async fn validasi_ruangan(state: &AppState, ruangan_id: Option<i64>) -> AppResult<()> {
        if let Some(rid) = ruangan_id {
            let ada = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM ruangan WHERE id = $1 AND is_active",
                rid
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0)
                > 0;
            if !ada {
                return Err(AppError::Validation(
                    "ruangan_id tidak ditemukan atau nonaktif".into(),
                ));
            }
        }
        Ok(())
    }
}
