//! Service penawaran — validasi, penunjukan supplier, prasyarat PO (F12).

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use serde_json::json;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::record_audit;
use crate::state::AppState;

use super::repository::{ItemPenawaranRow, PenawaranRepository, PenawaranRow};

/// Input pencatatan penawaran.
pub struct PenawaranInput {
    pub supplier_id: i64,
    pub item_id: i64,
    pub harga_satuan: BigDecimal,
    pub tanggal: NaiveDate,
    pub sumber: Option<String>,
    pub catatan: Option<String>,
}

pub struct PenawaranService;

impl PenawaranService {
    pub async fn create(
        state: &AppState,
        actor_id: i64,
        input: PenawaranInput,
    ) -> AppResult<PenawaranRow> {
        let PenawaranInput {
            supplier_id,
            item_id,
            harga_satuan,
            tanggal,
            sumber,
            catatan,
        } = input;
        if harga_satuan.is_zero() || harga_satuan.sign() == bigdecimal::num_bigint::Sign::Minus {
            return Err(AppError::Validation(
                "Harga penawaran harus lebih dari 0".into(),
            ));
        }

        let supplier_ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM supplier WHERE id = $1 AND is_active")
                .bind(supplier_id)
                .fetch_one(&state.pool)
                .await?;
        if supplier_ok == 0 {
            return Err(AppError::Validation(
                "Supplier tidak ditemukan atau nonaktif".into(),
            ));
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

        let id = PenawaranRepository::create(
            &state.pool,
            crate::domains::penawaran::repository::PenawaranBaru {
                supplier_id,
                item_id,
                harga_satuan: &harga_satuan,
                tanggal,
                sumber: sumber.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                catatan: catatan.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                created_by: actor_id,
            },
        )
        .await?;

        record_audit(
            &state.pool,
            actor_id,
            "penawaran",
            id,
            "CREATE",
            None,
            Some(&json!({
                "supplier_id": supplier_id, "item_id": item_id,
                "harga_satuan": harga_satuan.to_string(),
            })),
        )
        .await?;

        PenawaranRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn update(
        state: &AppState,
        actor_id: i64,
        id: i64,
        harga_satuan: BigDecimal,
        tanggal: NaiveDate,
        sumber: Option<String>,
        catatan: Option<String>,
    ) -> AppResult<PenawaranRow> {
        let before = PenawaranRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Penawaran".into()))?;
        if before.status == "DIPILIH" {
            return Err(AppError::Conflict(
                "Penawaran yang sudah dipilih tidak dapat diubah".into(),
            ));
        }
        if harga_satuan.is_zero() || harga_satuan.sign() == bigdecimal::num_bigint::Sign::Minus {
            return Err(AppError::Validation(
                "Harga penawaran harus lebih dari 0".into(),
            ));
        }

        PenawaranRepository::update(
            &state.pool,
            id,
            &harga_satuan,
            tanggal,
            sumber.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            catatan.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        )
        .await?;

        record_audit(
            &state.pool,
            actor_id,
            "penawaran",
            id,
            "UPDATE",
            Some(&json!({"harga_satuan": before.harga_satuan.to_string()})),
            Some(&json!({"harga_satuan": harga_satuan.to_string()})),
        )
        .await?;

        PenawaranRepository::find(&state.pool, id)
            .await?
            .ok_or(AppError::Internal)
    }

    /// Tunjuk satu penawaran sebagai pemenang untuk item tersebut.
    pub async fn pilih(
        state: &AppState,
        actor_id: i64,
        penawaran_id: i64,
        alasan: Option<String>,
    ) -> AppResult<PenawaranRow> {
        let row = PenawaranRepository::find(&state.pool, penawaran_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Penawaran".into()))?;

        let mut tx = state.pool.begin().await?;
        PenawaranRepository::pilih(
            &mut tx,
            penawaran_id,
            row.item_id,
            alasan.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        )
        .await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "penawaran",
            penawaran_id,
            "PILIH",
            Some(&json!({"status": row.status})),
            Some(&json!({"status": "DIPILIH", "alasan": alasan, "supplier": row.supplier_nama})),
        )
        .await?;

        PenawaranRepository::find(&state.pool, penawaran_id)
            .await?
            .ok_or(AppError::Internal)
    }

    pub async fn delete(state: &AppState, actor_id: i64, id: i64) -> AppResult<()> {
        let row = PenawaranRepository::find(&state.pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Penawaran".into()))?;
        if row.status == "DIPILIH" {
            return Err(AppError::Conflict(
                "Penawaran yang dipilih tidak dapat dihapus — batalkan penunjukan dulu".into(),
            ));
        }
        PenawaranRepository::delete(&state.pool, id).await?;
        record_audit(&state.pool, actor_id, "penawaran", id, "DELETE", None, None).await?;
        Ok(())
    }

    pub async fn by_item(state: &AppState, item_id: i64) -> AppResult<Vec<PenawaranRow>> {
        PenawaranRepository::list_by_item(&state.pool, item_id).await
    }

    pub async fn list_items(
        state: &AppState,
        search: Option<&str>,
        only_dipilih: bool,
        page: i64,
        per_page: i64,
    ) -> AppResult<(Vec<ItemPenawaranRow>, i64)> {
        PenawaranRepository::list_items(&state.pool, search, only_dipilih, page, per_page).await
    }

    /// Prasyarat PO: item harus punya ≥ F12 penawaran dan sudah ada yang DIPILIH.
    /// Mengembalikan `(jumlah_penawaran, min_penawaran, supplier_terpilih)`.
    pub async fn cek_prasyarat_po(
        state: &AppState,
        item_id: i64,
    ) -> AppResult<(i64, i64, Option<i64>)> {
        let min_penawaran = state
            .config
            .get_i64("F12", &ConfigContext::global())
            .await
            .unwrap_or(3)
            .max(0);

        let jumlah: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM penawaran WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(&state.pool)
            .await?;

        let terpilih = PenawaranRepository::terpilih(&state.pool, item_id).await?;
        Ok((
            jumlah,
            min_penawaran,
            terpilih.map(|(supplier_id, _)| supplier_id),
        ))
    }
}
