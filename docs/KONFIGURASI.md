# Katalog Config — 96 Parameter

> Sumber: tabel `config_item` + `config_value` live (seed `crates/seed/src/config_items.rs`).

## 1. Prinsip

- **Zero hardcode** — semua nilai bisnis ada di sini; ubah tanpa deploy.

- **Resolusi 5 lapis**: `config_snapshot` → RUANGAN → KATEGORI → GLOBAL → default.

  Setiap respons menyertakan `source` (SNAPSHOT/RUANGAN/KATEGORI/GLOBAL/DEFAULT).

- **Cache moka** TTL 5 menit, key `cfg:{kode}:{scope_type}:{scope_id}`; invalidate

  per-key saat `config_value` berubah.

- **Snapshot transaksi** mem-freeze nilai saat submit/approve/PO/dst (lihat

  `DATA-MODEL.md` §4) — histori tidak berubah.

- **12 kode kritis** tidak langsung berlaku: perubahan masuk

  `config_change_request` (PENDING) dan butuh approval role yang berwenang.

- Semua perubahan tercatat di `config_audit` (append-only).


## 2. 12 kode kritis

`B01 B02 B03 B04 C01 C02 C07 F01 F02 G03 K06 L01`

Alur: ADMIN ajukan nilai baru → approver (sesuai kolom Approver) →

APPROVED (config_value + audit + invalidate cache) / REJECTED.

## 3. Katalog


### A. Organisasi (8)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| A01 | Nama Rumah Sakit | text | `RSUD Kota Kendari` | WADIR_YANMED, ADMIN |
| A02 | Alamat Rumah Sakit | text | `Jl. Contoh No. 1, Kendari` | WADIR_YANMED, ADMIN |
| A03 | Path Logo | text | `/static/logo.png` | ADMIN |
| A04 | Template Kop Surat | template | `default_kop.html` | ADMIN |
| A05 | Format Kode Aset | text | `ALKES/{tahun}/{kode_ruangan}/{seq:04}` | ADMIN |
| A06 | Zona Waktu | enum | `Asia/Makassar` | ADMIN |
| A07 | Format Mata Uang | text | `Rp #,##0.00` | ADMIN |
| A08 | Format Tanggal | text | `dd/mm/yyyy` | ADMIN |

### B. Approval (7)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| B01 | Urutan Langkah Approval | json | `["KASI","KABID","WADIR"]` | WADIR_YANMED, ADMIN |
| B02 | Role per Langkah | json | `{"1":"KASI_SARPRAS","2":"KABID_YANMED","3":"WADIR_YANMED"}` | WADIR_YANMED, ADMIN |
| B03 | Boleh Lewati Level | boolean | `false` | WADIR_YANMED, ADMIN |
| B04 | Partial Approval Diizinkan | boolean | `true` | WADIR_YANMED, ADMIN |
| B05 | Catatan Wajib saat Reject | boolean | `true` | ADMIN |
| B06 | Delegasi Aktif | boolean | `true` | ADMIN |
| B07 | Maks Hari Delegasi | number | `14` | ADMIN |

### C. SLA (9)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| C01 | SLA Default (hari) | number | `5` | KABID_YANMED, ADMIN |
| C02 | SLA per Langkah (json) | json | `{"SUBMITTED":5,"VERIFIKASI_KASI":5,"APPROVED_KABID":5,"APPROVED_WADIR":5,"PO_APPROVAL":5}` | KABID_YANMED, ADMIN |
| C03 | Basis Hari | enum | `KERJA` | ADMIN |
| C04 | Jam Kerja Mulai | text | `08:00` | ADMIN |
| C05 | Jam Kerja Selesai | text | `16:00` | ADMIN |
| C06 | Pengingat H- (hari) | number | `1` | ADMIN |
| C07 | Eskalasi Setelah (hari) | number | `2` | KABID_YANMED, ADMIN |
| C08 | Level Eskalasi Maks | number | `2` | KABID_YANMED, ADMIN |
| C09 | Notif Overdue ke Atasan | boolean | `true` | ADMIN |

### D. Notifikasi (6)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| D01 | Channel Aktif | json | `["IN_APP"]` | ADMIN |
| D02 | Event Aktif | json | `["SUBMIT","APPROVE","REJECT","SLA_WARN","SLA_OVERDUE","PO_CREATED","BARANG_TIBA","SERAH_TERIMA","ITEM_REQUEST"]` | ADMIN |
| D03 | Penerima per Event (json) | json | `{"SUBMIT":"KASI_SARPRAS","APPROVE":"USER_RUANGAN","REJECT":"USER_RUANGAN","SLA_OVERDUE":"ATASAN"}` | ADMIN |
| D04 | Retensi Notifikasi (hari) | number | `365` | ADMIN |
| D05 | Badge Belum Dibaca Aktif | boolean | `true` | ADMIN |
| D06 | Push WebSocket Aktif | boolean | `true` | ADMIN |

### E. Permintaan (11)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| E01 | Periode Buka (bulan) | number | `1` | KABID_YANMED, ADMIN |
| E02 | Periode Buka (tanggal) | number | `1` | KABID_YANMED, ADMIN |
| E03 | Periode Tutup (bulan) | number | `12` | KABID_YANMED, ADMIN |
| E04 | Periode Tutup (tanggal) | number | `31` | KABID_YANMED, ADMIN |
| E05 | Cut-off Realisasi | text | `31/12` | KABID_YANMED, ADMIN |
| E06 | Maks Item per Permintaan | number | `0` | ADMIN |
| E07 | Maks Qty per Item | number | `0` | ADMIN |
| E08 | Lampiran Wajib | boolean | `false` | ADMIN |
| E09 | Urgent di Luar Periode | boolean | `false` | KABID_YANMED, ADMIN |
| E10 | Batas Edit Setelah Submit | enum | `SEBELUM_VERIFIKASI` | ADMIN |
| E11 | Batas Batal Setelah Submit | enum | `SEBELUM_APPROVAL_WADIR` | ADMIN |

### F. PO (12)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| F01 | PPN (%) | decimal | `11.00` | KABAG_KEUANGAN, ADMIN |
| F02 | Threshold Approval Kabag | decimal | `0` | KABAG_KEUANGAN, ADMIN |
| F03 | Mode Pengelompokan PO | enum | `SUPPLIER` | KABID_YANMED, ADMIN |
| F04 | Partial Receipt Diizinkan | boolean | `true` | ADMIN |
| F05 | Toleransi Qty (%) | decimal | `0.00` | ADMIN |
| F06 | Mode Retur Barang Reject | enum | `MANUAL` | ADMIN |
| F07 | Dokumen Wajib Terima | json | `["DO","INVOICE","FAKTUR_PAJAK"]` | KABAG_KEUANGAN, ADMIN |
| F08 | Auto Close PO Penuh | boolean | `true` | ADMIN |
| F09 | Alert PO Belum Diterima (hari) | number | `45` | ADMIN |
| F10 | Alert Final Belum PO (hari) | number | `30` | ADMIN |
| F11 | Format Nomor PO | text | `PO/{kategori}/{tahun}/{seq:04}` | ADMIN |
| F12 | Minimal Penawaran per Item | number | `3` | KABID_YANMED, ADMIN |

### G. Serah Terima (7)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| G01 | Konfirmasi Ruangan Wajib | boolean | `true` | ADMIN |
| G02 | Batas Hari Konfirmasi | number | `3` | ADMIN |
| G03 | Threshold Nilai Aset | decimal | `1000000.00` | KABAG_KEUANGAN, ADMIN |
| G04 | Auto Draft Aset | boolean | `true` | ADMIN |
| G05 | Format QR Label | text | `kode_aset` | nama|ruangan|ADMIN |
| G06 | Mutasi Butuh Approval | boolean | `true` | WADIR_YANMED, ADMIN |
| G07 | Hapus Aset Butuh Approval | boolean | `true` | WADIR_YANMED, ADMIN |

### H. Offline (7)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| H01 | Mode Offline Aktif | boolean | `true` | ADMIN |
| H02 | Cakupan Offline | json | `["DRAFT_PERMINTAAN"]` | ADMIN |
| H03 | Submit/Approval Online Only | boolean | `true` | ADMIN |
| H04 | Interval Sync (detik) | number | `60` | ADMIN |
| H05 | Kebijakan Konflik | json | `{"DRAFT":"LWW","APPROVAL":"MANUAL_REVIEW"}` | ADMIN |
| H06 | Maks Antrian Offline | number | `100` | ADMIN |
| H07 | Notif Sync Gagal | boolean | `true` | ADMIN |

### I. Import (4)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| I01 | Maks Baris per Import | number | `10000` | ADMIN |
| I02 | Fuzzy Match Threshold | number | `80` | ADMIN |
| I03 | Transaksi Atomic Commit | boolean | `true` | ADMIN |
| I04 | Simpan Raw JSON | boolean | `true` | ADMIN |

### J. Cetak (6)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| J01 | Watermark Draft | boolean | `true` | ADMIN |
| J02 | Log Cetak Aktif | boolean | `true` | ADMIN |
| J03 | Placeholder TTD Elektronik | boolean | `true` | ADMIN |
| J04 | Format Nomor BA Serah | text | `BAST/{tahun}/{seq:04}` | ADMIN |
| J05 | Format Nomor Penerimaan | text | `BAP/{tahun}/{seq:04}` | ADMIN |
| J06 | Format Export Default | enum | `XLSX` | ADMIN |

### K. Keamanan (8)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| K01 | JWT Access (menit) | number | `15` | ADMIN |
| K02 | JWT Refresh (jam) | number | `8` | ADMIN |
| K03 | Panjang Password Minimum | number | `8` | ADMIN |
| K04 | Password Butuh Kompleksitas | boolean | `true` | ADMIN |
| K05 | Maks Login per 15 Menit | number | `5` | ADMIN |
| K06 | MFA Wajib Role | json | `["WADIR_YANMED","KABAG_KEUANGAN"]` | WADIR_YANMED, ADMIN |
| K07 | Session Idle (menit) | number | `30` | ADMIN |
| K08 | Jam Akses | text | `00:00-23:59` | ADMIN |

### L. Retensi (5)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| L01 | Retensi Audit Trail | enum | `PERMANEN` | WADIR_YANMED, ADMIN |
| L02 | Retensi Notifikasi (hari) | number | `365` | ADMIN |
| L03 | Retensi Log Cetak | enum | `PERMANEN` | ADMIN |
| L04 | Retensi Backup (hari) | number | `365` | ADMIN |
| L05 | Alert Storage (%) | number | `80` | ADMIN |

### M. UI (6)

| Kode | Label | Tipe | Default | Approver |
|---|---|---|---|---|
| M01 | Bahasa | enum | `id` | ADMIN |
| M02 | Warna Status (json) | json | `{"KUNING":"#FFC107","HIJAU":"#4CAF50","PUTIH":"#FFFFFF","MERAH":"#F44336"}` | ADMIN |
| M03 | Pagination Default | number | `25` | ADMIN |
| M04 | Warna Tema Utama | color | `#1E88E5` | ADMIN |
| M05 | Path Favicon | text | `/static/favicon.ico` | ADMIN |
| M06 | Tampilkan Tahun Aktif | boolean | `true` | ADMIN |

## 4. Cara mengubah config

### Via UI (Config Center)
1. Login sebagai ADMIN → **Config Center** → pilih grup → klik **Ubah** pada parameter.
2. Isi nilai baru — dialog menampilkan **simulasi dampak** otomatis
   (mis. berapa record SLA akan overdue).
3. Non-kritis → **Simpan** langsung berlaku. Kritis → **Ajukan Perubahan**
   (masuk PENDING), lalu approver memproses di tab **Persetujuan**.

### Via API
```bash
# baca nilai efektif + sumbernya
curl -s $BASE/config/values/C01 -H "Authorization: Bearer $TOKEN"

# ubah non-kritis
curl -s -X PUT $BASE/config/values/A01 -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"nilai":"RSUD Baru","alasan":"SK 2026"}'

# ubah kritis → PENDING, lalu approver setujui
curl -s -X PUT $BASE/config/values/G03 -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"nilai":"2000000.00","alasan":"inflasi"}'
curl -s -X POST $BASE/config/change-requests/1/approve -H "Authorization: Bearer $TOKEN"
```

## 5. Tipe & validasi nilai

| Tipe | Validasi |
|---|---|
| number / duration_days | integer, min/max dari `validation_json` (default 0..999999) |
| decimal | angka, min (default 0) |
| boolean | `true`/`false` |
| enum | harus ada di `options` |
| json | JSON valid |
| text | maks `max_length` (default 500) |
| template | maks 20000 |
| color | `#RRGGBB` |
| email / url | format dasar |
| date | `YYYY-MM-DD` |

## 6. Pengaturan scope RUANGAN / KATEGORI

Skema mendukung `config_value` dengan `scope_type = RUANGAN|KATEGORI` +
`scope_id`. Resolusi otomatis memilih nilai paling spesifik. Saat ini semua
nilai berada pada scope GLOBAL; endpoint `GET /config/values/:kode?ruangan_id=`
sudah menghormati scope lebih spesifik bila barisnya ditambahkan via SQL/admin.
