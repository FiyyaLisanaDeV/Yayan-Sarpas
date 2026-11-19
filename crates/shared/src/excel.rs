//! Helper parsing Excel rekapan yang dipakai bersama oleh seed & import.
//!
//! Ditempatkan di `shared` agar `crates/seed` (tanpa dependensi api) dan
//! `crates/api` (Import Wizard) memakai logika identik: deteksi header,
//! forward-fill ruangan, dan stop condition legenda/footer.

/// Kata kunci baris legenda (warna status) — menghentikan parsing.
pub const LEGENDA_KATA: &[&str] = &["kuning", "hijau", "putih", "merah"];
/// Kata kunci footer tanda tangan — menghentikan parsing.
pub const FOOTER_KATA: &[&str] = &["mengetahui", "menyetujui", "nip.", "kendari "];

/// Posisi kolom penting pada baris header.
#[derive(Debug, Clone, Copy)]
pub struct ColMap {
    pub ruangan: usize,
    pub nama: usize,
    pub jumlah: usize,
    pub keterangan: Option<usize>,
}

/// Rapikan spasi: tab/newline berlebih & spasi ganda → satu spasi.
pub fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cari baris header (mengandung `RUANGAN` dan sel berawalan `NAMA`).
pub fn find_header(grid: &[Vec<String>]) -> Option<(usize, ColMap)> {
    for (idx, row) in grid.iter().enumerate().take(15) {
        let upper: Vec<String> = row.iter().map(|c| c.trim().to_uppercase()).collect();
        let ruangan = upper.iter().position(|c| c == "RUANGAN");
        let nama = upper.iter().position(|c| c.starts_with("NAMA"));
        if let (Some(ruangan), Some(nama)) = (ruangan, nama) {
            let jumlah = upper
                .iter()
                .position(|c| c.starts_with("JUMLAH"))
                .unwrap_or(nama + 1);
            let keterangan = upper.iter().position(|c| c.starts_with("KETERANGAN"));
            return Some((
                idx,
                ColMap {
                    ruangan,
                    nama,
                    jumlah,
                    keterangan,
                },
            ));
        }
    }
    None
}

/// Baris legenda warna atau footer tanda tangan → hentikan parsing.
pub fn is_stop_row(row: &[String]) -> bool {
    let joined = row.join(" ").to_lowercase();
    let ada_legenda = LEGENDA_KATA.iter().any(|k| joined.contains(k)) && joined.contains(':');
    let ada_footer = FOOTER_KATA.iter().any(|k| joined.contains(k));
    ada_legenda || ada_footer
}

pub fn cell_at(row: &[String], idx: Option<usize>) -> Option<&str> {
    idx.and_then(|i| row.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

/// Nama ruangan valid: bukan angka/no urut, bukan header.
pub fn is_valid_room(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 2 {
        return false;
    }
    if t.eq_ignore_ascii_case("RUANGAN") || t.to_uppercase().starts_with("NAMA") {
        return false;
    }
    !t.trim_end_matches('.').chars().all(|c| c.is_ascii_digit())
}

/// Nama item valid: bukan angka, bukan header, cukup panjang.
pub fn is_valid_item(s: &str) -> bool {
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

/// Konversi sel calamine ke string (angka bulat tanpa `.0`).
pub fn cell_to_string(d: &calamine::Data) -> String {
    match d {
        calamine::Data::Empty => String::new(),
        calamine::Data::String(s) => s.trim().to_string(),
        calamine::Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        calamine::Data::Int(i) => i.to_string(),
        other => other.to_string().trim().to_string(),
    }
}

/// Parse teks jumlah `"10 box"` → `(Some(10.0), "BOX")`.
pub fn parse_jumlah(raw: &str) -> (Option<f64>, String) {
    let t = raw.trim().replace(',', ".");
    if t.is_empty() {
        return (None, "UNIT".to_string());
    }

    let mut angka = String::new();
    let mut sisa = String::new();
    for ch in t.chars() {
        if (ch.is_ascii_digit() || ch == '.') && sisa.trim().is_empty() {
            angka.push(ch);
        } else {
            sisa.push(ch);
        }
    }

    let nilai = angka.trim_matches('.').parse::<f64>().ok();
    (nilai, unit_to_kode(&sisa))
}

/// Map teks satuan → kode satuan yang dikenal (default `UNIT`).
pub fn unit_to_kode(unit: &str) -> String {
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

/// True bila teks satuan TIDAK dikenali (metrik laporan saja).
pub fn is_unit_tak_dikenal(raw: &str) -> bool {
    let sisa: String = raw
        .chars()
        .skip_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || c.is_whitespace())
        .collect();
    let teks = normalize_spaces(&sisa).to_lowercase();
    if teks.is_empty() {
        return false;
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

/// Kunci dedupe: hanya huruf/angka, lowercase.
pub fn canonical_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
