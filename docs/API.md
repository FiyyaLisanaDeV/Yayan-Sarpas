# Referensi API — SARPRAS & ALKES

> Daftar endpoint yang **benar-benar ada** di kode (didata ulang dari
> `router.rs` + semua `handler.rs`, pasca-Fase 3). Tidak ada endpoint
> imajiner: PO/supplier/penerimaan belum diimplementasi.

## 1. Konvensi

- **Base URL**: origin server (tanpa prefix `/api`). SPA di-build disajikan
  oleh server yang sama.
- **Auth**: header `Authorization: Bearer <access_token>`. Semua endpoint butuh
  auth kecuali `/health`, `/auth/login`, `/auth/refresh`, `/auth/register`,
  `/auth/logout`.
- **Envelope**:
  ```json
  { "success": true, "data": ..., "error": null, "meta": null }
  { "success": false, "data": null,
    "error": { "code": "AUTH_001", "message": "...", "details": null }, "meta": null }
  ```
- **Pagination** untuk endpoint list: `?page=1&per_page=25` (maks 100),
  pencarian `?q=...`. Respons menyertakan `meta: { page, per_page, total }`.
- **Kode error**: `AUTH_001..006` (auth/rate), `VAL_001` (validasi),
  `NF_001` (tidak ditemukan), `CF_001` (konflik), `CFG_001` (config),
  `DB_001` (server), `SRV_001` (internal).

## 2. Auth

| Method | Path | Body | RBAC | Keterangan |
|---|---|---|---|---|
| GET | `/health` | — | publik | status + versi + db |
| POST | `/auth/register` | `{nama, nip, email, password}` | publik* | bootstrap admin — hanya saat tabel users kosong |
| POST | `/auth/login` | `{username, password}` (atau `{email, password}` untuk kompatibilitas) | publik | → `{access_token, refresh_token, user}`; rate limit 5×/15 mnt |
| POST | `/auth/refresh` | `{refresh_token}` | publik | rotasi; token lama hangus |
| GET | `/auth/me` | — | semua | profil user aktif |
| POST | `/auth/logout` | `{refresh_token}` | publik | cabut refresh token |

`user` = `{id, nama, nip, email, role, ruangan_id}`. Role: `USER_RUANGAN`,
`KASI_SARPRAS`, `KABID_YANMED`, `WADIR_YANMED`, `PETUGAS_GUDANG`,
`KABAG_KEUANGAN`, `ADMIN_ASET`, `ADMIN`, `AUDITOR`.

## 3. Master — Ruangan

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/ruangan` | `?page&per_page&q&include_inactive` | semua |
| POST | `/ruangan` | `{kode, nama}` | ADMIN |
| GET | `/ruangan/:id` | — | semua |
| PUT | `/ruangan/:id` | `{kode, nama}` | ADMIN |
| DELETE | `/ruangan/:id` | — (soft delete) | ADMIN |

Guard: tolak nonaktif bila masih dipakai user aktif/permintaan aktif.

## 4. Master — Kategori & Satuan

| Method | Path | Body | RBAC |
|---|---|---|---|
| GET | `/kategori` · `/satuan` | `?page&per_page` | semua |
| POST | `/kategori` · `/satuan` | `{kode, nama}` | ADMIN |
| GET | `/kategori/:id` · `/satuan/:id` | — | semua |
| PUT | `/kategori/:id` · `/satuan/:id` | `{kode, nama}` (kode tak bisa diubah) | ADMIN |

Kode kategori hanya `ALKES` | `SAPRAS`.

## 5. Master — Item

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/items` | `?page&per_page&q&kategori_id&include_inactive` | semua |
| GET | `/items/options` | — **tanpa paginasi**, semua item aktif (id, nama, satuan_id, satuan_kode, kategori) | semua |
| POST | `/items` | `{nama, satuan_id, kategori_id, threshold_aset?}` | ADMIN, KASI_SARPRAS |
| GET | `/items/:id` | — | semua |
| PUT | `/items/:id` | sama seperti POST | ADMIN, KASI_SARPRAS |
| DELETE | `/items/:id` | — (soft delete) | ADMIN, KASI_SARPRAS |

`/items/options` ada karena form permintaan butuh seluruh katalog
(246 item > batas per_page 100).

## 6. Master — User

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/users` | `?page&per_page&q&role&include_inactive` | ADMIN, AUDITOR |
| GET | `/users/options` | `?q&role` — hanya `id`,`nama`,`ruangan` untuk dropdown | ADMIN, AUDITOR, PETUGAS_GUDANG, KASI_SARPRAS |
| POST | `/users` | `{nama, nip, email, username?, role, ruangan_id?, password}` — `username` kosong = otomatis dari prefix email | ADMIN |
| GET | `/users/:id` | — | ADMIN, AUDITOR |
| PUT | `/users/:id` | `{nama, role, ruangan_id?, is_active}` | ADMIN |
| POST | `/users/:id/reset-password` | `{password}` | ADMIN |
| DELETE | `/users/:id` | — (soft delete) | ADMIN |

Guard: admin aktif terakhir tak bisa dinonaktifkan/diturunkan. Password
mengikuti K03 (min 8) + K04 (huruf besar/kecil/angka).

## 7. Permintaan (Fase 2)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/permintaan` | `?page&per_page&q&tahun&status` | semua; USER_RUANGAN otomatis dibatasi ruangannya |
| POST | `/permintaan` | `{ruangan_id, items:[{item_id, jumlah, satuan_id, keterangan?}], lampiran_ada?}` | USER_RUANGAN, KASI_SARPRAS, ADMIN |
| GET | `/permintaan/:id` | — | pemilik / role terkait |
| PUT | `/permintaan/:id` | `{jumlah, satuan_id, keterangan?}` — hanya DRAFT | pemilik |
| GET | `/permintaan/batch/:batch_id` | — | pemilik / role terkait |
| DELETE | `/permintaan/batch/:batch_id` | — hanya DRAFT | pemilik |
| POST | `/permintaan/batch/:batch_id/submit` | `{urgent?}` | pemilik; urgent hanya KASI/ADMIN |

Submit: validasi periode (E01–E04, E09), maks item/qty (E06/E07), lampiran
(E08) → status `SUBMITTED` + snapshot 6 config + SLA + notif ke KASI.

## 8. Approval (Fase 3)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/approval/inbox` | `?page&per_page&q` | KASI, KABID, WADIR, ADMIN, AUDITOR |
| GET | `/approval/pending-count` | — | sama di atas |
| GET | `/approval/rantai` | — | sama di atas |
| GET | `/approval/:batch_id/riwayat` | — | role terkait + pemohon |
| POST | `/approval/:batch_id/setujui` | `{catatan?}` | KASI, KABID, WADIR, ADMIN |
| POST | `/approval/:batch_id/tolak` | `{catatan}` **wajib (B05)** | KASI, KABID, WADIR, ADMIN |

- Rantai dibaca dari config B01/B02 (default: KASI → KABID → WADIR).
- `setujui` maju 1 step; step terakhir → `FINAL` (SLA ditutup).
- `tolak` → `RETURNED` (current_step 0), pemohon memperbaiki dari draft.
- B03 (skip level) aktif → semua role rantai melihat seluruh antrean.
- Otorisasi per step ketat: role salah → `403 AUTH_002`.

## 9. Notifikasi (Fase 2)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/notifikasi` | `?page&per_page&only_unread` | semua (milik sendiri) |
| GET | `/notifikasi/unread-count` | — | semua |
| POST | `/notifikasi/:id/read` | — | pemilik |
| POST | `/notifikasi/read-all` | — | semua |
| GET | `/notifikasi/ws` | upgrade WebSocket | semua — push per user |

Tipe notifikasi yang dipakai sejauh ini: `SUBMIT`, `APPROVE`, `APPROVE_FINAL`,
`REJECT`, `SLA_WARN`, `SLA_OVERDUE`, `SLA_ESCALATED`.

## 10. Config

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/config/groups` | — | semua |
| GET | `/config/items` | `?group&q` — menyertakan nilai efektif + source | semua |
| GET | `/config/values/:kode` | `?ruangan_id&kategori_id` | semua |
| PUT | `/config/values/:kode` | `{nilai, alasan?}` | ADMIN |
| GET | `/config/change-requests` | `?page&per_page&status` | ADMIN, WADIR, KABID, KABAG |
| POST | `/config/change-requests/:id/approve` | — | role approver kode tsb |
| POST | `/config/change-requests/:id/reject` | — | role approver kode tsb |
| GET | `/config/audit` | `?page&per_page&kode` | semua |
| POST | `/config/simulate` | `{kode, nilai_baru}` | semua |

PUT pada 12 kode kritis → `config_change_request` PENDING (bukan langsung
berlaku). Nilai divalidasi per tipe (`validation_json`). Lihat `KONFIGURASI.md`.

## 11. Import (Fase 5)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/import/preview` | `?file_path` (opsional, auto-detect `REKAPAN*.xlsx`) | ADMIN, AUDITOR |
| POST | `/import/excel` | `{file_path?, tahun?, dry_run?}` | ADMIN |
| GET | `/import/batch` | `?page&per_page` | ADMIN, AUDITOR |
| GET | `/import/batch/:id` | — | ADMIN, AUDITOR |
| GET | `/import/batch/:id/rows` | `?page&per_page` | ADMIN, AUDITOR |

- Parser membaca sheet `ALKES` + `SAPRAS` dengan forward-fill ruangan dan
  berhenti di baris legenda/footer.
- Baris dicocokkan ke master by nama kanonik (case/punktuasi-insensitif);
  yang tidak cocok dilaporkan per nama pada `preview`.
- Baris tersimpan sebagai `permintaan` **status CLOSED**, `client_id = 'IMPORT'`.
- **Idempoten**: baris identik (tahun, ruangan, item, jumlah, satuan) yang
  sudah pernah diimpor dihitung `DUPLIKAT` dan tidak menambah data.
- Setiap baris dicatat di `import_row_log` (status `SUKSES`/`DUPLIKAT`/`GAGAL`);
  ringkasan di `import_batch` (total/sukses/gagal).
- Batas baris mengikuti config `I01` (default 10.000); simpan raw JSON
  mengikuti `I04`.

## 12. Supplier & Katalog (Fase 8)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/supplier` | `?page&per_page&q&include_inactive` | semua |
| POST | `/supplier` | `{nama, npwp?, kontak?, alamat?}` | ADMIN, KASI |
| GET | `/supplier/:id` | — | semua |
| PUT | `/supplier/:id` | `{nama, npwp?, kontak?, alamat?, is_active}` | ADMIN, KASI |
| DELETE | `/supplier/:id` | — (soft; ditolak bila sudah dipakai PO) | ADMIN, KASI |
| GET | `/supplier/:id/items` | — (katalog item) | semua |
| POST | `/supplier/:id/items` | `{item_id, harga_referensi?, catatan?}` (upsert) | ADMIN, KASI |
| DELETE | `/supplier/:id/items/:katalog_id` | — | ADMIN, KASI |
| GET | `/supplier/:id/supplier-item/:item_id` | — supplier yang menyediakan item | semua |

## 13. Penawaran & Perbandingan (Fase 8)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/penawaran` | `?page&per_page&q&dipilih` — perbandingan per item | ADMIN, KASI, KABAG, AUDITOR |
| POST | `/penawaran` | `{supplier_id, item_id, harga_satuan, tanggal_penawaran?, sumber?, catatan?}` | ADMIN, KASI |
| GET | `/penawaran/item/:item_id` | — semua penawaran item (urut harga) | ADMIN, KASI, KABAG, AUDITOR |
| GET | `/penawaran/:id` | — | sama |
| PUT | `/penawaran/:id` | `{harga_satuan, tanggal_penawaran, sumber?, catatan?}` | ADMIN, KASI |
| POST | `/penawaran/:id/pilih` | `{alasan?}` — **penunjukan supplier** | ADMIN, KASI |
| DELETE | `/penawaran/:id` | — (ditolak bila DIPILIH) | ADMIN, KASI |

Aturan: penunjukan menandai `DIPILIH` dan sisanya `TIDAK_DIPILIH` untuk item yang
sama. Harga PO diambil dari penawaran `DIPILIH` bila harga tidak diisi.

## 14. Purchase Order (Fase 8)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/po` | `?page&per_page&tahun&status&q` | ADMIN, KASI, KABAG, WADIR, AUDITOR, GUDANG |
| GET | `/po/kandidat` | `?q` — permintaan FINAL belum masuk PO | ADMIN, KASI |
| POST | `/po` | `{supplier_id, tahun?, tanggal_po?, catatan?, details:[{permintaan_id, item_id, qty, satuan_id, harga_satuan?}]}` | ADMIN, KASI |
| GET | `/po/:id` | — header + details | role terkait |
| GET | `/po/:id/detail` | — daftar detail | role terkait |
| POST | `/po/:id/approve` | — DRAFT → APPROVED (threshold F02) | ADMIN, KASI, KABAG, WADIR |
| POST | `/po/:id/reject` | `{catatan?}` | sama |
| POST | `/po/:id/kirim` | — APPROVED → SENT | ADMIN, KASI |
| GET | `/po/realisasi` | `?tahun` — rekap PO per status & supplier | ADMIN, KABAG, WADIR, KASI, AUDITOR |

Prasyarat PO per item: **≥ F12 penawaran** (default 3) **dan** sudah ada penunjukan
supplier. Pelanggaran → `422 VAL_001`. Nomor PO mengikuti F11, PPN F01,
snapshot F01/F03/F11.

## 15. Penerimaan (Fase 9)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/penerimaan` | `?page&per_page` | ADMIN, GUDANG, KASI, AUDITOR, KABAG |
| GET | `/penerimaan/po/:po_id` | — riwayat terima per PO | sama |
| GET | `/penerimaan/po/:po_id/siap` | — detail PO + qty sisa | ADMIN, GUDANG, KASI |
| POST | `/penerimaan` | `{po_detail_id, qty_terima, qty_reject?, kondisi?, no_do, no_invoice, no_faktur_pajak, catatan?, tanggal_terima?, tahun?}` | ADMIN, GUDANG |

Validasi: dokumen wajib sesuai F07 (`DO`/`INVOICE`/`FAKTUR_PAJAK`), qty tidak
melebihi sisa (+ toleransi F05), partial mengikuti F04. Status PO otomatis:
`PARTIAL_RECEIVED` bila masih ada sisa, `CLOSED` bila penuh (F08). Nomor dari J05.

## 16. Serah Terima (Fase 9)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/serah-terima` | `?page&per_page&status` | ADMIN, GUDANG, KASI, USER_RUANGAN, AUDITOR |
| GET | `/serah-terima/siap` | — penerimaan belum diserahkan | ADMIN, GUDANG, KASI |
| POST | `/serah-terima` | `{penerimaan_id, ruangan_id, penerima_ruangan_id, qty?, tanggal_serah?, catatan?}` | ADMIN, GUDANG, KASI |
| GET | `/serah-terima/:id` | — | role terkait |
| POST | `/serah-terima/:id/konfirmasi` | — PENDING → CONFIRMED | penerima atau ADMIN |
| POST | `/serah-terima/:id/dispute` | `{catatan?}` — PENDING → DISPUTED | penerima atau ADMIN |
| POST | `/serah-terima/:id/resolve` | `{catatan?}` — DISPUTED → CONFIRMED | ADMIN, KASI |

Nomor BAST dari J04, snapshot G01/G02/G03/G05. Saat CONFIRMED, aset otomatis
dibuat bila nilai ≥ threshold (G03 atau `items.threshold_aset`) dan G04 aktif.

## 17. Aset & Mutasi (Fase 10)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/aset` | `?page&per_page&q&ruangan_id&status` | ADMIN, ADMIN_ASET, AUDITOR, KASI |
| POST | `/aset/registrasi` | `{item_id, ruangan_id, nilai, no_seri?, tgl_perolehan?, umur_ekonomis?, kondisi?, kode_ruangan?}` | ADMIN, ADMIN_ASET |
| GET | `/aset/:id` | — | sama |
| POST | `/aset/:id/status` | `{status}` — REGISTERED/IN_USE/MAINTAINED/DISPOSED | ADMIN, ADMIN_ASET |
| GET | `/aset/:id/mutasi` | — riwayat mutasi | ADMIN, ADMIN_ASET, AUDITOR |
| POST | `/aset/:id/mutasi` | `{ke_ruangan_id, alasan?, tanggal?}` | ADMIN, ADMIN_ASET |
| GET | `/aset/:id/qr` | — payload QR label (G05) + simpan path | ADMIN, ADMIN_ASET, AUDITOR, KASI |

Kode aset mengikuti A05 (mis. `ALKES/2026/LPA/0001`), dengan penanganan tabrakan
kode otomatis.

## 18. Dashboard (Fase 6/11)

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/dashboard/rekap` | `?tahun` — permintaan per status/kategori/ruangan (P1) | semua (USER_RUANGAN dibatasi ruangannya) |
| GET | `/dashboard/realisasi` | `?tahun` — nilai PO/terima/serah/aset + supplier top (P2) | ADMIN, KABAG, WADIR, KASI, AUDITOR |

## 19. Audit

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| GET | `/audit-trail` | `?page&per_page&q&tabel&user_id` | ADMIN, AUDITOR |

Append-only; retensi permanen (L01).

## 20. Upload

| Method | Path | Body / Query | RBAC |
|---|---|---|---|
| POST | `/uploads` | multipart: `ref_type`, `ref_id`, `file` | semua |
| GET | `/uploads/:id` | — | semua |

Ekstensi diizinkan: `pdf jpg jpeg png docx xlsx xls zip csv`. Maks ukuran dari
`MAX_UPLOAD_MB` (default 10). File disimpan di `./data/uploads/{ref_type}/{ref_id}/{uuid}.{ext}`.

## 21. Contoh sesi (curl)

```bash
BASE=http://127.0.0.1:8080
TOKEN=$(curl -s -X POST $BASE/auth/login -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123"}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["data"]["access_token"])')

# buat draft permintaan 2 item
curl -s -X POST $BASE/permintaan -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"ruangan_id":7,"items":[{"item_id":208,"jumlah":2,"satuan_id":2}]}'

# submit batch
curl -s -X POST $BASE/permintaan/batch/<BATCH_ID>/submit \
  -H "Authorization: Bearer $TOKEN" -d '{}'

# approver menyetujui
curl -s -X POST $BASE/approval/<BATCH_ID>/setujui \
  -H "Authorization: Bearer $TOKEN" -d '{"catatan":"lengkap"}'
```
