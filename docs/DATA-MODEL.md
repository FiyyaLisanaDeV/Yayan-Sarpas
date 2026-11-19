# Model Data — SARPRAS & ALKES

> Sumber: `migrations/0001..0013` + query live `information_schema`
(38 tabel). Kolom persis sesuai database saat ini. Relasi utama:
> `permintaan` → `disposisi` (batch_id; arahan pimpinan + penunjukan pelaksana).

## 1. Diagram relasi (inti)

```
users ──────┬── permintaan ────── approval_log
            │        │
            │        ├── sla_tracking
            │        └── config_snapshot (ref_type='permintaan')
            ├── notifikasi
            ├── audit_trail
            ├── refresh_tokens
            └── attachments.uploaded_by

ruangan ────┬── users.ruangan_id
            ├── permintaan.ruangan_id
            ├── serah_terima.ruangan_id
            ├── aset_tetap.ruangan_id
            └── stok_opname.ruangan_id

kategori ─── items.kategori_id
satuan ───── items.satuan_id · po_detail.satuan_id

items ──────┬── permintaan.item_id
            ├── po_detail.item_id
            ├── aset_tetap.item_id
            └── stok_opname.item_id

supplier ─── po_header.supplier_id      (modul PO: skema siap, Fase 8)
po_header ── po_detail ── penerimaan ── serah_terima ──▶ (aset_tetap, Fase 10)
config_group ─ config_item ─┬─ config_value · config_audit · config_snapshot
                            └─ config_change_request
```

## 2. Tabel — kolom lengkap

### Master & referensi

| Tabel | Kolom | Catatan |
|---|---|---|
| `users` | id, nama, nip, email, password_hash, role, ruangan_id, is_active, mfa_secret, created_at | role ∈ 9 role §5; mfa_secret disiapkan (implementasi TOTP ditunda) |
| `ruangan` | id, kode, nama, is_active | soft delete |
| `kategori` | id, kode (`ALKES`/`SAPRAS`), nama | kode immutable |
| `satuan` | id, kode, nama | 11 kode seeded |
| `items` | id, nama, satuan_id, kategori_id, threshold_aset, is_active | unique `(nama, kategori_id)` (0011) |
| `supplier` | id, nama, npwp, kontak, alamat, is_active | **aktif Fase 8** |
| `supplier_item` | id, supplier_id, item_id, harga_referensi, catatan, is_active, created_at | **Fase 8** — katalog supplier-item; unique `(supplier_id, item_id)` |
| `penawaran` | id, supplier_id, item_id, harga_satuan, tanggal_penawaran, sumber, status, alasan_penunjukan, catatan, created_by, created_at | **Fase 8** — status DIAJUKAN/DIPILIH/TIDAK_DIPILIH |
| `ref_status` | id, kode, label, warna, urutan | label/warna status untuk UI |

### Permintaan & approval

| Tabel | Kolom | Catatan |
|---|---|---|
| `permintaan` | id, batch_id, tahun, ruangan_id, item_id, jumlah, satuan_id, keterangan, status, current_step, **revisi_ke**, sla_deadline, sla_status, created_by, submitted_at, closed_at | 1 baris = 1 item; batch = 1 pengajuan; `revisi_ke` naik saat submit ulang setelah penolakan |
| `disposisi` | id, batch_id, dari_user_id, untuk_user_id?, status_asal, isi, created_at | arahan pimpinan (WADIR/ADMIN) + penunjukan pelaksana; tidak mengubah langkah approval |
| `approval_log` | id, permintaan_id, step, actor_id, aksi, catatan, timestamp | aksi: `SUBMIT`, `RESUBMIT`, `APPROVE`, `REJECT` |
| `request_item` | id, nama_usulan, satuan_usulan, alasan, ruangan_id, requested_by, status, approved_by, approved_at, item_id_hasil | usul item baru (skema siap; UI menyusul) |
| `attachments` | id, ref_type, ref_id, path, jenis, uploaded_by, uploaded_at | path relatif ke `./data/uploads/` |
| `notifikasi` | id, user_id, tipe, ref_id, pesan, is_read, created_at | tipe: SUBMIT/APPROVE/REJECT/SLA_* |

### Realisasi — aktif Fase 8–10

| Tabel | Kolom |
|---|---|
| `po_header` **aktif** | id, no_po, tahun, supplier_id, tanggal_po, nilai_total, ppn, status, created_by, approved_by, approved_at, catatan, sla_deadline, sla_status, updated_at |
| `po_detail` | id, po_header_id, permintaan_id, item_id, qty, harga_satuan, subtotal, satuan_id |
| `penerimaan` **aktif** | id, po_detail_id, no_terima, tanggal_terima, qty_terima, qty_reject, kondisi, no_do, no_invoice, no_faktur_pajak, diterima_oleh, catatan, updated_at |
| `serah_terima` **aktif** | id, penerimaan_id, ruangan_id, tanggal_serah, qty_serah, penerima_ruangan_id, pemberi_id, no_ba, file_ba, catatan, status_konfirmasi, updated_at |
| `aset_tetap` **aktif** | id, item_id, ruangan_id, kode_aset, no_seri, tgl_perolehan, nilai, umur_ekonomis, kondisi, status, qr_path |
| `mutasi_aset` **aktif** | id, aset_id, dari_ruangan_id, ke_ruangan_id, tanggal, alasan, oleh_id |
| `stok_opname` | id, tahun, periode, item_id, ruangan_id, stok_fisik, stok_sistem, selisih, catatan, oleh_id |

### Config

| Tabel | Kolom | Catatan |
|---|---|---|
| `config_group` | id, kode, nama, deskripsi, urutan | 13 grup A–M |
| `config_item` | id, group_id, kode, label, tipe, nilai_default, scope, validation_json, is_sensitive, **approver_roles**, urutan | 95 baris; `approver_roles` tambahan di luar spec §3 |
| `config_value` | id, config_item_id, scope_type, scope_id, nilai, updated_by, updated_at, alasan | unique `(item, scope_type, COALESCE(scope_id,0))` |
| `config_audit` | id, config_item_id, scope_type, scope_id, nilai_lama, nilai_baru, changed_by, changed_at, alasan | append-only |
| `config_snapshot` | id, ref_type, ref_id, config_item_id, nilai | freeze saat transaksi; unique `(ref_type, ref_id, config_item_id)` |
| `config_change_request` | id, config_item_id, scope_type, scope_id, nilai_baru, alasan, status, requested_by, approved_by, approved_at, created_at | **tambahan di luar spec §3** — approval config kritis |
| `hari_libur` | id, tanggal, nama, tahun | 25 tanggal 2026 (SKB) |

### SLA

| Tabel | Kolom | Catatan |
|---|---|---|
| `sla_config` | id, step, hari, basis, aktif | tabel disiapkan; sumber aktif = config C01/C02 |
| `sla_tracking` | id, ref_type, ref_id, step, deadline, status, escalated_to, escalated_at, **closed_at** (0013) | status ON_TIME→OVERDUE→ESCALATED |

### Sync & import

| Tabel | Kolom | Catatan |
|---|---|---|
| `import_batch` | id, tipe, file_path, total_row, sukses, gagal, created_by, created_at | **Aktif Fase 5** — ringkasan per import Excel |
| `import_row_log` | id, batch_id, row_no, status, error_msg, raw_json | status: `SUKSES` / `DUPLIKAT` / `GAGAL` |
| `sync_conflict` | id, client_id, tabel, record_id, field, client_value, server_value, resolved_by, resolved_at | skema siap, Fase 4 (ditunda) |

### Audit & dokumen

| Tabel | Kolom | Catatan |
|---|---|---|
| `audit_trail` | id, user_id, tabel, record_id, aksi, before_json, after_json, timestamp | append-only, retensi permanen (L01) |
| `print_log` | id, ref_type, ref_id, jenis_dokumen, printed_by, printed_at | dipakai Fase 6 |
| `ttd_elektronik` | id, ref_type, ref_id, path, hash, signed_by, signed_at | fase lanjutan |
| `refresh_tokens` | id, user_id, token_hash, expires_at, revoked_at, created_at | **tambahan di luar spec §3** — rotasi refresh JWT |

## 3. State machine (status di kolom `status`)

| Entitas | Alur | Implementasi |
|---|---|---|
| Permintaan | DRAFT → SUBMITTED → VERIFIKASI_KASI → APPROVED_KABID → APPROVED_WADIR → FINAL → IN_PO → RECEIVED → HANDED_OVER → CLOSED · reject ⇒ RETURNED→DRAFT | **Aktif** sampai FINAL + RETURNED; status pasca-FINAL menunggu Fase 8–9 |
| PO | DRAFT → APPROVED → SENT → PARTIAL_RECEIVED → CLOSED · reject ⇒ DRAFT | **Aktif** |
| Serah terima | PENDING → CONFIRMED \| DISPUTED → CONFIRMED | **Aktif** |
| Aset | REGISTERED → IN_USE → MAINTAINED → DISPOSED | **Aktif** |

Pemetaan status → label/warna ada di `ref_status` (seed `0010`).

## 4. Snapshot config per transisi

| Transisi | Kode yang di-freeze ke `config_snapshot` |
|---|---|
| Submit permintaan | C01, C02, E11, F11, G03, J04 |
| Approve | B01, B02, C01 |
| (Fase 8+) Buat PO | F01, F03, F11 |
| (Fase 9+) Terima barang | F04, F05, F07 |
| (Fase 9+) Serah terima | G01, G02, G03, G05 |
| (Fase 10+) Registrasi aset | A05, G03, G05 |
| (Fase 6+) Cetak | J01..J05, A01..A04 |

## 5. Konvensi data

- **Soft delete** di semua master (`is_active`), tidak ada hard delete.
- **Optimistic lock** (`updated_at`) di tabel transaksi; trigger `set_updated_at` otomatis.
- **Uang** = `NUMERIC(18,2)` (BigDecimal di Rust, string di JSON).
- **Waktu** = `TIMESTAMPTZ` (WITA); tanggal = `DATE`.
- **JSON** = `JSONB` untuk `validation_json`, `approver_roles`, `before/after_json`, `raw_json`.
- **Batch permintaan** = `UUID` (`batch_id`), satu pengajuan banyak baris.
