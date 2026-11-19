//! Domain-domain API: auth, ruangan, kategori, satuan, item, user, config.

pub mod approval;
pub mod aset;
pub mod auditlog;
pub mod auth;
pub mod config;
pub mod dashboard;
pub mod import;
pub mod item;
pub mod kategori;
pub mod penawaran;
pub mod penerimaan;
pub mod permintaan;
pub mod po;
pub mod ruangan;
pub mod satuan;
pub mod serah_terima;
pub mod supplier;
pub mod user;

use serde::Deserialize;

/// Query pagination standar: ?page=&per_page=&q=
/// Catatan: serde_urlencoded (dipakai axum Query) TIDAK mendukung
/// `#[serde(flatten)]`, jadi field ini ditulis eksplisit di tiap query struct.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub q: Option<String>,
}

impl PaginationQuery {
    pub fn norm(self) -> (i64, i64, Option<String>) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(25).clamp(1, 100);
        (page, per_page, self.q)
    }
}

/// Helper normalisasi pagination langsung dari field query.
pub fn norm_page(page: Option<i64>, per_page: Option<i64>) -> (i64, i64) {
    (
        page.unwrap_or(1).max(1),
        per_page.unwrap_or(25).clamp(1, 100),
    )
}
