//! Handler dashboard — agregasi permintaan, realisasi PO, aset.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use sqlx::Row;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::AppError;
use sarpras_shared::rbac::{ensure, Role};

use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/rekap", get(rekap))
        .route("/realisasi", get(realisasi))
}

#[derive(Debug, Deserialize)]
struct TahunQuery {
    tahun: Option<i16>,
}

fn tahun_atau_sekarang(t: Option<i16>) -> i16 {
    t.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2026)
    })
}

/// Dashboard P1: rekap permintaan per status/kategori/ruangan.
async fn rekap(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<TahunQuery>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    // USER_RUANGAN hanya melihat ruangannya.
    let filter_ruangan = if actor.role == Role::UserRuangan {
        actor.ruangan_id
    } else {
        None
    };
    let tahun = tahun_atau_sekarang(q.tahun);

    let per_status = sqlx::query(
        r#"SELECT status, COUNT(*) AS jumlah, COALESCE(SUM(jumlah),0) AS qty
           FROM permintaan
           WHERE tahun = $1 AND ($2::bigint IS NULL OR ruangan_id = $2)
           GROUP BY status ORDER BY status"#,
    )
    .bind(tahun)
    .bind(filter_ruangan)
    .fetch_all(&state.pool)
    .await?;

    let per_kategori = sqlx::query(
        r#"SELECT k.kode AS kategori, COUNT(*) AS jumlah
           FROM permintaan p
           JOIN items i ON i.id = p.item_id
           JOIN kategori k ON k.id = i.kategori_id
           WHERE p.tahun = $1 AND ($2::bigint IS NULL OR p.ruangan_id = $2)
           GROUP BY k.kode ORDER BY k.kode"#,
    )
    .bind(tahun)
    .bind(filter_ruangan)
    .fetch_all(&state.pool)
    .await?;

    let per_ruangan = sqlx::query(
        r#"SELECT r.nama AS ruangan, COUNT(*) AS jumlah
           FROM permintaan p JOIN ruangan r ON r.id = p.ruangan_id
           WHERE p.tahun = $1 AND ($2::bigint IS NULL OR p.ruangan_id = $2)
           GROUP BY r.nama ORDER BY COUNT(*) DESC LIMIT 15"#,
    )
    .bind(tahun)
    .bind(filter_ruangan)
    .fetch_all(&state.pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM permintaan WHERE tahun = $1 AND ($2::bigint IS NULL OR ruangan_id = $2)",
    )
    .bind(tahun)
    .bind(filter_ruangan)
    .fetch_one(&state.pool)
    .await?;

    Ok(ApiResponse::ok(serde_json::json!({
        "tahun": tahun,
        "total_permintaan": total,
        "per_status": per_status.iter().map(|r| serde_json::json!({
            "status": r.get::<String, _>("status"),
            "jumlah": r.get::<i64, _>("jumlah"),
            "qty": r.get::<bigdecimal::BigDecimal, _>("qty"),
        })).collect::<Vec<_>>(),
        "per_kategori": per_kategori.iter().map(|r| serde_json::json!({
            "kategori": r.get::<String, _>("kategori"),
            "jumlah": r.get::<i64, _>("jumlah"),
        })).collect::<Vec<_>>(),
        "per_ruangan": per_ruangan.iter().map(|r| serde_json::json!({
            "ruangan": r.get::<String, _>("ruangan"),
            "jumlah": r.get::<i64, _>("jumlah"),
        })).collect::<Vec<_>>(),
    })))
}

/// Dashboard P2: realisasi PO → penerimaan → serah terima → aset.
async fn realisasi(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<TahunQuery>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    ensure(
        actor.role,
        &[
            Role::Admin,
            Role::KabagKeuangan,
            Role::WadirYanmed,
            Role::KasiSarpras,
            Role::Auditor,
        ],
    )?;
    let tahun = tahun_atau_sekarang(q.tahun);

    let po_per_status = sqlx::query(
        r#"SELECT status, COUNT(*) AS jumlah, COALESCE(SUM(nilai_total),0) AS nilai
           FROM po_header WHERE tahun = $1 GROUP BY status ORDER BY status"#,
    )
    .bind(tahun)
    .fetch_all(&state.pool)
    .await?;

    let total_po: bigdecimal::BigDecimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(nilai_total),0) FROM po_header WHERE tahun = $1 AND status <> 'DRAFT'",
    )
    .bind(tahun)
    .fetch_one(&state.pool)
    .await?;

    let nilai_terima: bigdecimal::BigDecimal = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(pn.qty_terima * d.harga_satuan), 0)
           FROM penerimaan pn
           JOIN po_detail d ON d.id = pn.po_detail_id
           JOIN po_header ph ON ph.id = d.po_header_id
           WHERE ph.tahun = $1"#,
    )
    .bind(tahun)
    .fetch_one(&state.pool)
    .await?;

    let nilai_serah: bigdecimal::BigDecimal = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(st.qty_serah * d.harga_satuan), 0)
           FROM serah_terima st
           JOIN penerimaan pn ON pn.id = st.penerimaan_id
           JOIN po_detail d ON d.id = pn.po_detail_id
           JOIN po_header ph ON ph.id = d.po_header_id
           WHERE ph.tahun = $1 AND st.status_konfirmasi = 'CONFIRMED'"#,
    )
    .bind(tahun)
    .fetch_one(&state.pool)
    .await?;

    let jumlah_aset: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aset_tetap WHERE EXTRACT(YEAR FROM tgl_perolehan) = $1",
    )
    .bind(tahun)
    .fetch_one(&state.pool)
    .await?;

    let nilai_aset: bigdecimal::BigDecimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(nilai),0) FROM aset_tetap WHERE EXTRACT(YEAR FROM tgl_perolehan) = $1",
    )
    .bind(tahun)
    .fetch_one(&state.pool)
    .await?;

    let supplier_top = sqlx::query(
        r#"SELECT s.nama, COUNT(p.id) AS jumlah_po, COALESCE(SUM(p.nilai_total),0) AS nilai
           FROM po_header p JOIN supplier s ON s.id = p.supplier_id
           WHERE p.tahun = $1
           GROUP BY s.nama ORDER BY SUM(p.nilai_total) DESC NULLS LAST LIMIT 10"#,
    )
    .bind(tahun)
    .fetch_all(&state.pool)
    .await?;

    let menunggu_po: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM permintaan WHERE status = 'FINAL'")
            .fetch_one(&state.pool)
            .await?;

    Ok(ApiResponse::ok(serde_json::json!({
        "tahun": tahun,
        "po_per_status": po_per_status.iter().map(|r| serde_json::json!({
            "status": r.get::<String, _>("status"),
            "jumlah": r.get::<i64, _>("jumlah"),
            "nilai": r.get::<bigdecimal::BigDecimal, _>("nilai"),
        })).collect::<Vec<_>>(),
        "nilai_po": total_po,
        "nilai_terima": nilai_terima,
        "nilai_serah": nilai_serah,
        "jumlah_aset": jumlah_aset,
        "nilai_aset": nilai_aset,
        "permintaan_final_belum_po": menunggu_po,
        "supplier_top": supplier_top.iter().map(|r| serde_json::json!({
            "supplier": r.get::<String, _>("nama"),
            "jumlah_po": r.get::<i64, _>("jumlah_po"),
            "nilai": r.get::<bigdecimal::BigDecimal, _>("nilai"),
        })).collect::<Vec<_>>(),
    })))
}
