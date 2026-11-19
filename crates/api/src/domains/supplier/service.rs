//! Service supplier — validasi + audit.

use bigdecimal::BigDecimal;
use serde_json::json;

use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{SupplierItemRow, SupplierRepository, SupplierRow};

/// Input pembaruan supplier.
pub struct UpdateSupplierInput {
    pub nama: String,
    pub npwp: Option<String>,
    pub kontak: Option<String>,
    pub alamat: Option<String>,
    pub is_active: bool,
}

pub struct SupplierService;

impl SupplierService {
    pub async fn create(
        state: &AppState,
        actor_id: i64,
        nama: &str,
        npwp: Option<String>,
        kontak: Option<String>,
        alamat: Option<String>,
    ) -> AppResult<SupplierRow> {
        let nama = nama.trim();
        if nama.len() < 2 {
            return Err(AppError::Validation("Nama supplier wajib diisi".into()));
        }

        let id = SupplierRepository::create(
            &state.pool,
            nama,
            npwp.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            kontak.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            alamat.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        )
        .await?;

        record_audit(
            &state.pool,
            actor_id,
            "supplier",
            id,
            "CREATE",
            None,
            Some(&json!({"nama": nama, "npwp": npwp})),
        )
        .await?;

        SupplierRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn update(
        state: &AppState,
        actor_id: i64,
        id: i64,
        input: UpdateSupplierInput,
    ) -> AppResult<SupplierRow> {
        let UpdateSupplierInput {
            nama,
            npwp,
            kontak,
            alamat,
            is_active,
        } = input;
        let nama = nama.as_str();
        let nama = nama.trim();
        if nama.len() < 2 {
            return Err(AppError::Validation("Nama supplier wajib diisi".into()));
        }
        let before = SupplierRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Supplier".into()))?;

        SupplierRepository::update(
            &state.pool,
            id,
            nama,
            npwp.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            kontak.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            alamat.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            is_active,
        )
        .await?;

        record_audit(
            &state.pool,
            actor_id,
            "supplier",
            id,
            "UPDATE",
            Some(&json!({"nama": before.nama, "is_active": before.is_active})),
            Some(&json!({"nama": nama, "is_active": is_active})),
        )
        .await?;

        SupplierRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn soft_delete(state: &AppState, actor_id: i64, id: i64) -> AppResult<()> {
        let before = SupplierRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Supplier".into()))?;
        if !before.is_active {
            return Err(AppError::Conflict("Supplier sudah nonaktif".into()));
        }

        // Guard: supplier yang sudah dipakai PO tidak boleh dinonaktifkan.
        let dipakai: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM po_header WHERE supplier_id = $1")
                .bind(id)
                .fetch_one(&state.pool)
                .await?;
        if dipakai > 0 {
            return Err(AppError::Conflict(format!(
                "Supplier sudah dipakai {dipakai} PO — tidak dapat dinonaktifkan"
            )));
        }

        SupplierRepository::soft_delete(&state.pool, id).await?;
        record_audit(
            &state.pool,
            actor_id,
            "supplier",
            id,
            "DELETE",
            Some(&json!({"nama": before.nama})),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn add_item(
        state: &AppState,
        actor_id: i64,
        supplier_id: i64,
        item_id: i64,
        harga_referensi: Option<BigDecimal>,
        catatan: Option<String>,
    ) -> AppResult<i64> {
        let supplier = SupplierRepository::find(&state.pool, supplier_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Supplier".into()))?;
        if !supplier.is_active {
            return Err(AppError::Conflict("Supplier nonaktif".into()));
        }

        let item_ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM items WHERE id = $1 AND is_active")
                .bind(item_id)
                .fetch_one(&state.pool)
                .await?;
        if item_ok == 0 {
            return Err(AppError::Validation(
                "Item tidak ditemukan atau nonaktif".into(),
            ));
        }

        if let Some(h) = &harga_referensi {
            use bigdecimal::Zero;
            if h.is_zero() || h.sign() == bigdecimal::num_bigint::Sign::Minus {
                return Err(AppError::Validation(
                    "Harga referensi harus lebih dari 0".into(),
                ));
            }
        }

        let id = SupplierRepository::add_item(
            &state.pool,
            supplier_id,
            item_id,
            harga_referensi.as_ref(),
            catatan.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        )
        .await?;

        record_audit(
            &state.pool,
            actor_id,
            "supplier_item",
            id,
            "UPSERT",
            None,
            Some(&json!({
                "supplier_id": supplier_id,
                "item_id": item_id,
                "harga_referensi": harga_referensi.map(|h| h.to_string()),
            })),
        )
        .await?;

        Ok(id)
    }

    pub async fn remove_item(state: &AppState, actor_id: i64, id: i64) -> AppResult<()> {
        let n = SupplierRepository::remove_item(&state.pool, id).await?;
        if n == 0 {
            return Err(AppError::NotFound("Katalog supplier".into()));
        }
        record_audit(
            &state.pool,
            actor_id,
            "supplier_item",
            id,
            "DELETE",
            None,
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn list_items(state: &AppState, supplier_id: i64) -> AppResult<Vec<SupplierItemRow>> {
        SupplierRepository::list_items(&state.pool, supplier_id).await
    }
}
