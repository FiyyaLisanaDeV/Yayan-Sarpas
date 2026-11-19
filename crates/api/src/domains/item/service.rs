//! Service item — validasi relasi + audit.

use serde_json::json;

use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{ItemRepository, ItemRow};

pub struct ItemService;

/// Konversi input f64 → BigDecimal untuk kolom NUMERIC.
fn to_decimal(v: Option<f64>) -> Option<bigdecimal::BigDecimal> {
    use std::str::FromStr;
    v.and_then(|x| bigdecimal::BigDecimal::from_str(&format!("{x:.2}")).ok())
}

async fn validasi_relasi(state: &AppState, satuan_id: i64, kategori_id: i64) -> AppResult<()> {
    let satuan_ada = sqlx::query_scalar!("SELECT COUNT(*) FROM satuan WHERE id = $1", satuan_id)
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(0)
        > 0;
    if !satuan_ada {
        return Err(AppError::Validation("satuan_id tidak ditemukan".into()));
    }
    let kategori_ada =
        sqlx::query_scalar!("SELECT COUNT(*) FROM kategori WHERE id = $1", kategori_id)
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0)
            > 0;
    if !kategori_ada {
        return Err(AppError::Validation("kategori_id tidak ditemukan".into()));
    }
    Ok(())
}

impl ItemService {
    pub async fn create(
        state: &AppState,
        actor_id: i64,
        nama: &str,
        satuan_id: i64,
        kategori_id: i64,
        threshold_aset: Option<f64>,
    ) -> AppResult<ItemRow> {
        let nama = nama.trim();
        if nama.is_empty() {
            return Err(AppError::Validation("nama wajib diisi".into()));
        }
        if let Some(t) = threshold_aset {
            if t < 0.0 {
                return Err(AppError::Validation(
                    "threshold_aset tidak boleh negatif".into(),
                ));
            }
        }
        validasi_relasi(state, satuan_id, kategori_id).await?;

        let id = ItemRepository::create(
            &state.pool,
            nama,
            satuan_id,
            kategori_id,
            to_decimal(threshold_aset),
        )
        .await?;
        record_audit(
            &state.pool,
            actor_id,
            "items",
            id,
            "CREATE",
            None,
            Some(&json!({"nama": nama, "satuan_id": satuan_id, "kategori_id": kategori_id, "threshold_aset": threshold_aset})),
        )
        .await?;
        ItemRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    pub async fn update(
        state: &AppState,
        actor_id: i64,
        id: i64,
        nama: &str,
        satuan_id: i64,
        kategori_id: i64,
        threshold_aset: Option<f64>,
    ) -> AppResult<ItemRow> {
        let nama = nama.trim();
        if nama.is_empty() {
            return Err(AppError::Validation("nama wajib diisi".into()));
        }
        if let Some(t) = threshold_aset {
            if t < 0.0 {
                return Err(AppError::Validation(
                    "threshold_aset tidak boleh negatif".into(),
                ));
            }
        }
        validasi_relasi(state, satuan_id, kategori_id).await?;

        let before = ItemRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Item".into()))?;

        ItemRepository::update(
            &state.pool,
            id,
            nama,
            satuan_id,
            kategori_id,
            to_decimal(threshold_aset),
        )
        .await?;
        record_audit(
            &state.pool,
            actor_id,
            "items",
            id,
            "UPDATE",
            Some(&json!({"nama": before.nama, "satuan_id": before.satuan_id, "kategori_id": before.kategori_id})),
            Some(&json!({"nama": nama, "satuan_id": satuan_id, "kategori_id": kategori_id, "threshold_aset": threshold_aset})),
        )
        .await?;
        ItemRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::Internal)
    }

    pub async fn soft_delete(state: &AppState, actor_id: i64, id: i64) -> AppResult<()> {
        let before = ItemRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Item".into()))?;
        if !before.is_active {
            return Err(AppError::Conflict("Item sudah nonaktif".into()));
        }
        ItemRepository::soft_delete(&state.pool, id).await?;
        record_audit(
            &state.pool,
            actor_id,
            "items",
            id,
            "DELETE",
            Some(&json!({"nama": before.nama})),
            None,
        )
        .await?;
        Ok(())
    }
}
