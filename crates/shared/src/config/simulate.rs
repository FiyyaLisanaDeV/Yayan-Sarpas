//! Simulasi dampak perubahan config (§6) — untuk UI Config Center.
//! Hasil berupa jumlah record terdampak + pesan deskriptif.

use serde::Serialize;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct SimulateResult {
    pub kode: String,
    pub pesan: String,
    pub jumlah: i64,
}

/// Simulasi berdasarkan kode config yang diubah.
/// Phase 1: perhitungan berbasis data yang sudah ada (sla_tracking, po_detail, permintaan).
/// Detail per-domain disempurnakan di fase transaksi terkait.
pub async fn simulate(pool: &PgPool, kode: &str, nilai_baru: &str) -> AppResult<SimulateResult> {
    match kode {
        // SLA: "X record akan overdue dengan nilai baru"
        "C01" | "C02" | "C06" | "C07" => {
            let days: i64 = nilai_baru
                .parse()
                .map_err(|_| AppError::Validation(format!("'{nilai_baru}' bukan angka hari")))?;
            let jumlah = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM sla_tracking
                 WHERE status = 'ON_TIME'
                   AND deadline < now() + ($1::int * INTERVAL '1 day')",
                days as i32
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
            Ok(SimulateResult {
                kode: kode.to_string(),
                pesan: format!("{jumlah} record SLA akan overdue dengan nilai baru"),
                jumlah,
            })
        }
        // Threshold aset: "Y serah terima akan jadi aset"
        "G03" => {
            let threshold: f64 = nilai_baru
                .parse()
                .map_err(|_| AppError::Validation(format!("'{nilai_baru}' bukan angka")))?;
            let jumlah = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM po_detail WHERE harga_satuan >= $1::float8",
                threshold
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
            Ok(SimulateResult {
                kode: kode.to_string(),
                pesan: format!("{jumlah} baris PO dengan harga ≥ {nilai_baru} akan menjadi aset saat serah terima"),
                jumlah,
            })
        }
        // Approval chain: "Z record in-flight terdampak (snapshot tetap)"
        "B01" | "B02" | "B03" | "B04" => {
            let jumlah = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM permintaan
                 WHERE status IN ('SUBMITTED','VERIFIKASI_KASI','APPROVED_KABID','APPROVED_WADIR')"
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
            Ok(SimulateResult {
                kode: kode.to_string(),
                pesan: format!(
                    "{jumlah} permintaan in-flight terdampak (snapshot transaksi tetap)"
                ),
                jumlah,
            })
        }
        // PPN: "N PO draft akan dihitung ulang"
        "F01" => {
            let jumlah =
                sqlx::query_scalar!("SELECT COUNT(*) FROM po_header WHERE status = 'DRAFT'")
                    .fetch_one(pool)
                    .await?
                    .unwrap_or(0);
            Ok(SimulateResult {
                kode: kode.to_string(),
                pesan: format!("{jumlah} PO draft akan dihitung ulang PPN-nya"),
                jumlah,
            })
        }
        // Retensi notifikasi: "N notifikasi akan terhapus"
        "L02" => {
            let days: i64 = nilai_baru
                .parse()
                .map_err(|_| AppError::Validation(format!("'{nilai_baru}' bukan angka hari")))?;
            let jumlah = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM notifikasi
                 WHERE created_at < now() - ($1::int * INTERVAL '1 day')",
                days as i32
            )
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
            Ok(SimulateResult {
                kode: kode.to_string(),
                pesan: format!("{jumlah} notifikasi lebih tua dari {nilai_baru} hari akan dihapus"),
                jumlah,
            })
        }
        _ => Ok(SimulateResult {
            kode: kode.to_string(),
            pesan: "Tidak ada data terdampak untuk perubahan config ini".to_string(),
            jumlah: 0,
        }),
    }
}
