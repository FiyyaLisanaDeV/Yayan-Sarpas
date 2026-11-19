//! Parser Excel rekapan → permintaan historis (murni, tanpa DB).
//!
//! Memakai helper bersama `sarpras_shared::excel` (deteksi header, forward-fill
//! ruangan, stop condition legenda) sehingga identik dengan seed master.
//!
//! Berbeda dari seed master: parser ini menghasilkan **permintaan historis**
//! (ruangan + item + jumlah + satuan) untuk tabel `permintaan`.

use std::path::Path;

use calamine::Reader;

use sarpras_shared::excel::{
    cell_at, cell_to_string, find_header, is_stop_row, is_valid_item, is_valid_room,
    normalize_spaces, parse_jumlah,
};

/// Satu baris permintaan historis hasil parsing.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BarisHistoris {
    pub kategori: String,
    pub ruangan_nama: String,
    pub item_nama: String,
    pub jumlah: f64,
    pub satuan_kode: String,
    pub keterangan: Option<String>,
    pub row_no: usize,
}

/// Hasil parsing workbook.
#[derive(Debug, Default)]
pub struct ParsedHistoris {
    pub baris: Vec<BarisHistoris>,
    pub dilewati: usize,
}

/// Parse seluruh workbook (sheet ALKES + SAPRAS).
pub fn parse_workbook(path: &Path) -> Result<ParsedHistoris, Box<dyn std::error::Error>> {
    let mut workbook = calamine::open_workbook_auto(path)
        .map_err(|e| format!("gagal membuka {}: {e}", path.display()))?;

    let sheets = workbook.sheet_names().to_vec();
    let mut hasil = ParsedHistoris::default();

    for kategori in ["ALKES", "SAPRAS"] {
        let Some(actual) = sheets
            .iter()
            .find(|s| s.trim().eq_ignore_ascii_case(kategori))
            .cloned()
        else {
            continue;
        };

        let range = workbook
            .worksheet_range(&actual)
            .map_err(|e| format!("gagal membaca sheet '{actual}': {e}"))?;

        let grid: Vec<Vec<String>> = range
            .rows()
            .map(|row| row.iter().map(cell_to_string).collect())
            .collect();

        parse_grid(kategori, &grid, &mut hasil);
    }

    Ok(hasil)
}

/// Parse satu sheet ke `hasil`.
fn parse_grid(kategori: &str, grid: &[Vec<String>], hasil: &mut ParsedHistoris) {
    let Some((header_idx, cols)) = find_header(grid) else {
        return;
    };

    let mut current_room: Option<String> = None;

    for (idx, row) in grid.iter().enumerate().skip(header_idx + 1) {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        if is_stop_row(row) {
            break;
        }

        if let Some(ruangan) = cell_at(row, Some(cols.ruangan)) {
            if is_valid_room(ruangan) {
                current_room = Some(normalize_spaces(ruangan));
            }
        }

        let Some(nama) = cell_at(row, Some(cols.nama)) else {
            continue;
        };
        if !is_valid_item(nama) {
            hasil.dilewati += 1;
            continue;
        }

        let Some(ruangan_nama) = current_room.clone() else {
            hasil.dilewati += 1;
            continue;
        };

        let jumlah_raw = cell_at(row, Some(cols.jumlah)).unwrap_or_default();
        let (jumlah_opt, satuan_kode) = parse_jumlah(jumlah_raw);

        // Tanpa angka / nol = bukan permintaan nyata.
        let Some(jumlah) = jumlah_opt.filter(|j| *j > 0.0) else {
            hasil.dilewati += 1;
            continue;
        };

        hasil.baris.push(BarisHistoris {
            kategori: kategori.to_string(),
            ruangan_nama,
            item_nama: normalize_spaces(nama),
            jumlah,
            satuan_kode,
            keterangan: cell_at(row, cols.keterangan).map(normalize_spaces),
            row_no: idx + 1,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sarpras_shared::excel::parse_jumlah;

    #[test]
    fn parse_jumlah_dan_satuan() {
        assert_eq!(parse_jumlah("10 box"), (Some(10.0), "BOX".to_string()));
        assert_eq!(parse_jumlah("25 pintu"), (Some(25.0), "PINTU".to_string()));
        assert_eq!(parse_jumlah("3 set"), (Some(3.0), "SET".to_string()));
        assert_eq!(parse_jumlah("12"), (Some(12.0), "UNIT".to_string()));
        assert_eq!(parse_jumlah(""), (None, "UNIT".to_string()));
        assert_eq!(parse_jumlah("tidak ada"), (None, "UNIT".to_string()));
    }

    fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(|c| c.to_string()).collect())
            .collect()
    }

    #[test]
    fn forward_fill_ruangan_dan_stop_legenda() {
        let g = grid(&[
            &["REKAPAN"],
            &[
                "NO",
                "RUANGAN",
                "NAMA ALKES",
                "JUMLAH PERMINTAAN RUANGAN",
                "KETERANGAN",
            ],
            &[
                "1.",
                "Laboratorium Patologi Anatomi",
                "Grossing set",
                "1",
                "",
            ],
            &["", "", "Bantex", "10 box", "catatan"],
            &["2.", "Poli VCT", "Tensi meter", "2", ""],
            &["", "", "Kuning : belum ter input"],
            &["", "", "Barang setelah legenda", "9", ""],
        ]);
        let mut hasil = ParsedHistoris::default();
        parse_grid("ALKES", &g, &mut hasil);

        assert_eq!(hasil.baris.len(), 3, "legenda menghentikan parsing");
        assert_eq!(hasil.baris[0].ruangan_nama, "Laboratorium Patologi Anatomi");
        assert_eq!(hasil.baris[1].ruangan_nama, "Laboratorium Patologi Anatomi");
        assert_eq!(hasil.baris[1].satuan_kode, "BOX");
        assert_eq!(hasil.baris[1].keterangan.as_deref(), Some("catatan"));
        assert_eq!(hasil.baris[2].ruangan_nama, "Poli VCT");
    }

    #[test]
    fn baris_tanpa_jumlah_dilewati() {
        let g = grid(&[
            &["NO", "RUANGAN", "NAMA BARANG", "JUMLAH"],
            &["1.", "UPD", "Lemari", "1"],
            &["", "", "Barang tanpa jumlah", ""],
            &["", "", "Barang jumlah nol", "0"],
        ]);
        let mut hasil = ParsedHistoris::default();
        parse_grid("SAPRAS", &g, &mut hasil);

        assert_eq!(hasil.baris.len(), 1);
        assert!(hasil.dilewati >= 2);
    }
}
