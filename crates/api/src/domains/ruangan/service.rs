//! Service ruangan — validasi + audit trail.

use serde_json::json;

use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{RuanganRepository, RuanganRow};

pub struct RuanganService;

impl RuanganService {
    pub async fn create(
        state: &AppState,
        actor_id: i64,
        kode: &str,
        nama: &str,
    ) -> AppResult<RuanganRow> {
        let kode = kode.trim().to_uppercase();
        let nama = nama.trim();
        if kode.is_empty() || nama.is_empty() {
            return Err(AppError::Validation("kode dan nama wajib diisi".into()));
        }
        if RuanganRepository::find_by_kode(&state.pool, &kode)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Kode ruangan '{kode}' sudah terdaftar"
            )));
        }

        let id = RuanganRepository::create(&state.pool, &kode, nama).await?;
        record_audit(
            &state.pool,
            actor_id,
            "ruangan",
            id,
            "CREATE",
            None,
            Some(&json!({"kode": kode, "nama": nama})),
        )
        .await?;

        RuanganRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    pub async fn update(
        state: &AppState,
        actor_id: i64,
        id: i64,
        kode: &str,
        nama: &str,
    ) -> AppResult<RuanganRow> {
        let kode = kode.trim().to_uppercase();
        let nama = nama.trim();
        if kode.is_empty() || nama.is_empty() {
            return Err(AppError::Validation("kode dan nama wajib diisi".into()));
        }

        let before = RuanganRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Ruangan".into()))?;

        if let Some(existing) = RuanganRepository::find_by_kode(&state.pool, &kode).await? {
            if existing.id != id {
                return Err(AppError::Conflict(format!(
                    "Kode ruangan '{kode}' sudah dipakai ruangan lain"
                )));
            }
        }

        RuanganRepository::update(&state.pool, id, &kode, nama).await?;
        record_audit(
            &state.pool,
            actor_id,
            "ruangan",
            id,
            "UPDATE",
            Some(&json!({"kode": before.kode, "nama": before.nama})),
            Some(&json!({"kode": kode, "nama": nama})),
        )
        .await?;

        RuanganRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    pub async fn soft_delete(state: &AppState, actor_id: i64, id: i64) -> AppResult<()> {
        let before = RuanganRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Ruangan".into()))?;
        if !before.is_active {
            return Err(AppError::Conflict("Ruangan sudah nonaktif".into()));
        }

        // Jangan nonaktifkan ruangan yang masih dipakai user aktif.
        let dipakai = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users WHERE ruangan_id = $1 AND is_active",
            id
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(0);
        if dipakai > 0 {
            return Err(AppError::Conflict(format!(
                "Ruangan masih dipakai {dipakai} user aktif — pindahkan dulu user tersebut"
            )));
        }

        RuanganRepository::soft_delete(&state.pool, id).await?;
        record_audit(
            &state.pool,
            actor_id,
            "ruangan",
            id,
            "DELETE",
            Some(&json!({"kode": before.kode, "nama": before.nama})),
            None,
        )
        .await?;
        Ok(())
    }
}
