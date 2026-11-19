//! Import master ruangan & item dari Excel "REKAPAN ALKES DAN SAPRAS".
//!
//! Format sumber (bukan asumsi generik):
//! - Sheet `ALKES` dan `SAPRAS`.
//! - Kolom: `NO | RUANGAN | NAMA ALKES/BARANG | JUMLAH PERMINTAAN RUANGAN | KETERANGAN`.
//! - Kolom RUANGAN hanya terisi di baris pertama tiap blok → forward-fill.
//! - Satuan menempel pada teks jumlah (`10 box`, `1 unit`, `8 Batang`) → di-parse.
//! - Baris legenda warna & footer tanda tangan → stop condition.
//!
//! Jika file tidak ditemukan atau gagal di-parse, seed TIDAK abort:
//! dipakai data placeholder agar pengembangan tetap berjalan.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use calamine::{Data, Reader};
use sqlx::PgPool;
use tracing::{info, warn};

/// Kandidat lokasi file Excel, dicoba berurutan.
const EXCEL_CANDIDATES: &[&str] = &[
    "seed-data/ruangan_items.xlsx",
    "seed-data/REKAPAN ALKES DAN SAPRAS  2026 - Copy.xlsx",
];

/// Kata kunci baris legenda (warna status) — menghentikan parsing.
const LEGENDA_KATA: &[&str] = &["kuning", "hijau", "putih", "merah"];
/// Kata kunci footer tanda tangan — menghentikan parsing.
const FOOTER_KATA: &[&str] = &["mengetahui", "menyetujui", "nip.", "kendari "];
/// Prefiks nama ruangan yang dibuang saat membentuk kode.
const PREFIX_KODE: &[&str] = &["POLI", "RUANG", "INSTALASI", "DEPO"];

/// Hasil satu kali impor — dipakai untuk logging ringkas.
#[derive(Debug, Default)]
pub struct ImportReport {
    pub sumber: String,
    pub ruangan_baru: u32,
    pub item_baru: u32,
    pub duplikat: u32,
    pub dilewati: u32,
    pub satuan_default: u32,
    pub placeholder_dihapus: u32,
}

/// Definisi ruangan hasil parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomDef {
    pub kode: String,
    pub nama: String,
}

/// Definisi item hasil parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemDef {
    pub nama: String,
    pub kategori: String,
    pub satuan: String,
}

/// Hasil parsing workbook (murni, tanpa DB).
#[derive(Debug, Default)]
pub struct ParsedMaster {
    pub rooms: Vec<RoomDef>,
    pub items: Vec<ItemDef>,
    pub dilewati: u32,
    pub duplikat: u32,
    pub satuan_default: u32,
}

// ---------------------------------------------------------------- entry point

/// Import ruangan & item ke DB.
/// Bila Excel tidak ada / gagal dibaca, jatuh ke data placeholder.
pub async fn import_ruangan_items(
    pool: &PgPool,
) -> Result<ImportReport, Box<dyn std::error::Error>> {
    let Some(path) = locate_excel() else {
        warn!(
            "File Excel rekapan tidak ditemukan (dicari: {}). Memakai data PLACEHOLDER.",
            EXCEL_CANDIDATES.join(", ")
        );
        let report = seed_placeholder(pool).await?;
        return Ok(report);
    };

    let parsed = match parse_workbook(&path) {
        Ok(p) => p,
        Err(e) => {
            // Seed tidak boleh abort hanya karena format Excel berubah.
            warn!("Gagal mem-parse Excel ({e}). Memakai data PLACEHOLDER.");
            return seed_placeholder(pool).await;
        }
    };

    if parsed.rooms.is_empty() || parsed.items.is_empty() {
        warn!(
            "Excel terbaca tetapi tidak menghasilkan data (ruangan={}, item={}). Memakai PLACEHOLDER.",
            parsed.rooms.len(),
            parsed.items.len()
        );
        return seed_placeholder(pool).await;
    }

    info!(
        "Excel ter-parse: {} ruangan, {} item (duplikat dilewati: {}, baris dilewati: {})",
        parsed.rooms.len(),
        parsed.items.len(),
        parsed.duplikat,
        parsed.dilewati
    );

    write_master(pool, &path, parsed).await
}

/// Cari file Excel: kandidat eksplisit → glob `REKAPAN*.xlsx` di seed-data/ dan root.
fn locate_excel() -> Option<PathBuf> {
    for candidate in EXCEL_CANDIDATES {
        let p = Path::new(candidate);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }

    let mut dirs = vec![PathBuf::from("seed-data"), PathBuf::from(".")];
    // Juga coba direktori kerja induk agar skrip bisa dijalankan dari crate.
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("..").join(".."));
    }

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut found: Vec<PathBuf> = entries
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
    None
}

// ------------------------------------------------------------------- parsing

/// Parse seluruh workbook menjadi definisi ruangan & item.
pub fn parse_workbook(path: &Path) -> Result<ParsedMaster, Box<dyn std::error::Error>> {
    let mut workbook = calamine::open_workbook_auto(path)
        .map_err(|e| format!("gagal membuka {}: {e}", path.display()))?;

    let sheets = workbook.sheet_names().to_vec();
    let mut master = ParsedMaster::default();
    let mut room_keys: HashMap<String, String> = HashMap::new(); // canonical key → kode
    let mut room_codes: HashSet<String> = HashSet::new();
    let mut item_keys: HashSet<(String, String)> = HashSet::new();

    for (kategori, sheet_name) in [("ALKES", "ALKES"), ("SAPRAS", "SAPRAS")] {
        let Some(actual) = sheets
            .iter()
            .find(|s| s.trim().eq_ignore_ascii_case(sheet_name))
            .cloned()
        else {
            warn!("Sheet '{sheet_name}' tidak ada — dilewati");
            continue;
        };

        let range = workbook
            .worksheet_range(&actual)
            .map_err(|e| format!("gagal membaca sheet '{actual}': {e}"))?;

        let grid: Vec<Vec<String>> = range
            .rows()
            .map(|row| row.iter().map(cell_to_string).collect())
            .collect();

        let hasil = parse_grid(kategori, &grid);
        master.dilewati += hasil.dilewati;
        master.duplikat += hasil.duplikat;
        master.satuan_default += hasil.satuan_default;

        for (nama_ruangan, nama_item, satuan) in hasil.rows {
            // Daftarkan ruangan sekali (varian nama apa pun dianggap sama).
            let key = canonical_key(&nama_ruangan);
            if let std::collections::hash_map::Entry::Vacant(slot) = room_keys.entry(key) {
                let kode = unique_room_code(&nama_ruangan, &mut room_codes);
                slot.insert(kode.clone());
                master.rooms.push(RoomDef {
                    kode,
                    nama: nama_ruangan.clone(),
                });
            }

            let item_key = (
                normalize_spaces(&nama_item).to_lowercase(),
                kategori.to_string(),
            );
            if !item_keys.insert(item_key) {
                master.duplikat += 1;
                continue;
            }
            master.items.push(ItemDef {
                nama: normalize_spaces(&nama_item),
                kategori: kategori.to_string(),
                satuan,
            });
        }
    }

    master.rooms.sort_by(|a, b| a.kode.cmp(&b.kode));
    Ok(master)
}

/// Hasil parsing satu sheet.
#[derive(Debug, Default)]
struct GridResult {
    rows: Vec<(String, String, String)>, // (ruangan, item, satuan_kode)
    dilewati: u32,
    duplikat: u32,
    satuan_default: u32,
}

/// Parse grid satu sheet: deteksi header, forward-fill ruangan, stop di legenda.
fn parse_grid(kategori: &str, grid: &[Vec<String>]) -> GridResult {
    let mut result = GridResult::default();

    let Some((header_idx, cols)) = find_header(grid) else {
        warn!("Header (RUANGAN + NAMA) tidak ditemukan pada sheet {kategori}");
        return result;
    };

    let mut current_room: Option<String> = None;

    for row in grid.iter().skip(header_idx + 1) {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }

        if is_stop_row(row) {
            break;
        }

        if let Some(ruangan) = cell_at(row, cols.ruangan) {
            if is_valid_room(ruangan) {
                current_room = Some(normalize_spaces(ruangan));
            }
        }

        let Some(nama) = cell_at(row, cols.nama) else {
            continue;
        };
        if !is_valid_item(nama) {
            result.dilewati += 1;
            continue;
        }

        let Some(ruangan) = current_room.clone() else {
            // Item tanpa konteks ruangan (mis. baris nyasar) — dilewati.
            result.dilewati += 1;
            continue;
        };

        let jumlah_raw = cell_at(row, cols.jumlah).unwrap_or_default();
        let (_, satuan) = parse_jumlah(jumlah_raw);
        if is_unit_tak_dikenal(jumlah_raw) {
            result.satuan_default += 1;
        }

        result.rows.push((ruangan, normalize_spaces(nama), satuan));
    }

    result
}

#[derive(Debug, Clone, Copy)]
struct ColMap {
    ruangan: usize,
    nama: usize,
    jumlah: usize,
}

/// Cari baris header (mengandung `RUANGAN` dan sel berawalan `NAMA`).
fn find_header(grid: &[Vec<String>]) -> Option<(usize, ColMap)> {
    for (idx, row) in grid.iter().enumerate().take(15) {
        let upper: Vec<String> = row.iter().map(|c| c.trim().to_uppercase()).collect();
        let ruangan = upper.iter().position(|c| c == "RUANGAN");
        let nama = upper.iter().position(|c| c.starts_with("NAMA"));
        if let (Some(ruangan), Some(nama)) = (ruangan, nama) {
            let jumlah = upper
                .iter()
                .position(|c| c.starts_with("JUMLAH"))
                .unwrap_or(nama + 1);
            return Some((
                idx,
                ColMap {
                    ruangan,
                    nama,
                    jumlah,
                },
            ));
        }
    }
    None
}

/// Baris legenda warna atau footer tanda tangan → hentikan parsing.
fn is_stop_row(row: &[String]) -> bool {
    let joined = row.join(" ").to_lowercase();
    let ada_legenda = LEGENDA_KATA.iter().any(|k| joined.contains(k)) && joined.contains(':');
    let ada_footer = FOOTER_KATA.iter().any(|k| joined.contains(k));
    ada_legenda || ada_footer
}

fn cell_at(row: &[String], idx: usize) -> Option<&str> {
    row.get(idx).map(|s| s.trim()).filter(|s| !s.is_empty())
}

/// Nama ruangan valid: bukan angka/no urut, bukan header.
fn is_valid_room(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 2 {
        return false;
    }
    if t.eq_ignore_ascii_case("RUANGAN") || t.to_uppercase().starts_with("NAMA") {
        return false;
    }
    // nomor urut seperti "1." atau "12"
    !t.trim_end_matches('.').chars().all(|c| c.is_ascii_digit())
}

/// Nama item valid: bukan angka, bukan header, cukup panjang.
fn is_valid_item(s: &str) -> bool {
    let t = normalize_spaces(s);
    if t.len() < 2 {
        return false;
    }
    if t.chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        return false;
    }
    let up = t.to_uppercase();
    if up.starts_with("NAMA ") || up == "RUANGAN" {
        return false;
    }
    let low = t.to_lowercase();
    if LEGENDA_KATA.iter().any(|k| low.starts_with(k))
        || FOOTER_KATA.iter().any(|k| low.contains(k))
    {
        return false;
    }
    true
}

fn cell_to_string(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            // Excel menyimpan angka sebagai float; tampilkan tanpa ".0" bila bulat.
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        other => other.to_string().trim().to_string(),
    }
}

/// Rapikan spasi: tab/newline berlebih & spasi ganda → satu spasi.
pub fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Kunci dedupe: hanya huruf/angka, lowercase.
pub fn canonical_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

// ------------------------------------------------------- jumlah & satuan

/// Parse teks jumlah `"10 box"` → `(Some(10.0), "BOX")`.
/// Bila angka tidak ada → `(None, "UNIT")`.
pub fn parse_jumlah(raw: &str) -> (Option<f64>, String) {
    let t = raw.trim().replace(',', ".");
    if t.is_empty() {
        return (None, "UNIT".to_string());
    }

    let mut angka = String::new();
    let mut sisa = String::new();
    for ch in t.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            if sisa.trim().is_empty() {
                angka.push(ch);
            } else {
                sisa.push(ch);
            }
        } else {
            sisa.push(ch);
        }
    }

    let nilai = angka.trim_matches('.').parse::<f64>().ok();
    let satuan = unit_to_kode(sisa.trim());
    (nilai, satuan)
}

/// True bila teks satuan TIDAK dikenali (sehingga jatuh ke UNIT).
/// Dipakai hanya untuk metrik laporan, bukan logika penyimpanan.
fn is_unit_tak_dikenal(raw: &str) -> bool {
    let sisa: String = raw
        .chars()
        .skip_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || c.is_whitespace())
        .collect();
    let teks = normalize_spaces(&sisa).to_lowercase();
    if teks.is_empty() {
        return false; // memang tanpa satuan — normal, bukan "tak dikenal"
    }
    let kandidat = teks.split_whitespace().next().unwrap_or("");
    !matches!(
        kandidat,
        "box"
            | "boxes"
            | "unit"
            | "pc"
            | "pcs"
            | "buah"
            | "set"
            | "batang"
            | "dos"
            | "lembar"
            | "kaleng"
            | "botol"
            | "pintu"
    )
}

/// Map teks satuan → kode satuan yang dikenal.
/// Satuan tak dikenal jatuh ke `UNIT` (item tidak boleh hilang hanya
/// karena penulisan satuan di Excel berbeda).
fn unit_to_kode(unit: &str) -> String {
    let u = normalize_spaces(unit).to_lowercase();
    let kandidat = u.split_whitespace().next().unwrap_or("");

    for teks in [u.as_str(), kandidat] {
        let kode = match teks {
            "box" | "boxes" => "BOX",
            "unit" => "UNIT",
            "pc" | "pcs" => "PCS",
            "buah" => "BUAH",
            "set" => "SET",
            "batang" => "BATANG",
            "dos" => "DOS",
            "lembar" => "LEMBAR",
            "kaleng" => "KALENG",
            "botol" => "BOTOL",
            "pintu" => "PINTU",
            _ => continue,
        };
        return kode.to_string();
    }

    "UNIT".to_string()
}

// --------------------------------------------------------- kode ruangan

/// Bentuk kode ruangan unik & deterministik dari nama.
pub fn unique_room_code(nama: &str, used: &mut HashSet<String>) -> String {
    let base = room_code(nama);
    if used.insert(base.clone()) {
        return base;
    }
    let mut n = 2;
    loop {
        let kandidat = format!("{base}-{n}");
        if used.insert(kandidat.clone()) {
            return kandidat;
        }
        n += 1;
    }
}

/// Kode dasar: inisial kata bermakna; kurung sebagai sufiks.
pub fn room_code(nama: &str) -> String {
    let norm = normalize_spaces(nama);
    let (main, parent) = split_parenthetical(&norm);

    let mut words = kata_bermakna(&main);
    // Buang prefiks jenis ruangan selama masih ada kata lain.
    while words.len() > 1 && PREFIX_KODE.contains(&words[0].to_uppercase().as_str()) {
        words.remove(0);
    }

    let mut kode = if words.len() >= 2 {
        let inisial: String = words.iter().filter_map(|w| w.chars().next()).collect();
        let inisial = inisial.to_uppercase();
        if inisial.chars().count() >= 2 {
            inisial
        } else {
            truncate_upper(&words[0], 12)
        }
    } else if let Some(w) = words.first() {
        truncate_upper(w, 12)
    } else {
        "RUANG".to_string()
    };

    if let Some(p) = parent {
        let pw = kata_bermakna(&p);
        if !pw.is_empty() {
            let suf: String = pw.iter().filter_map(|w| w.chars().next()).collect();
            let suf = suf.to_uppercase();
            if !suf.is_empty() {
                kode = format!("{kode}-{suf}");
            }
        }
    }

    kode
}

/// Pisahkan `"Instalasi farmasi (DEPO PMCC)"` → `("Instalasi farmasi", Some("DEPO PMCC"))`.
fn split_parenthetical(s: &str) -> (String, Option<String>) {
    if let (Some(open), Some(close)) = (s.find('('), s.rfind(')')) {
        if open < close {
            let main = s[..open].trim().to_string();
            let parent = s[open + 1..close].trim().to_string();
            return (main, (!parent.is_empty()).then_some(parent));
        }
    }
    (s.to_string(), None)
}

/// Kata bermakna: alfanumerik saja, panjang ≥ 2.
fn kata_bermakna(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| w.chars().count() >= 2)
        .collect()
}

fn truncate_upper(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .take(max)
        .collect::<String>()
        .to_uppercase()
}

// ------------------------------------------------------------- tulis ke DB

async fn write_master(
    pool: &PgPool,
    path: &Path,
    parsed: ParsedMaster,
) -> Result<ImportReport, Box<dyn std::error::Error>> {
    let mut report = ImportReport {
        sumber: path.display().to_string(),
        ..Default::default()
    };

    // Lookup kategori & satuan sekali.
    let kategori: HashMap<String, i64> = sqlx::query!("SELECT id, kode FROM kategori")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| (r.kode, r.id))
        .collect();

    let satuan: HashMap<String, i64> = sqlx::query!("SELECT id, kode FROM satuan")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| (r.kode, r.id))
        .collect();

    for room in &parsed.rooms {
        let inserted = sqlx::query!(
            "INSERT INTO ruangan (kode, nama) VALUES ($1, $2) ON CONFLICT (kode) DO NOTHING",
            room.kode,
            room.nama
        )
        .execute(pool)
        .await?
        .rows_affected();
        report.ruangan_baru += inserted as u32;
    }

    for item in &parsed.items {
        let Some(&kategori_id) = kategori.get(&item.kategori) else {
            warn!(
                "kategori '{}' tidak ada — item '{}' dilewati",
                item.kategori, item.nama
            );
            report.dilewati += 1;
            continue;
        };
        let satuan_id = satuan
            .get(&item.satuan)
            .copied()
            .or_else(|| satuan.get("UNIT").copied());
        let Some(satuan_id) = satuan_id else {
            warn!("satuan tidak tersedia — item '{}' dilewati", item.nama);
            report.dilewati += 1;
            continue;
        };

        let inserted = sqlx::query!(
            "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ($1, $2, $3)
             ON CONFLICT (nama, kategori_id) DO NOTHING",
            item.nama,
            satuan_id,
            kategori_id
        )
        .execute(pool)
        .await?
        .rows_affected();
        report.item_baru += inserted as u32;
    }

    report.duplikat = parsed.duplikat;
    report.dilewati += parsed.dilewati;
    report.satuan_default = parsed.satuan_default;

    // Hapus placeholder lama HANYA setelah import sukses, dengan guard FK.
    let hapus_item = sqlx::query!(
        "DELETE FROM items
         WHERE nama LIKE '%(PLACEHOLDER)%'
           AND NOT EXISTS (SELECT 1 FROM permintaan p WHERE p.item_id = items.id)"
    )
    .execute(pool)
    .await?
    .rows_affected();

    let hapus_ruangan = sqlx::query!(
        "DELETE FROM ruangan
         WHERE nama LIKE '%(PLACEHOLDER)%'
           AND NOT EXISTS (SELECT 1 FROM users u WHERE u.ruangan_id = ruangan.id)
           AND NOT EXISTS (SELECT 1 FROM permintaan p WHERE p.ruangan_id = ruangan.id)"
    )
    .execute(pool)
    .await?
    .rows_affected();

    report.placeholder_dihapus = (hapus_item + hapus_ruangan) as u32;

    Ok(report)
}

/// Data placeholder fiktif — dipakai bila Excel tidak tersedia/rusak.
async fn seed_placeholder(pool: &PgPool) -> Result<ImportReport, Box<dyn std::error::Error>> {
    let ruangan: &[(&str, &str)] = &[
        ("UGD", "Unit Gawat Darurat (PLACEHOLDER)"),
        ("IRNA", "Instalasi Rawat Inap (PLACEHOLDER)"),
        ("ICU", "Intensive Care Unit (PLACEHOLDER)"),
        ("LAB", "Laboratorium (PLACEHOLDER)"),
        ("FARMASI", "Farmasi (PLACEHOLDER)"),
    ];

    let items: &[(&str, &str, &str)] = &[
        ("Tensimeter Digital (PLACEHOLDER)", "UNIT", "ALKES"),
        ("Termometer Infrared (PLACEHOLDER)", "PCS", "ALKES"),
        ("Infus Set (PLACEHOLDER)", "SET", "ALKES"),
        ("AC 1 PK (PLACEHOLDER)", "UNIT", "SAPRAS"),
        ("Kursi Roda (PLACEHOLDER)", "UNIT", "SAPRAS"),
        ("Brankar (PLACEHOLDER)", "UNIT", "ALKES"),
        ("Meja Kerja (PLACEHOLDER)", "BUAH", "SAPRAS"),
        ("Handsanitizer (PLACEHOLDER)", "BOTOL", "ALKES"),
        ("Sarung Tangan Medis (PLACEHOLDER)", "BOX", "ALKES"),
        ("Printer Laserjet (PLACEHOLDER)", "UNIT", "SAPRAS"),
    ];

    let mut report = ImportReport {
        sumber: "(placeholder — Excel tidak tersedia)".to_string(),
        ..Default::default()
    };

    for (kode, nama) in ruangan {
        report.ruangan_baru += sqlx::query!(
            "INSERT INTO ruangan (kode, nama) VALUES ($1, $2) ON CONFLICT (kode) DO NOTHING",
            kode,
            nama
        )
        .execute(pool)
        .await?
        .rows_affected() as u32;
    }

    for (nama, satuan, kategori) in items {
        let satuan_id: Option<i64> = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = $1")
            .bind(satuan)
            .fetch_optional(pool)
            .await?;
        let kategori_id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM kategori WHERE kode = $1")
                .bind(kategori)
                .fetch_optional(pool)
                .await?;
        if let (Some(s), Some(k)) = (satuan_id, kategori_id) {
            report.item_baru += sqlx::query!(
                "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ($1, $2, $3)
                 ON CONFLICT (nama, kategori_id) DO NOTHING",
                nama,
                s,
                k
            )
            .execute(pool)
            .await?
            .rows_affected() as u32;
        }
    }

    Ok(report)
}

// ------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(|c| c.to_string()).collect())
            .collect()
    }

    #[test]
    fn parse_jumlah_mengenali_satuan() {
        assert_eq!(parse_jumlah("10 box"), (Some(10.0), "BOX".to_string()));
        assert_eq!(parse_jumlah("1 unit"), (Some(1.0), "UNIT".to_string()));
        assert_eq!(parse_jumlah("5 pcs"), (Some(5.0), "PCS".to_string()));
        assert_eq!(parse_jumlah("2 buah"), (Some(2.0), "BUAH".to_string()));
        assert_eq!(parse_jumlah("3 set"), (Some(3.0), "SET".to_string()));
        assert_eq!(parse_jumlah("8 Batang"), (Some(8.0), "BATANG".to_string()));
        assert_eq!(parse_jumlah("1 Dos"), (Some(1.0), "DOS".to_string()));
        assert_eq!(parse_jumlah("2 Lembar"), (Some(2.0), "LEMBAR".to_string()));
        assert_eq!(parse_jumlah("1 Kaleng"), (Some(1.0), "KALENG".to_string()));
        assert_eq!(parse_jumlah("1 Botol"), (Some(1.0), "BOTOL".to_string()));
        assert_eq!(parse_jumlah("25 pintu"), (Some(25.0), "PINTU".to_string()));
        assert_eq!(parse_jumlah("12"), (Some(12.0), "UNIT".to_string()));
        assert_eq!(parse_jumlah(""), (None, "UNIT".to_string()));
        assert_eq!(parse_jumlah("tidak ada"), (None, "UNIT".to_string()));
    }

    #[test]
    fn ruangan_forward_fill_dan_stop_di_legenda() {
        let g = grid(&[
            &["REKAPAN PERMINTAAN ALKES 2026"],
            &[
                "NO",
                "RUANGAN",
                "NAMA ALKES",
                "JUMLAH PERMINTAAN RUANGAN",
                "KETERANGAN",
            ],
            &["1.", "Laboratorium Patologi Anatomi", "Grossing set", "1"],
            &["", "", "Manual staining jar", "1"],
            &["2.", "Poli VCT", "Tensi meter", "1"],
            &["", "", "Kuning : belum ter input di link"],
            &["", "", "Barang setelah legenda", "9"],
        ]);
        let r = parse_grid("ALKES", &g);

        assert_eq!(r.rows.len(), 3, "baris setelah legenda harus diabaikan");
        assert_eq!(
            r.rows[0],
            (
                "Laboratorium Patologi Anatomi".into(),
                "Grossing set".into(),
                "UNIT".into()
            )
        );
        assert_eq!(
            r.rows[1].0, "Laboratorium Patologi Anatomi",
            "ruangan harus forward-fill"
        );
        assert_eq!(r.rows[2].0, "Poli VCT");
    }

    #[test]
    fn footer_tanda_tangan_menghentikan_parsing() {
        let g = grid(&[
            &["NO", "RUANGAN", "NAMA BARANG", "JUMLAH"],
            &["1.", "UPD", "Lemari", "1"],
            &["", "", "Mengetahui,"],
            &["", "", "NIP. 19830202"],
            &["", "UPD", "Barang Setelah Footer", "1"],
        ]);
        let r = parse_grid("SAPRAS", &g);
        assert_eq!(r.rows.len(), 1);
    }

    #[test]
    fn item_invalid_dilewati() {
        let g = grid(&[
            &["NO", "RUANGAN", "NAMA ALKES", "JUMLAH"],
            &["1.", "UPD", "Lemari", "1"],
            &["", "", "123", "1"],
            &["", "", "  ", "1"],
        ]);
        let r = parse_grid("ALKES", &g);
        assert_eq!(r.rows.len(), 1);
        assert!(r.dilewati >= 1);
    }

    #[test]
    fn kode_ruangan_unik_dan_terbaca() {
        assert_eq!(room_code("Laboratorium Patologi Anatomi"), "LPA");
        assert_eq!(room_code("Laboratorium Patologi Klinik"), "LPK");
        assert_eq!(room_code("Poli VCT"), "VCT");
        assert_eq!(room_code("POLI GIGI"), "GIGI");
        assert_eq!(room_code("POLI GIZI"), "GIZI");
        assert_eq!(room_code("UPD"), "UPD");
        assert_eq!(room_code("OKUPASI TERAPI"), "OT");
        assert_eq!(room_code("DEPO RAWAT JALAN"), "RJ");
        assert_eq!(room_code("Instalasi farmasi (DEPO PMCC)"), "FARMASI-DP");
        assert_eq!(room_code("Instalasi farmasi (DEPO OK)"), "FARMASI-DO");
        assert_eq!(room_code("Bedah Thorax ( \tCathlab )"), "BT-C");
        assert_eq!(room_code("Bedah Thorax (Kardiak )"), "BT-K");
        assert_eq!(room_code("dr. Akbar ( IBS)"), "DA-I");
    }

    #[test]
    fn varian_nama_dokter_menyatu() {
        assert_eq!(
            canonical_key("dr. Akbar ( IBS)"),
            canonical_key("dr.Akbar(IBS)")
        );
        assert_eq!(
            canonical_key("dr. Akbar ( IBS)"),
            canonical_key("dr.akbar ( IBS)")
        );
        assert_ne!(canonical_key("dr Akbar"), canonical_key("dr Tomy"));
    }

    #[test]
    fn tabrakan_kode_diberi_sufiks() {
        let mut used = HashSet::new();
        assert_eq!(unique_room_code("Poli Gigi", &mut used), "GIGI");
        let kedua = unique_room_code("Gigi", &mut used);
        assert_eq!(kedua, "GIGI-2");
        let ketiga = unique_room_code("GIGI", &mut used);
        assert_eq!(ketiga, "GIGI-3");
    }

    #[test]
    fn normalize_spaces_merapikan_tab() {
        assert_eq!(
            normalize_spaces("  Bedah   Thorax ( \tCathlab ) "),
            "Bedah Thorax ( Cathlab )"
        );
    }
}
