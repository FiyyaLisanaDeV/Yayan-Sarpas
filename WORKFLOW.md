# Alur Kerja Aplikasi — SARPRAS & ALKES

> Dokumen ini adalah **acuan tunggal alur kerja aplikasi** dari permintaan
> sampai realisasi anggaran. Setiap fase pengembangan mengacu ke tahap di sini.
> Diperbarui setiap kali ada keputusan alur baru.

---

## 1. Alur End-to-End

```
┌────────────┐   submit    ┌──────────────┐  setuju   ┌──────────────┐  setuju   ┌──────────────┐  setuju   ┌───────┐
│ USER_RUANGAN│───────────▶│ VERIFIKASI   │──────────▶│ APPROVED     │──────────▶│ APPROVED     │──────────▶│ FINAL │
│ Form        │            │ _KASI        │           │ _KABID       │           │ _WADIR       │           │       │
│ Permintaan  │            │ (step 1)     │           │ (step 2)     │           │ (step 3)     │           │       │
│ (DRAFT)     │            └──────┬───────┘           └──────────────┘           └──────────────┘           └───┬───┘
└─────────────┘                   │ reject (catatan wajib, B05)                                                │
     ▲                            ▼                                                                             │
     └────────────────── RETURNED ─┐ (kembali ke DRAFT, approval_log + audit)                                   │
                                   │                                                                             ▼
                                   │   ┌───────────────────────────────────────────────────────────────────────────────────────────┐
                                   │   │ PERSIAPAN PENGADAAN (KASI_SARPRAS)                                                       │
                                   │   │ 1. Katalog Supplier-Item — referensi: supplier mana menjual item apa + harga referensi    │
                                   │   │ 2. Penawaran Harga — minta ≥ N penawaran per item dari supplier berbeda (F12)             │
                                   │   │ 3. Perbandingan Harga — tabel antar supplier → PENUNJUKAN SUPPLIER (pilih + alasan)        │
                                   │   └───────────────────────────────────────────────────────────────────────────────────────────┘
                                   │                                                                             ▼
                                   │                                                                   ┌──────────────────┐
                                   │                                                                   │ PO (DRAFT)       │
                                   │    KASI_SARPRAS grouping per SUPPLIER (F03), nomor F11, PPN F01   │ supplier + detail│
                                   │                                                                   │ (penawaran DIPILIH│
                                   │                                                                   │  → harga_satuan) │
                                   │                                                                   └────────┬─────────┘
                                                                                                                │ approve KABAG_KEUANGAN (threshold F02)
                                                                                                        reject  │
                                                                                                          ▼     ▼
                                                                                                    ┌───────────────┐  kirim  ┌────────┐
                                                                                                    │ APPROVED      │────────▶│ SENT   │
                                                                                                    └───────────────┘         └───┬────┘
                                                                                                                                  │ barang tiba
                                                                                                                                  ▼
                                                            ┌───────────────────────────────────────────────────────────────────────────┐
                                                            │ PENERIMAAN (PETUGAS_GUDANG)                                             │
                                                            │ partial diizinkan (F04), toleransi qty (F05), dokumen wajib (F07):        │
                                                            │ DO / INVOICE / FAKTUR_PAJAK (upload lampiran)                            │
                                                            └───────────────────────────────────────────────────────────────────────────┘
                                                                                                                                  │ qty penuh → PO CLOSED (F08)
                                                                                                                                  ▼
                                            ┌───────────────────────────────────────────────────────────────────────────────────────────────┐
                                            │ SERAH TERIMA (gudang → ruangan)                                                             │
                                            │ BAST (nomor J04), konfirmasi ruangan wajib (G01), batas hari (G02)                          │
                                            │ PENDING ──konfirmasi──▶ CONFIRMED    ──dispute──▶ DISPUTED ──resolve Kasi──▶ CONFIRMED       │
                                            └───────────────────────────────────────────────────────────────────────────────────────────────┘
                                                                                                                                  │ nilai ≥ threshold G03 (atau item.threshold_aset)
                                                                                                                                  ▼
                                            ┌───────────────────────────────────────────────────────────────────────────────────────────────┐
                                            │ REGISTRASI ASET (ADMIN_ASET)                                                                │
                                            │ kode aset (A05), label QR (G05, qrcode) → REGISTERED → IN_USE → MAINTAINED → DISPOSED         │
                                            │ mutasi antar ruangan (approval WADIR bila G06), hapus aset (approval WADIR bila G07)           │
                                            └───────────────────────────────────────────────────────────────────────────────────────────────┘
                                                                                                                                  │
                                                                                                                                  ▼
                                            ┌───────────────────────────────────────────────────────────────────────────────────────────────┐
                                            │ REALISASI ANGGARAN & PELAPORAN                                                              │
                                            │ Dashboard P1 (permintaan per ruangan/kategori/status) + Dashboard P2 (realisasi PO/penerimaan/│
                                            │ aset), Laporan Permintaan, Stok Opname, Audit Log — semua list/detail bisa Cetak PDF + Export  │
                                            │ Excel (J06), log cetak (J02)                                                                 │
                                            └───────────────────────────────────────────────────────────────────────────────────────────────┘

SEMUA transisi menulis approval_log + audit_trail, dan mem-freeze config ke config_snapshot (§8).
```

## 2. Peran & Kewenangan

| Role | Tanggung jawab inti | Screen utama |
|---|---|---|
| USER_RUANGAN | Buat/draft permintaan ruangan sendiri; konfirmasi serah terima | Form Permintaan, My Requests |
| KASI_SARPRAS | Verifikasi (step 1), kelola supplier & katalog, penawaran + perbandingan + penunjukan supplier, buat PO + serah terima, resolve dispute | Master Supplier, Penawaran Harga, Buat PO, Serah Terima |
| KABID_YANMED | Approve step 2 | Inbox Approval |
| WADIR_YANMED | Approve final step 3; approval mutasi/hapus aset (G06/G07) | Inbox Approval |
| PETUGAS_GUDANG | Input penerimaan, serah terima, stok opname | Penerimaan, Serah Terima, Stok Opname |
| KABAG_KEUANGAN | Approve PO (threshold F02), lihat realisasi | Approval PO, Dashboard Realisasi |
| ADMIN_ASET | Registrasi aset, mutasi, label QR | Registrasi Aset, Mutasi Aset |
| ADMIN | Semua + Config Center | Config Center, Master |
| AUDITOR | Read-only + audit trail | Audit Log |

## 3. Workflow CRUD per Modul — Integrasi Antar Modul

### 3.0 Komponen CRUD (definisi)

| Operasi | Arti | SQL | Aturan di aplikasi ini |
|---|---|---|---|
| **C — Create** | Menambahkan/memasukkan data baru ke basis data | `INSERT INTO` | Validasi referensi (FK wajib ada & aktif) + tulis `audit_trail` aksi CREATE |
| **R — Read** | Mengambil, mencari, atau menampilkan data yang sudah ada | `SELECT` | Semua role terautentikasi (kecuali modul dibatasi role); list selalu berpaginasi |
| **U — Update** | Mengubah/memodifikasi data yang tersimpan | `UPDATE` | `audit_trail` UPDATE (before/after) + optimistic lock `updated_at` |
| **D — Delete** | Menghapus data dari basis data | `DELETE FROM` | Master memakai **soft delete** (`is_active`); hard delete dilarang untuk transaksi/riwayat |

Prinsip umum yang berlaku untuk SEMUA modul:

- **Transaksi alur** (permintaan, PO, penawaran, serah terima) TIDAK diedit bebas — hanya lewat transisi state machine yang tercatat di `approval_log`.

### 3.1 Rantai Integrasi (referensi antar modul)

```
ruangan ──┬── users.ruangan_id
          ├── permintaan.ruangan_id ──── approval_log.permintaan_id
          ├── serah_terima.ruangan_id
          ├── aset_tetap.ruangan_id ──── mutasi_aset.aset_id
          └── stok_opname.ruangan_id

kategori ──── items.kategori_id
satuan  ──┬── items.satuan_id
          └── po_detail.satuan_id

items ──┬── permintaan.item_id
        ├── supplier_item.item_id ──── supplier
        ├── penawaran.item_id ──────── supplier
        ├── po_detail.item_id ──────── po_header ──── supplier
        ├── aset_tetap.item_id
        └── stok_opname.item_id

alur transaksi:
permintaan(FINAL) ──▶ po_detail ──▶ penerimaan ──▶ serah_terima ──▶ aset_tetap
penawaran(DIPILIH) ──▶ po_detail.harga_satuan (referensi harga)
notifikasi ──▶ users · audit_trail ──▶ users
```

### 3.2 CRUD Modul Master & Referensi

| # | Modul | Role | C | U | D | Dipakai oleh (integrasi) | Guard saat U/D |
|---|---|---|---|---|---|---|---|
| 2 | Master Ruangan | ADMIN | ✓ | ✓ | soft | users, permintaan, serah_terima, aset_tetap, stok_opname | Tolak nonaktif bila masih dipakai user aktif / permintaan aktif |
| 3 | Master Item + Satuan | ADMIN, KASI | ✓ | ✓ | soft | permintaan, supplier_item, penawaran, po_detail, aset_tetap, stok_opname | Item nonaktif tidak bisa dipilih di permintaan baru; kode satuan unik |
| 4 | Master User + Role | ADMIN | ✓ | ✓ | soft | created_by/actor di seluruh transaksi & audit | Admin aktif terakhir tak boleh nonaktif; password policy K03/K04; reset password teraudit |
| 11 | Master Supplier | ADMIN, KASI | ✓ | ✓ | soft | supplier_item, penawaran, po_header | Supplier nonaktif tak bisa dipilih di PO/penawaran |
| 11b | Katalog Supplier-Item | ADMIN, KASI | ✓ | ✓ | soft | dasar validasi "supplier mana menjual item apa" | Hanya supplier & item aktif |
| 23 | Hari Libur | ADMIN | ✓ | ✓ | ✓ | kalkulasi SLA basis KERJA (C03) | Tanggal unik |

### 3.3 CRUD Modul Transaksi & Alur

| # | Modul | Actor | Aksi (bukan CRUD bebas) | Efek samping |
|---|---|---|---|---|
| 1 | Login | semua | login/refresh/logout (rate limit K05) | rotasi refresh token; session |
| 5 | Form Permintaan | USER_RUANGAN | C draft multi-row → U (saat DRAFT / ≤ E10) → submit | submit: status SUBMITTED + snapshot C01,C02,E11,F11,G03,J04 + notif SUBMIT |
| 6 | My Requests | USER_RUANGAN | R milik ruangan sendiri; U/D sesuai E10/E11 | lihat riwayat approval_log |
| 7 | Inbox Approval | KASI → KABID → WADIR | approve (step+1) / reject (catatan wajib B05) | approval_log + notif APPROVE/REJECT + sla_tracking + snapshot B01,B02,C01 |
| 11c | Penawaran Harga | KASI | C/U/D saat DIAJUKAN | snapshot F01,F03,F11; prasyarat PO |
| 11d | Perbandingan & Penunjukan | KASI | U: pilih DIPILIH + alasan | penawaran DIPILIH → harga masuk PO; audit |
| 12 | Buat PO | KASI | C dari permintaan FINAL (grouping F03); U/D saat DRAFT | permintaan → IN_PO; snapshot F01,F03,F11; notif PO_CREATED |
| 13 | Approval PO | KABAG_KEUANGAN | approve → SENT / reject → DRAFT | threshold F02; notif |
| 15 | Penerimaan | PETUGAS_GUDANG | C per po_detail; U koreksi (audit) | qty penuh → PO CLOSED (F08); notif BARANG_TIBA; snapshot F04,F05,F07 |
| 16 | Serah Terima | PETUGAS_GUDANG (C), USER_RUANGAN (konfirmasi/dispute), KASI (resolve) | PENDING → CONFIRMED/DISPUTED → CONFIRMED | snapshot G01,G02,G03,G05; notif SERAH_TERIMA; permintaan HANDED_OVER |
| 17 | Registrasi Aset + QR | ADMIN_ASET | C/R/U; D = dispose | kode aset A05, QR (G05), snapshot A05,G03,G05 |
| 18 | Mutasi Aset | ADMIN_ASET | C; approval WADIR bila G06 | mutasi_aset + audit |
| 20 | Stok Opname | PETUGAS_GUDANG | C/R | selisih fisik vs sistem tercatat |
| 26 | Import Wizard | ADMIN | C batch + R log | import_batch/row_log; permintaan historis + snapshot |

### 3.4 CRUD Modul Laporan & Read-only

| # | Modul | Role | Aksi |
|---|---|---|---|
| 8 | Dashboard Rekap | semua | R (agregat) |
| 9 | Laporan Permintaan | semua | R + Cetak PDF/Export Excel (log J02) |
| 10 | Audit Log | ADMIN, AUDITOR | R (append-only) |
| 14 | Detail PO + PDF | pihak terkait | R + cetak (print_log) |
| 19 | Dashboard Realisasi | KABAG + semua | R |
| 22 | Config Audit | semua | R (append-only) |
| 25 | Notifikasi Center | semua | R + U(is_read); WebSocket realtime |

### 3.5 CRUD Modul Config & Approval

| # | Modul | Actor | Aksi | Efek |
|---|---|---|---|---|
| 21 | Config Center | ADMIN | R + U | Non-kritis langsung; 12 kode kritis → `config_change_request` PENDING |
| 24 | Config Approval | approver (WADIR/KABID/KABAG/ADMIN) | approve / reject | config_audit + invalidate cache moka |

## 4. Tahapan Alur — Detail

### 4.1 Permintaan Barang — USER_RUANGAN
- **Screen**: Form Permintaan (multi-row, kategori ALKES/SAPRAS), My Requests.
- **Input**: tahun, item (master), jumlah, satuan, keterangan, lampiran (wajib bila E08).
- **Aturan**: periode buka/tutup (E01–E04), urgent di luar periode (E09), maks item/qty (E06/E07), usul item baru → `request_item` (Kasi setujui → jadi `items`).
- **Transisi**: DRAFT → SUBMITTED. **Snapshot saat submit**: C01, C02, E11, F11, G03, J04.
- **Notifikasi**: SUBMIT → KASI_SARPRAS (D03).

### 4.2 Approval Berjenjang — KASI → KABID → WADIR
- **Screen**: Inbox Approval (list milik role), detail dengan riwayat `approval_log`.
- **Urutan & role per step**: dari config B01/B02 (default 3 langkah: KASI, KABID, WADIR).
- **Aturan**: skip level (B03), partial approval (B04), **catatan wajib saat reject** (B05), delegasi (B06/B07).
- **Transisi per step**: SUBMITTED → VERIFIKASI_KASI → APPROVED_KABID → APPROVED_WADIR → FINAL. Reject di step mana pun → RETURNED (kembali DRAFT + catatan). Batas edit/batal setelah submit: E10/E11.
- **Snapshot saat approve**: B01, B02, C01.
- **Notifikasi**: APPROVE → USER_RUANGAN; REJECT → USER_RUANGAN.
- **SLA**: setiap step punya deadline (C02); eskalasi C07/C08 → atasan (C09).

### 4.3 Manajemen Supplier, Pengadaan & PO — KASI_SARPRAS (+ KABAG_KEUANGAN)

Alur pengadaan dibagi 4 sub-tahap: **master & katalog → penawaran → perbandingan & penunjukan → PO**.

#### 4.3.1 Master Supplier & Katalog Supplier-Item — KASI_SARPRAS
- **Screen**: Master Supplier (CRUD), tab Katalog Item per supplier.
- **Input supplier**: nama, npwp, kontak, alamat (soft delete `is_active`).
- **Input katalog** (`supplier_item`): item apa saja yang disediakan supplier, harga referensi, catatan.
- **Aturan**: hanya supplier ber-katalog yang bisa dipilih di PO; katalog audit-able.

#### 4.3.2 Penawaran Harga — KASI_SARPRAS
- **Screen**: Penawaran Harga (per item, multi-supplier).
- **Input** (`penawaran`): supplier, item, harga satuan, tanggal penawaran, sumber (surat/WA/lisan), catatan.
- **Aturan**: untuk tiap item yang akan di-PO-kan, wajib ≥ `F12` penawaran dari supplier berbeda sebelum boleh masuk PO.
- **Snapshot saat catat penawaran**: F01, F03, F11.

#### 4.3.3 Perbandingan Harga & Penunjukan Supplier — KASI_SARPRAS
- **Screen**: Perbandingan Harga — tabel per item: supplier × harga × tanggal × sumber.
- **Tindakan**: Kasi menunjuk supplier per item (status penawaran `DIPILIH`) + alasan (audit).
- **Notifikasi**: penawaran dipilih → tercatat audit; belum ada notif eksternal.

#### 4.3.4 Buat PO — KASI_SARPRAS
- **Screen**: Buat PO, Approval PO, Detail PO (+PDF).
- **Input**: grouping permintaan FINAL per SUPPLIER (F03), tanggal PO, harga satuan (otomatis dari penawaran DIPILIH), PPN (F01), nomor PO (F11).
- **Aturan**: approval KABAG bila nilai ≥ F02; reject → DRAFT.
- **Transisi**: DRAFT → APPROVED → SENT → PARTIAL_RECEIVED → CLOSED. Auto-close bila qty penuh (F08).
- **Snapshot saat buat PO**: F01, F03, F11.
- **Notifikasi**: PO_CREATED; alert PO belum diterima (F09), alert FINAL belum di-PO (F10).

### 4.4 Penerimaan — PETUGAS_GUDANG
- **Screen**: Penerimaan (per PO detail), upload lampiran.
- **Input**: no terima (J05), qty terima/reject, kondisi, no DO/invoice/faktur pajak.
- **Aturan**: partial (F04), toleransi qty (F05), dokumen wajib (F07), retur reject mode (F06).
- **Snapshot saat terima**: F04, F05, F07.
- **Notifikasi**: BARANG_TIBA.

### 4.5 Serah Terima — PETUGAS_GUDANG + USER_RUANGAN
- **Screen**: Serah Terima (BAST, nomor J04), konfirmasi ruangan.
- **Transisi**: PENDING → CONFIRMED | DISPUTED → (resolve Kasi) → CONFIRMED.
- **Aturan**: konfirmasi wajib (G01), batas hari (G02).
- **Snapshot**: G01, G02, G03, G05.
- **Notifikasi**: SERAH_TERIMA → ruangan; overdue konfirmasi → eskalasi.

### 4.6 Registrasi Aset & Mutasi — ADMIN_ASET
- **Screen**: Registrasi Aset (+QR), Mutasi Aset.
- **Trigger**: serah terima dengan nilai ≥ G03 (atau `items.threshold_aset`) → auto-draft aset (G04).
- **Input**: kode aset (format A05), no seri, umur ekonomis, kondisi, nilai; QR label (G05).
- **Transisi aset**: REGISTERED → IN_USE → MAINTAINED → DISPOSED. Mutasi butuh approval WADIR (G06); hapus butuh approval (G07).
- **Snapshot**: A05, G03, G05.

### 4.7 Realisasi & Pelaporan — KABAG_KEUANGAN, semua
- **Screen**: Dashboard Rekap, Dashboard Realisasi, Laporan Permintaan, Audit Log.
- **Isi**: nilai PO per supplier/kategori, progres penerimaan, serah terima, aset terdaftar.
- **Cetak**: setiap list/detail punya Cetak PDF + Export Excel (J06); watermark draft (J01), log cetak (J02), placeholder TTD (J03).

### 4.8 Stok Opname — PETUGAS_GUDANG
- **Screen**: Stok Opname. Input fisik vs sistem, selisih tercatat + audit.

## 5. State Machine (ringkas)

| Entitas | Alur status |
|---|---|
| Permintaan | DRAFT → SUBMITTED → VERIFIKASI_KASI → APPROVED_KABID → APPROVED_WADIR → FINAL → IN_PO → RECEIVED → HANDED_OVER → CLOSED · reject ⇒ RETURNED → (edit) → SUBMITTED (`revisi_ke` +1, riwayat utuh) |
| PO | DRAFT → APPROVED → SENT → PARTIAL_RECEIVED → CLOSED · reject ⇒ DRAFT |
| Penawaran | DIAJUKAN → DIPILIH (menjadi harga PO) | TIDAK_DIPILIH (masih tercatat sebagai pembanding) |
| Serah terima | PENDING → CONFIRMED | DISPUTED → CONFIRMED |
| Aset | REGISTERED → IN_USE → MAINTAINED → DISPOSED |

Semua transisi tercatat di `approval_log` (alur) & `audit_trail` (data).

**Revisi permintaan.** Penolakan (langkah mana pun) → `RETURNED`; pemohon boleh
mengubah jumlah/keterangan (seperti DRAFT) lalu submit ulang: status kembali
`SUBMITTED` langkah 1 dengan SLA baru, `revisi_ke` naik, riwayat menampilkan
Diajukan → Ditolak (catatan) → Diajukan ulang. Notifikasi baru ke Kasi.

**Disposisi pimpinan.** Wadir/Admin dapat menulis arahan pada satu pengajuan
(tabel `disposisi`) dan opsional menunjuk pelaksana. Disposisi **tidak**
mengubah langkah approval — tercatat di audit + notifikasi ke pemohon dan
pelaksana. Tampil di Detail Permintaan (Riwayat & Disposisi Pimpinan).

## 6. Notifikasi Event

| Event | Penerima (default D03) | Titik alur |
|---|---|---|
| SUBMIT | KASI_SARPRAS | 4.1 submit |
| APPROVE / REJECT | USER_RUANGAN | 4.2 tiap step |
| SLA_WARN / SLA_OVERDUE | pemilik step; atasan (C09) | job harian 08:00 |
| PO_CREATED | approver PO | 4.3 |
| BARANG_TIBA | ruangan peminta | 4.4 |
| SERAH_TERIMA | ruangan penerima | 4.5 |
| ITEM_REQUEST | KASI_SARPRAS | 4.1 usulan item baru |

Channel: IN_APP (WebSocket broadcast per user_id); event aktif di D02; retensi L02.

## 7. SLA & Eskalasi

- Deadline per step dari C02 (default 5 hari kerja tiap step, basis C03, jam kerja C04/C05).
- `sla_tracking` diisi saat transisi masuk step; status ON_TIME → OVERDUE → ESCALATED.
- Job `sla_checker` harian 08:00 WITA; pengingat H-1 (C06); eskalasi setelah C07 hari, maks C08 level, notif ke atasan (C09).

## 8. Snapshot Config per Transisi

| Transisi | Kode config yang di-freeze |
|---|---|
| Submit permintaan | C01, C02, E11, F11, G03, J04 |
| Approve | B01, B02, C01 |
| Buat PO | F01, F03, F11 |
| Terima barang | F04, F05, F07 |
| Serah terima | G01, G02, G03, G05 |
| Registrasi aset | A05, G03, G05 |
| Cetak dokumen | J01, J02, J03, J04, J05, A01, A02, A03, A04 |

Resolusi config tetap 5 lapis: SNAPSHOT → RUANGAN → KATEGORI → GLOBAL → default.

## 9. Pemetaan 26 Modul ke Tahap & Fase

| Modul | Tahap alur | Fase |
|---|---|---|
| 1 Login, 2 Master Ruangan, 3 Master Item+Satuan, 4 Master User+Role, 21 Config Center, 22 Config Audit, 10 Audit Log | Fondasi | **Fase 1 — selesai** |
| 5 Form Permintaan, 6 My Requests, 25 Notifikasi Center | 4.1 | **Fase 2 — selesai** |
| 7 Inbox Approval, 24 Config Approval | 4.2 | **Fase 3 — selesai** |
| 26 Import Wizard (Excel → permintaan historis) | data | **Fase 5 — selesai** |
| 11 Master Supplier, 11b Katalog Supplier-Item, 11c Penawaran Harga, 11d Perbandingan & Penunjukan, 12 Buat PO, 13 Approval PO, 14 Detail PO+PDF | 4.3 | **Fase 8 — selesai** (PDF menyusul) |
| 15 Penerimaan, 16 Serah Terima | 4.4–4.5 | **Fase 9 — selesai** |
| 17 Registrasi Aset+QR, 18 Mutasi Aset | 4.6 | **Fase 10 — selesai** |
| 8 Dashboard Rekap, 9 Laporan Permintaan | 4.7 | Fase 6 |
| 19 Dashboard Realisasi | 4.7 | **selesai** · 20 Stok Opname | 4.8 | Fase 11 |
| 23 Hari Libur | SLA (C03) | Fase 3 |

## 10. Urutan Pengerjaan Sampai Selesai

**Prinsip: satu fase satu waktu; akhir tiap fase = ringkasan + instruksi uji + STOP review.**

| Urutan | Fase | Isi | Kriteria Selesai |
|---|---|---|---|
| 1 | **Fase 2 — Permintaan** ✅ | Form Permintaan multi-row, My Requests, submit + snapshot, notifikasi IN_APP (bell) | Ruangan bisa draft → submit; Kasi menerima notifikasi; snapshot C01/C02/E11/F11/G03/J04 tersimpan |
| 2 | **Fase 3 — Approval + SLA** ✅ | Inbox Approval, 3 step berjenjang, reject→RETURNED, catatan wajib, SLA per step + eskalasi + hari libur (C03) | Alur DRAFT→FINAL tuntas; reject mengembalikan ke draft; sla_tracking terisi; job eskalasi jalan |
| 3 | **Fase 5 — Import Data Excel** ✅ | Import Wizard: kolom JUMLAH Excel → `permintaan` historis (status CLOSED, snapshot), import_batch/log | 270+ baris permintaan 2026 masuk; pelaporan langsung punya data |
| 4 | **Fase 8 — Supplier & PO** ✅ | Master Supplier, Katalog Supplier-Item, Penawaran Harga, Perbandingan & Penunjukan Supplier, Buat PO (grouping F03, PPN F01, nomor F11), Approval PO (threshold F02), Detail PO | FINAL → penawaran ≥F12 → penunjukan → PO → APPROVED → SENT; perbandingan harga terekam audit |
| 5 | **Fase 9 — Penerimaan & Serah Terima** ✅ | Penerimaan partial + dokumen wajib, Serah Terima + konfirmasi ruangan + BAST | Barang tiba → terima → serah → CONFIRMED; dispute resolvable |
| 6 | **Fase 10 — Aset & Mutasi** ✅ | Registrasi aset + QR, mutasi (approval), auto-draft aset dari threshold | Serah terima ≥ G03 → aset REGISTERED dengan QR |
| 7 | **Fase 6 — Dashboard P1 & Cetak** | Dashboard Rekap, Laporan Permintaan, Cetak PDF + Export Excel semua list | Laporan bisa dicetak; log cetak tercatat |
| 8 | **Fase 11 — Dashboard P2 & Opname** (dashboard ✅, opname tersisa) | Dashboard Realisasi, Stok Opname | Realisasi PO→aset terlihat end-to-end |
| 9 | **Fase 12 — UAT & Deploy** | Config lanjutan, MFA (K06), UAT, deploy binary tunggal | Go-live |
| (gated) | Fase 4 — Offline Sync | **Ditunda** sampai ada bukti ruangan kehilangan koneksi; draft sementara di localStorage SPA | — |

**Catatan urutan**: pelaporan (Fase 6/11) ditaruh setelah alur transaksi inti karena laporan butuh data asli; import Excel (Fase 5) dimajukan sebelum PO agar realisasi bisa dibandingkan dengan tahun berjalan.

## 11. Tambahan Skema untuk Manajemen Supplier (di luar spec §3)

Manajemen supplier lengkap (katalog → penawaran → perbandingan → penunjukan)
butuh 2 tabel baru — spec §3 hanya punya `supplier`:

| Tabel | Kolom utama | Fungsi |
|---|---|---|
| `supplier_item` | supplier_id, item_id, harga_referensi, catatan, is_active | Katalog "supplier mana menjual item apa" + harga referensi |
| `penawaran` | supplier_id, item_id, harga_satuan, tanggal_penawaran, sumber, status (DIAJUKAN/DIPILIH/TIDAK_DIPILIH), alasan_penunjukan | Riwayat penawaran per item → dasar perbandingan & penunjukan |

Plus 1 config baru (di luar katalog 95 — perlu approval):

| Kode | Nilai default | Fungsi |
|---|---|---|
| `F12` `po/penawaran_min` number `3` | minimal penawaran dari supplier berbeda per item sebelum boleh masuk PO | Pengendali perbandingan harga |

Keduanya ikut pola yang sama: audit trail di semua mutasi, soft delete di `supplier_item`, snapshot F01/F03/F11 saat penawaran dicatat.

## 12. Definisi Selesai (berlaku untuk setiap fase)

1. `cargo fmt --check` bersih; `cargo clippy --workspace --all-targets -- -D warnings` 0 issue.
2. Semua test (unit + integrasi) lulus; `svelte-check` 0 error.
3. Endpoint baru teruji (curl / integration test) + RBAC per role benar.
4. Screen berbahasa Indonesia; list/detail punya tombol Cetak PDF + Export Excel.
5. Semua mutasi menulis `audit_trail`; transisi alur menulis `approval_log` + snapshot config.
6. Akhir fase: **ringkasan hasil + instruksi uji + STOP untuk review** — tidak lanjut ke fase berikutnya tanpa persetujuan.

## 13. Prinsip UX & Navigasi (dikunci)

Berlaku untuk semua layar; melanggar berarti regresi UX.

1. **Menu whitelist per peran.** Setiap rute wajib mendeklarasikan `roles`.
   `routeAllowed()` menolak bila `roles` kosong — bukan mengizinkan. Menu yang
   bukan kewenangan peran tidak ditampilkan sama sekali.
2. **Beranda = tugas hari ini**, bukan statistik sistem. Isinya per peran:
   draft belum diajukan, ditolak perlu perbaikan, sedang diproses, menunggu
   keputusan saya, final belum dipesan, barang dalam perjalanan, siap diserahkan.
   Setiap kartu punya tombol aksi menuju layar yang relevan.
3. **Satu sumber label status.** `lib/labels.ts` (`labelStatus`, `toneStatus`,
   `sisaSla`, `tahapAktif`). Kode status mentah (`IN_PO`, `CLOSED`, …) tidak
   boleh tampil ke pengguna; semua lewat `labelStatus()`.
4. **Satu baris = satu pengajuan.** Daftar permintaan dikelompokkan per
   `batch_id`; tombol Ajukan/Batalkan muncul sekali per pengajuan.
5. **Tahap alur selalu terlihat.** Komponen `StatusStepper` menampilkan
   5 tahap (Draft → Kasi → Kabid → Wadir → Final), "Menunggu: <peran>", dan SLA.
6. **Dialog wajib bisa diselesaikan.** `max-height: 90vh` + area aksi sticky,
   agar tombol simpan tidak pernah keluar layar. Error tampil **di dalam**
   dialog yang gagal, bukan hanya di belakangnya.
7. **Aksi disembunyikan bila API menolak.** Tombol konfirmasi BAST hanya muncul
   untuk `penerima_ruangan_id` yang bersangkutan (atau ADMIN) — cerminan aturan
   service, bukan tebakan berdasarkan role.
8. **Sidebar:** `nav { min-height: 0; overflow-y: auto }` dan item rapat, agar
   seluruh menu bisa dijangkau pada layar pendek (768p).
9. **Urutan sidebar = urutan alur kerja** (bukan abjad, bukan campur utilitas):
   Beranda → **Buat Permintaan** → Permintaan Saya → Persetujuan →
   Harga & Supplier → Pesanan (PO) → Penerimaan Barang → Serah Terima →
   Data Aset → Laporan → Master → Pengaturan. **Notifikasi di grup "Lainnya"
   paling bawah** (utilitas; lonceng header tetap jalan). "Buat Permintaan"
   tampil di sidebar — ini aksi paling sering pemohon.
10. **Istilah Indonesia** di permukaan pengguna: Persetujuan, Pesanan (PO),
    Penerimaan Barang, Serah Terima, Data Aset, Harga & Supplier,
    Pengaturan Sistem, Riwayat Pengaturan, Jejak Audit.
11. **Menu = matriks CRUD per role (WORKFLOW §3.2/3.3).** Rute menu hanya
    menyertakan role yang memang berwenang di modul tsb (auditor read-only →
    laporan & audit saja; kasi tidak melihat Master Ruangan/Data Aset; kabag
    tidak mengelola Harga & Supplier). Tombol aksi di dalam layar di-guard
    mengikuti RBAC handler API, bukan sekadar role layar.

## 14. Keputusan yang Dikunci

| Keputusan | Nilai |
|---|---|
| Kolom JUMLAH di Excel | **Opsi A**: diimpor sebagai `permintaan` historis status CLOSED di Fase 5 |
| Manajemen supplier | Alur 4 tahap: master+katalog (`supplier_item`) → penawaran (`penawaran`) → perbandingan & penunjukan (pilih + alasan) → PO. Minimal F12 penawaran per item |
| Offline sync (Fase 4) | Ditunda (gated), bukan prioritas |
| Katalog config | **96 parameter** (95 katalog §7 + F12 penawaran minimal) |
| Deployment | Binary tunggal (api + worker + SPA statis), PostgreSQL native, tanpa Docker |
| Bahasa & waktu | UI Bahasa Indonesia; zona waktu Asia/Makassar (WITA) |
