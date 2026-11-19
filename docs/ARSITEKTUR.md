# Arsitektur — SARPRAS & ALKES

> Sumber: kode aktual (`Cargo.toml`, `crates/*`, `clients/*`). Diperbarui setelah Fase 3.

## 1. Gambaran

Aplikasi pengadaan RS berbasis **API-centric**: server adalah satu-satunya sumber
kebenaran; client (Web SPA, Tauri desktop, Android masa depan) hanya view + cache
token. Offline hanya untuk draft permintaan (Fase 4, ditunda).

```
┌─────────────┐     HTTP/JSON      ┌──────────────────────────────────────┐
│ Web SPA     │ ────────────────▶  │  BINARY TUNGGAL (sarpras-api)        │
│ (Svelte 5)  │                    │  ┌────────────────────────────────┐  │
└─────────────┘                    │  │ axum 0.7 (HTTP + WebSocket)    │  │
┌─────────────┐                    │  │  middleware: auth/RBAC/rate    │  │
│ Tauri 2     │ ────────────────▶  │  └──────────────┬─────────────────┘  │
│ desktop     │   (dist sama)      │                 │ tokio task         │
└─────────────┘                    │  ┌──────────────▼─────────────────┐  │
                                   │  │ worker (tokio-cron-scheduler)  │  │
                                   │  │  sla_checker 08:00 · cleanup   │  │
                                   │  └──────────────┬─────────────────┘  │
                                   │                 │                    │
                                   │      PostgreSQL 16 (sqlx)             │
                                   │      + moka cache (in-memory)        │
                                   │      + ./data/uploads/ (file lokal)  │
                                   └──────────────────────────────────────┘
```

- **Satu proses**: server HTTP + background job dalam satu binary (`cargo run -p sarpras-api`).
- **SPA statis disajikan binary yang sama** (`clients/web-spa/dist`) → deploy = 1 file + DB.
- **Tanpa Docker, tanpa Redis, tanpa MinIO** — sesuai keputusan awal.

## 2. Workspace

| Path | Jenis | Isi |
|---|---|---|
| `crates/shared` | lib `sarpras_shared` | `AppError` (kode + envelope), `ApiResponse`, auth (JWT+argon2), RBAC, **ConfigLoader** (moka, snapshot, simulasi), `AppConfig` (figment) |
| `crates/api` | bin `sarpras-api` + lib | Router, middleware, domain (handler→service→repository), notifikasi (WS), upload file |
| `crates/worker` | lib `sarpras_worker` | Job: `sla_checker`, `notif_cleanup`, `config_cache_warm` |
| `crates/seed` | bin `sarpras-seed` | Migrasi + seed 95 config + import Excel ruangan/item + admin awal |
| `clients/web-spa` | Svelte 5 + TS + Vite | Satu-satunya basis UI; dibuild jadi `dist/` |
| `clients/desktop-tauri` | Tauri 2 | Shell desktop; `frontendDist` menunjuk `../web-spa/dist` |
| `migrations/` | SQL | 0001–0013 (skema + seed statis + index + batch permintaan + SLA closed) |
| `scripts/` | sh | `db-setup.sh`, `reset-dev-data.sh` |

## 3. Layer per domain (backend)

```
handler.rs   →  HTTP: ekstraksi input, RBAC (ensure), validasi bentuk, serialize
service.rs   →  aturan bisnis: validasi referensi, transisi status, snapshot config,
                notifikasi, tulis audit_trail
repository.rs→  sqlx::query!/query_as! (compile-time checked), transaksi
```

Prinsip yang dipegang:

- **Tidak ada `unwrap()`** di jalur produksi — semua error lewat `AppError`.
- **Semua mutasi menulis `audit_trail`** (CREATE/UPDATE/DELETE/SUBMIT/APPROVE/…).
- **Transisi alur menulis `approval_log`** + menutup/membuka `sla_tracking`.
- **Transaksi tidak diedit bebas** — hanya lewat transisi state machine.
- **Master memakai soft delete** (`is_active`); transaksi tak bisa dihapus fisik.

## 4. Middleware

| Middleware | Peran | Catatan |
|---|---|---|
| `AuthUser` (extractor) | Decode Bearer JWT → muat user aktif dari DB | Sekaligus rate limit API 100 req/menit/user |
| RBAC `ensure(role, &[...])` | Guard per route | `ADMIN` selalu lolos |
| Rate limiter (moka) | Login 5×/15 menit (K05); API fixed-window 60 dtk | In-memory, reset saat restart |
| `TraceLayer` | Tracing HTTP | Env filter `RUST_LOG` |
| CORS permissive | Dev | Ketatkan untuk produksi di reverse proxy |

## 5. Alur data inti (terimplementasi)

```
USER_RUANGAN ──draft multi-row──▶ permintaan (DRAFT, batch_id)
      │ submit ── snapshot C01,C02,E11,F11,G03,J04 + notif SUBMIT
      ▼
permintaan (SUBMITTED) ──▶ sla_tracking terbuka (deadline = C02 basis kerja C03)
      │ KASI setujui  → VERIFIKASI_KASI   (approval_log + close/open SLA)
      │ KABID setujui → APPROVED_KABID
      │ WADIR setujui → FINAL             (SLA ditutup)
      │ siapa pun reject (catatan wajib B05) → RETURNED (kembali DRAFT)
      ▼
job sla_checker 08:00 → ON_TIME → OVERDUE (C06 pengingat) → ESCALATED (C07)
```

- **Snapshot config** (`config_snapshot`) mem-freeze nilai pada saat transaksi —
  perubahan config di kemudian hari tidak mengubah riwayat.
- **Notifikasi**: tersimpan di DB (`notifikasi`) + push WebSocket broadcast
  per-user (`PushHub`, single instance — cukup untuk 1 proses).

## 6. Keputusan desain & trade-off

| Keputusan | Alasan | Konsekuensi |
|---|---|---|
| SQL `permintaan` = 1 baris per item + `batch_id` | Skema spec per-item; UI multi-row | Approval berjalan per batch |
| Config zero-hardcode (95 parameter) | Ubah aturan tanpa deploy | Resolusi 5 lapis + cache harus selalu benar |
| Cache moka TTL 5 menit | Cukup untuk nilai config; tanpa Redis | Invalidate per-key saat update |
| WebSocket broadcast per-user in-memory | 1 instance server | Tidak untuk multi-instance horizontal |
| File upload ke `./data/uploads/` | Tanpa MinIO | Backup harus menyertakan direktori ini |
| Refresh token rotasi + hash di DB | Logout & reuse detection | Tabel `refresh_tokens` (di luar spec §3) |
| `config_change_request` (di luar spec §3) | 12 config kritis butuh approval | Alur: PENDING → APPROVED/REJECTED |
| `approver_roles` JSON di `config_item` | Kolom approver §7 butuh tempat | Ditentukan saat seed |

## 7. Diagram tabel (ringkas)

```
users ──┬─ permintaan ── approval_log · sla_tracking · config_snapshot
        ├─ notifikasi
        └─ audit_trail
ruangan ─┬─ users · permintaan · serah_terima · aset_tetap · stok_opname
items ───┬─ permintaan · po_detail · aset_tetap · stok_opname
satuan ── items · po_detail
kategori─ items
supplier─ po_header   (modul PO: skema siap, implementasi Fase 8)
config_group ─ config_item ─┬─ config_value · config_audit · config_snapshot
                            └─ config_change_request
```

Status implementasi modul: **Fase 1–3, 5, 8–10 selesai** (fondasi, permintaan,
approval+SLA, import Excel, supplier/penawaran/PO, penerimaan/serah terima,
aset/mutasi) + dashboard realisasi. Sisa: cetak PDF/Excel (Fase 6), stok
opname (Fase 11), UAT/deploy (Fase 12). Lihat `WORKFLOW.md` §10.

## 8. Import Excel (Fase 5)

Satu parser bersama dipakai dua tempat:

```
sarpras_shared::excel   ← helper: find_header, is_stop_row, parse_jumlah,
                          unit_to_kode, canonical_key, normalize_spaces
      ├── crates/seed          → master ruangan & items (25 ruangan, 246 item)
      └── crates/api/import    → permintaan historis (271 baris, status CLOSED)
```

## 9. Alur pengadaan (Fase 8–10)

```
permintaan FINAL
   │  penawaran (≥ F12 per item, dari supplier berbeda)
   ▼
perbandingan harga ──penunjukan supplier (DIPILIH + alasan)──┐
                                                            ▼
                              PO (supplier, PPN F01, no. F11) → APPROVED → SENT
                                                            ▼
              penerimaan (dokumen F07, partial F04, toleransi F05) → PO CLOSED
                                                            ▼
       serah terima (BAST J04, konfirmasi G01) → CONFIRMED ──nilai ≥ G03──▶ aset
                                                            ▼
                                   aset REGISTERED → IN_USE/MAINTAINED → mutasi
```

Setiap tahap menulis `audit_trail`; PO/penerimaan/serah/aset mem-freeze snapshot
config masing-masing.

- Pencocokan ke master memakai nama kanonik (huruf/angka saja, lowercase)
  sehingga perbedaan spasi/tanda baca tidak menggagalkan import.
- Import idempoten: baris identik yang sudah ada dihitung `DUPLIKAT`.
- Riwayat tersimpan di `import_batch` + `import_row_log` (SUKSES/DUPLIKAT/GAGAL).
