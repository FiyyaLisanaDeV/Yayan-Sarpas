//! Service import — pencocokan nama Excel → master, lalu simpan permintaan historis.

use std::collections::HashMap;

use bigdecimal::BigDecimal;
use uuid::Uuid;

use sarpras_shared::config::ConfigContext;
use sarpras_shared::error::{AppError, AppResult};
use sarpras_shared::excel::canonical_key;

use crate::middleware::record_audit;
use crate::state::AppState;

use super::parser::{parse_workbook, BarisHistoris};
use super::repository::ImportRepository;

/// Ringkasan hasil satu kali import.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportReport {
    pub batch_id: i64,
    pub file_path: String,
    pub total_row: i32,
    pub sukses: i32,
    pub gagal: i32,
    /// Baris identik yang sudah pernah diimpor (dilewati, bukan error).
    pub duplikat: i32,
    pub dilewati_parser: usize,
    pub ruangan_tidak_cocok: i32,
    pub item_tidak_cocok: i32,
}

pub struct ImportService;

/// Lookup master yang dipakai berulang saat memproses baris.
struct MasterLookup {
    ruangan: HashMap<String, (i64, String)>,
    item: HashMap<(String, String), (i64, String, i64)>,
    satuan: HashMap<String, i64>,
}

impl ImportService {
    /// Cari file Excel rekapan di lokasi standar.
    pub fn locate_excel() -> Option<std::path::PathBuf> {
        let kandidat = [
            "seed-data/ruangan_items.xlsx",
            "REKAPAN ALKES DAN SAPRAS  2026 - Copy.xlsx",
        ];
        for k in kandidat {
            let p = std::path::Path::new(k);
            if p.is_file() {
                return Some(p.to_path_buf());
            }
        }
        // Cari `REKAPAN*.xlsx` di root dan seed-data.
        for dir in ["seed-data", "."] {
            if let Ok(entries) = std::fs::read_dir(dir) {
                let mut found: Vec<std::path::PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file()
                            && p.extension()
                                .and_then(|x| x.to_str())
                                .map(|x| x.eq_ignore_ascii_case("xlsx"))
                                .unwrap_or(false)
                            && p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|n| n.to_uppercase().starts_with("REKAPAN"))
                                .unwrap_or(false)
                    })
                    .collect();
                found.sort();
                if let Some(p) = found.into_iter().next() {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Import dari file Excel ke `permintaan` historis.
    pub async fn import_excel(
        state: &AppState,
        actor_id: i64,
        file_path: Option<String>,
        tahun: i16,
    ) -> AppResult<ImportReport> {
        let path = match file_path.as_deref() {
            Some(p) => std::path::PathBuf::from(p),
            None => Self::locate_excel().ok_or_else(|| {
                AppError::NotFound(
                    "File Excel rekapan tidak ditemukan (letakkan REKAPAN*.xlsx di root)".into(),
                )
            })?,
        };

        if !path.is_file() {
            return Err(AppError::NotFound(format!(
                "File '{}' tidak ditemukan",
                path.display()
            )));
        }

        // Batas baris per import dari config I01.
        let maks_baris = state
            .config
            .get_i64("I01", &ConfigContext::global())
            .await
            .unwrap_or(10_000) as usize;
        let simpan_raw = state
            .config
            .get_bool("I04", &ConfigContext::global())
            .await
            .unwrap_or(true);

        let parsed = parse_workbook(&path)
            .map_err(|e| AppError::Validation(format!("Gagal mem-parse Excel: {e}")))?;

        if parsed.baris.len() > maks_baris {
            return Err(AppError::Validation(format!(
                "Baris melebihi batas {} (config I01); ditemukan {}",
                maks_baris,
                parsed.baris.len()
            )));
        }

        // Lookup master sekali (hindari N+1).
        let mut ruangan_kanonik: HashMap<String, (i64, String)> = HashMap::new();
        for (id, nama) in ImportRepository::ruangan_map(&state.pool).await? {
            ruangan_kanonik.insert(canonical_key(&nama), (id, nama));
        }

        let mut item_kanonik: HashMap<(String, String), (i64, String, i64)> = HashMap::new();
        for (id, nama, kategori, satuan_id) in ImportRepository::item_map(&state.pool).await? {
            item_kanonik.insert((canonical_key(&nama), kategori), (id, nama, satuan_id));
        }

        let satuan_map: HashMap<String, i64> = ImportRepository::satuan_map(&state.pool)
            .await?
            .into_iter()
            .collect();

        let lookup = MasterLookup {
            ruangan: ruangan_kanonik,
            item: item_kanonik,
            satuan: satuan_map,
        };

        let batch_uuid = Uuid::new_v4();
        let mut tx = state.pool.begin().await?;
        let batch_id = ImportRepository::create_batch(
            &mut tx,
            "PERMINTAAN_HISTORIS",
            &path.display().to_string(),
            parsed.baris.len() as i32,
            actor_id,
        )
        .await?;

        let mut sukses = 0i32;
        let mut gagal = 0i32;
        let mut duplikat = 0i32;
        let mut ruangan_tidak_cocok = 0i32;
        let mut item_tidak_cocok = 0i32;

        for b in &parsed.baris {
            let raw_json = if simpan_raw {
                serde_json::to_value(b).ok()
            } else {
                None
            };

            let hasil = Self::proses_baris(&mut tx, &batch_uuid, b, tahun, actor_id, &lookup).await;

            match hasil {
                Ok(()) => {
                    sukses += 1;
                    ImportRepository::log_row(
                        &mut tx,
                        batch_id,
                        b.row_no as i32,
                        "SUKSES",
                        None,
                        raw_json.as_ref(),
                    )
                    .await?;
                }
                Err(pesan) if pesan.starts_with("duplikat:") => {
                    duplikat += 1;
                    ImportRepository::log_row(
                        &mut tx,
                        batch_id,
                        b.row_no as i32,
                        "DUPLIKAT",
                        Some(&pesan),
                        raw_json.as_ref(),
                    )
                    .await?;
                }
                Err(pesan) => {
                    gagal += 1;
                    if pesan.contains("ruangan") {
                        ruangan_tidak_cocok += 1;
                    }
                    if pesan.contains("item") {
                        item_tidak_cocok += 1;
                    }
                    ImportRepository::log_row(
                        &mut tx,
                        batch_id,
                        b.row_no as i32,
                        "GAGAL",
                        Some(&pesan),
                        raw_json.as_ref(),
                    )
                    .await?;
                }
            }
        }

        ImportRepository::update_batch_counts(&mut tx, batch_id, sukses, gagal).await?;
        tx.commit().await?;

        record_audit(
            &state.pool,
            actor_id,
            "import_batch",
            batch_id,
            "IMPORT",
            None,
            Some(&serde_json::json!({
                "tipe": "PERMINTAAN_HISTORIS",
                "file": path.display().to_string(),
                "sukses": sukses,
                "gagal": gagal,
                "tahun": tahun,
            })),
        )
        .await?;

        Ok(ImportReport {
            batch_id,
            file_path: path.display().to_string(),
            total_row: parsed.baris.len() as i32,
            sukses,
            gagal,
            dilewati_parser: parsed.dilewati,
            duplikat,
            ruangan_tidak_cocok,
            item_tidak_cocok,
        })
    }

    /// Proses satu baris di dalam transaksi; error = pesan alasannya.
    async fn proses_baris(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        batch_uuid: &Uuid,
        b: &BarisHistoris,
        tahun: i16,
        actor_id: i64,
        lookup: &MasterLookup,
    ) -> Result<(), String> {
        let (ruangan_id, _nama_ruangan) = lookup
            .ruangan
            .get(&canonical_key(&b.ruangan_nama))
            .ok_or_else(|| format!("ruangan '{}' tidak ditemukan di master", b.ruangan_nama))?;

        let key = (canonical_key(&b.item_nama), b.kategori.clone());
        let (item_id, _nama_item, satuan_default) = lookup.item.get(&key).ok_or_else(|| {
            format!(
                "item '{}' ({}) tidak ditemukan di master",
                b.item_nama, b.kategori
            )
        })?;

        // Satuan: dari teks Excel bila dikenal, else satuan default item.
        let satuan_id = lookup
            .satuan
            .get(&b.satuan_kode)
            .copied()
            .unwrap_or(*satuan_default);

        let jumlah = BigDecimal::try_from(b.jumlah)
            .map_err(|_| format!("jumlah '{}' tidak valid", b.jumlah))?;

        // Idempotensi: baris identik yang sudah pernah diimpor dilewati.
        if ImportRepository::historis_sudah_ada(
            tx,
            tahun,
            *ruangan_id,
            *item_id,
            &jumlah,
            satuan_id,
        )
        .await
        .map_err(|e| format!("gagal memeriksa duplikat: {e}"))?
        {
            return Err("duplikat: baris ini sudah pernah diimpor".to_string());
        }

        ImportRepository::insert_historis(
            tx,
            *batch_uuid,
            tahun,
            *ruangan_id,
            *item_id,
            &jumlah,
            satuan_id,
            b.keterangan.as_deref(),
            actor_id,
        )
        .await
        .map_err(|e| format!("gagal menyimpan: {e}"))?;

        Ok(())
    }

    /// Pratinjau tanpa menyimpan: berapa yang akan cocok / tidak.
    pub async fn preview(
        state: &AppState,
        file_path: Option<String>,
    ) -> AppResult<serde_json::Value> {
        let path = match file_path.as_deref() {
            Some(p) => std::path::PathBuf::from(p),
            None => Self::locate_excel()
                .ok_or_else(|| AppError::NotFound("File Excel rekapan tidak ditemukan".into()))?,
        };

        let parsed = parse_workbook(&path)
            .map_err(|e| AppError::Validation(format!("Gagal mem-parse Excel: {e}")))?;

        let ruangan_rows = ImportRepository::ruangan_map(&state.pool).await?;
        let ruangan_kanonik: HashMap<String, String> = ruangan_rows
            .into_iter()
            .map(|(_, nama)| (canonical_key(&nama), nama))
            .collect();

        let item_rows = ImportRepository::item_map(&state.pool).await?;
        let item_kanonik: HashMap<(String, String), String> = item_rows
            .into_iter()
            .map(|(_, nama, kategori, _)| ((canonical_key(&nama), kategori), nama))
            .collect();

        let mut cocok = 0usize;
        let mut ruangan_hilang: HashMap<String, usize> = HashMap::new();
        let mut item_hilang: HashMap<String, usize> = HashMap::new();

        for b in &parsed.baris {
            let ok_ruangan = ruangan_kanonik.contains_key(&canonical_key(&b.ruangan_nama));
            let ok_item =
                item_kanonik.contains_key(&(canonical_key(&b.item_nama), b.kategori.clone()));
            if ok_ruangan && ok_item {
                cocok += 1;
            } else {
                if !ok_ruangan {
                    *ruangan_hilang.entry(b.ruangan_nama.clone()).or_default() += 1;
                }
                if !ok_item {
                    *item_hilang
                        .entry(format!("{} ({})", b.item_nama, b.kategori))
                        .or_default() += 1;
                }
            }
        }

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "total_baris": parsed.baris.len(),
            "cocok": cocok,
            "tidak_cocok": parsed.baris.len() - cocok,
            "dilewati_parser": parsed.dilewati,
            "ruangan_tidak_ditemukan": ruangan_hilang,
            "item_tidak_ditemukan": item_hilang,
        }))
    }
}
