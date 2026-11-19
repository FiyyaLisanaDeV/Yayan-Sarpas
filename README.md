# SARPRAS & ALKES — Sistem Manajemen Sarana Prasarana & Alat Kesehatan

Aplikasi desktop + web untuk mengelola siklus pengadaan barang rumah sakit:
permintaan ruangan → approval berjenjang → PO → penerimaan → serah terima →
registrasi aset → realisasi anggaran. Dua kategori: **ALKES** & **SAPRAS**.

Menggantikan pengelolaan manual berbasis Excel.

> **Status: Fase 1–3, 5, 8–10 selesai** (fondasi, permintaan, approval+SLA,
> import Excel, supplier/penawaran/PO, penerimaan/serah terima, aset/mutasi)
> plus dashboard realisasi. Sisa: cetak PDF/Excel, stok opname, UAT/deploy.
>
> **Acuan alur kerja & urutan pengerjaan:** lihat [WORKFLOW.md](./WORKFLOW.md) —
> memetakan tahap bisnis → screen → role → transisi status → config → notifikasi,
> plus workflow CRUD per modul dan urutan fase sampai selesai.

---

## Dokumentasi

| Dokumen | Isi |
|---|---|
| [docs/ARSITEKTUR.md](./docs/ARSITEKTUR.md) | Workspace, crate, layer, middleware, alur data, keputusan desain |
| [docs/API.md](./docs/API.md) | Referensi semua endpoint aktual (method, path, RBAC, body, contoh curl) |
| [docs/DATA-MODEL.md](./docs/DATA-MODEL.md) | 38 tabel + relasi + state machine + snapshot config |
| [docs/KONFIGURASI.md](./docs/KONFIGURASI.md) | Katalog 95 parameter, resolusi 5 lapis, 12 kritis, cara mengubah |
| [docs/PANDUAN-PENGGUNA.md](./docs/PANDUAN-PENGGUNA.md) | Panduan per role & per screen (end-user) |
| [docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md) | Build release, systemd, HTTPS, backup, upgrade, monitoring |
| [docs/PENGEMBANG.md](./docs/PENGEMBANG.md) | Setup, konvensi, testing, gotcha, proses fase |
| [WORKFLOW.md](./WORKFLOW.md) | Alur kerja aplikasi & urutan fase (acuan utama) |

---

## 1. Ringkasan Fase 1

| Komponen | Status |
|---|---|
| Workspace Rust (api, worker, shared, seed) + Tauri + Web SPA | Selesai |
| 38 tabel (migrasi sqlx) + index FK/status/tahun/updated_at + trigger | Selesai |
| 96 parameter config + nilai GLOBAL + audit 12 config kritis | Selesai |
| Master: ruangan, kategori, satuan, item, user | Selesai |
| Seed master dari Excel rekapan RS (25 ruangan, 246 item) | Selesai |
| Auth: login JWT, refresh rotasi, me, logout, register bootstrap | Selesai |
| Middleware: auth, RBAC, rate limit, error envelope, tracing | Selesai |
| Config loader: resolusi 5 lapis + cache moka + snapshot + simulasi | Selesai |
| Config Center: edit GLOBAL, approval kritis, audit trail | Selesai |
| Upload file ke filesystem lokal | Selesai |
| Binary tunggal (axum + background job) | Selesai |
| Web SPA + Tauri shell | Selesai |
| Test: 55 (unit + integrasi) | Lulus |
| **Fase 2:** Form Permintaan multi-row, My Requests, notifikasi in-app | Selesai |
| **Fase 3:** Inbox Approval 3 jenjang, reject→RETURNED, SLA + eskalasi | Selesai |
| **Revisi:** RETURNED bisa diubah & diajukan ulang (revisi_ke + riwayat) | Selesai |
| **Disposisi:** arahan pimpinan + penunjukan pelaksana pada pengajuan | Selesai |
| **Fase 5:** Import Wizard Excel → permintaan historis (271 baris), idempoten | Selesai |
| **Fase 8:** Supplier + katalog, penawaran, perbandingan & penunjukan, PO | Selesai |
| **Fase 9:** Penerimaan partial + dokumen wajib, serah terima + konfirmasi | Selesai |
| **Fase 10:** Registrasi aset + QR, mutasi, status aset | Selesai |
| **Dashboard:** Rekap permintaan (P1) + realisasi PO/aset (P2) | Selesai |
| **UX:** menu whitelist per peran, beranda berbasis tugas, stepper alur, label status terpusat | Selesai |
| **UX:** layar Penerimaan Barang + Serah Terima (sebelumnya hanya API) | Selesai |

---

## 2. Prasyarat (tanpa Docker)

- **Rust** stable (lihat `rust-toolchain.toml`) — `rustup` disarankan
- **PostgreSQL 16** native
- **Node.js** 20+ dan npm
- macOS/Linux/Windows

### Pasang PostgreSQL 16 (macOS via Homebrew)

```bash
brew install postgresql@16
brew services start postgresql@16
```

Pastikan `pg_config`/`libpq` dapat ditemukan saat build sqlx. Jika perlu:

```bash
export PATH="$(brew --prefix postgresql@16)/bin:$PATH"
```

---

## 3. Setup

```bash
# 1. Buat role + database (idempoten)
./scripts/db-setup.sh

# 2. Konfigurasi environment
cp .env.example .env
#   ubah JWT_SECRET (min 32 karakter) & ADMIN_INITIAL_PASSWORD untuk produksi

# 3. Migrasi + seed data
cargo run -p sarpras-seed

# 4. Jalankan server (api + worker dalam satu proses)
cargo run -p sarpras-api
```

Server: <http://127.0.0.1:8080> — `GET /health` untuk cek status.

**Akun awal:** nama pengguna `admin` / password dari `ADMIN_INITIAL_PASSWORD`
(default `admin123`). **Wajib diganti setelah login pertama** lewat Master User
→ Reset Password. Login memakai **nama pengguna** (bukan email); email tetap
diterima untuk kompatibilitas.

### Seed data ruangan & item dari Excel

Seed **otomatis menemukan** file rekapan, urutan pencarian:

1. `seed-data/ruangan_items.xlsx`
2. `seed-data/REKAPAN*.xlsx`
3. `REKAPAN*.xlsx` di root proyek ← **file Anda ada di sini**

Cukup jalankan:

```bash
cargo run -p sarpras-seed
```

Format yang didukung adalah format rekapan RS apa adanya:

- Sheet `ALKES` dan `SAPRAS` (nama sheet dicari case-insensitive).
- Kolom: `NO | RUANGAN | NAMA ALKES/BARANG | JUMLAH PERMINTAAN RUANGAN | KETERANGAN`.
- Kolom `RUANGAN` boleh hanya terisi di baris pertama tiap blok (forward-fill otomatis).
- Satuan menempel pada teks jumlah (`10 box`, `1 unit`, `8 Batang`) → dipetakan ke
  kode satuan (`BOX`, `UNIT`, `BATANG`, dst). Satuan tak dikenal → `UNIT`.
- Baris legenda warna (`Kuning : belum ter input…`) dan footer tanda tangan
  otomatis dianggap akhir data.

Hasil impor pada data saat ini: **25 ruangan, 246 item** (ALKES 165, SAPRAS 81).
Baris `(PLACEHOLDER)` lama dihapus otomatis setelah impor berhasil.

Catatan:

- Kolom `JUMLAH PERMINTAAN RUANGAN` **tidak** disimpan ke tabel master — itu data
  permintaan (Fase 2/5). Seed hanya mengisi master `ruangan` dan `items`.
- Seed **idempoten**: dijalankan berulang tidak menggandakan data
  (unique index `items(nama, kategori_id)`).
- Bila file tidak ditemukan atau gagal dibaca, seed **tidak gagal** — memakai data
  placeholder bertanda `(PLACEHOLDER)` dan menampilkan peringatan.

### Bersihkan residu uji (database dev)

```bash
./scripts/reset-dev-data.sh
```

Menghapus `config_change_request`, `config_audit` (kecuali `INITIAL_SEED`),
`audit_trail`, `refresh_tokens`, `attachments`, dan isi `data/uploads/`.
Config dan master data **tidak** disentuh.

---

## 4. Menjalankan client

### Web SPA (dev)

```bash
cd clients/web-spa
npm install
npm run dev          # http://localhost:5173 (proxy API ke :8080)
```

### Web SPA (produksi)

```bash
cd clients/web-spa && npm install && npm run build
```

Setelah `dist/` ada, **binary API otomatis menyajikan SPA** — cukup satu
proses untuk API + UI:

```bash
cargo run -p sarpras-api   # buka http://127.0.0.1:8080
```

### Desktop (Tauri 2)

```bash
cd clients/desktop-tauri
npm install
npm run tauri dev      # dev: window + Vite dev server
npm run tauri build    # paket aplikasi desktop
```

Tauri memakai `clients/web-spa/dist` sebagai frontend (satu basis kode UI).
Draft offline (SQLite + sync queue) menyusul Fase 4.

---

## 5. Struktur repositori

```
Cargo.toml                    Workspace: shared, api, worker, seed
rust-toolchain.toml
.env / .env.example
migrations/                   0001..0008 skema + 0010 seed statis
scripts/db-setup.sh           Role + DB dev & test
data/uploads/                 File lampiran (gitignored)
crates/
  shared/                     AppError, envelope, auth (JWT+argon2), RBAC,
                              ConfigLoader (moka + snapshot + simulasi), AppConfig
  api/                        axum 0.7, handler→service→repository per domain
  worker/                     Job scheduler in-process
  seed/                       Migrasi + seeding 95 config + import Excel
clients/
  web-spa/                    Svelte 5 + TypeScript + Vite
  desktop-tauri/              Tauri 2 shell
```

### Layer per domain

```
handler (HTTP, validasi input, RBAC)  →  service (aturan bisnis, audit)  →  repository (sqlx)
```

---

## 6. Endpoint API (Fase 1)

Response selalu `{ success, data, error, meta }`. Semua endpoint butuh
`Authorization: Bearer <access_token>` kecuali `/health`, `/auth/login`,
`/auth/refresh`, `/auth/register` (bootstrap), `/auth/logout`.

| Method | Path | Keterangan |
|---|---|---|
| GET | `/health` | Status server + database |
| POST | `/auth/register` | Bootstrap admin (hanya saat belum ada user) |
| POST | `/auth/login` | Login → access + refresh token |
| POST | `/auth/refresh` | Rotasi refresh token |
| GET | `/auth/me` | Profil user aktif |
| POST | `/auth/logout` | Cabut refresh token |
| GET/POST | `/ruangan` · `/ruangan/:id` | Master ruangan |
| GET/POST | `/kategori` · `/satuan` | Master kategori & satuan |
| GET/POST | `/items` · `/items/:id` | Master item |
| GET/POST | `/users` · `/users/:id` · `/users/:id/reset-password` | Master user |
| GET | `/config/groups` · `/config/items` | Daftar grup & parameter (+nilai efektif) |
| GET/PUT | `/config/values/:kode` | Nilai efektif / ubah nilai GLOBAL |
| GET | `/config/change-requests` | Daftar perubahan config kritis |
| POST | `/config/change-requests/:id/approve` · `/reject` | Keputusan approver |
| GET | `/config/audit` | Riwayat perubahan config |
| POST | `/config/simulate` | Simulasi dampak perubahan config |
| GET | `/audit-trail` | Audit log aksi pengguna |
| POST/GET | `/uploads` · `/uploads/:id` | Unggah / unduh lampiran |

List selalu berpaginasi: `?page=1&per_page=25&q=kata-kunci`.

---

## 7. Config Center

- **Resolusi 5 lapis:** Snapshot → RUANGAN → KATEGORI → GLOBAL → default.
  Setiap respons menyertakan `source` (SNAPSHOT/RUANGAN/KATEGORI/GLOBAL/DEFAULT).
- **Cache moka:** TTL 5 menit; invalidate per-key saat `config_value` berubah,
  invalidate total saat startup.
- **Snapshot transaksi:** `ConfigLoader::snapshot(ref_type, ref_id, kodes, ctx)`
  membekukan nilai ke `config_snapshot` (histori tidak berubah).
- **12 config kritis** (`B01 B02 B03 B04 C01 C02 C07 F01 F02 G03 K06 L01`)
  tidak langsung diterapkan: perubahan masuk `config_change_request` (PENDING)
  dan baru aktif setelah disetujui role approver yang berwenang.
  Semua perubahan tercatat di `config_audit`.

### Skenario uji wajib

1. Login sebagai ADMIN → **Config Center** → filter grup **SLA**.
2. Ubah `C01` (SLA Default) dari `5` → `7` → tombol **Ajukan Perubahan**.
3. Buka tab **Persetujuan** → **Setujui**.
4. Nilai `C01` menjadi `7`, dan **Config Audit** mencatat `5 → 7`.

Verifikasi via API:

```bash
TOKEN=$(curl -s -X POST localhost:8080/auth/login -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123"}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["data"]["access_token"])')

curl -s localhost:8080/config/values/C01 -H "Authorization: Bearer $TOKEN"
curl -s "localhost:8080/config/audit?kode=C01" -H "Authorization: Bearer $TOKEN"
```

---

## 8. Test & kualitas kode

```bash
cargo fmt --all -- --check          # format
cargo clippy --workspace --all-targets -- -D warnings   # lint (0 warning)
cargo test --workspace              # unit + integrasi
cargo audit                         # kerentanan dependensi (butuh cargo-audit)

cd clients/web-spa && npx svelte-check --tsconfig ./tsconfig.json && npm run build
```

Integration test memakai database `sarpras_test` (dibuat `scripts/db-setup.sh`)
dan berjalan serial karena berbagi satu DB.

---

## 9. Catatan lingkungan

- **Timezone:** Asia/Makassar (WITA) — atur zona waktu server sesuai deployment.
- **Rate limit:** login 5×/15 menit (K05); API 100 req/menit/user.
- **Password:** minimal 8 karakter + huruf besar/kecil/angka (K03/K04).
- **Hari libur 2026:** 25 tanggal (SKB 3 Menteri No. 1497/2/5 Tahun 2025).
- **MFA (K06):** kolom `mfa_secret` sudah tersedia; implementasi TOTP menyusul.
- **Tidak ada `unwrap()`** di jalur produksi; error dikembalikan sebagai
  `AppError` dengan kode terstruktur.

### Katalog config: 68 vs 95

Spesifikasi menyebut "68 seed wajib", namun rincian per grup (A:8, B:7, C:9,
D:6, E:11, F:11, G:7, H:7, I:4, J:6, K:8, L:5, M:6) berjumlah **95**.
Semua parameter pada katalog di-seed (95) — tidak ada yang dihilangkan.

---

## 10. Roadmap fase berikutnya

| Fase | Isi |
|---|---|
| 2 | Permintaan + notifikasi |
| 3 | Approval berjenjang + SLA |
| 4 | Offline sync (Tauri + SQLite) |
| 5 | Import data lama (Excel) |
| 6 | Dashboard P1 + cetak PDF/Excel |
| 7 | Web client gudang/approver |
| 8 | Supplier & PO |
| 9 | Penerimaan + serah terima |
| 10 | Aset + QR + mutasi |
| 11 | Dashboard P2 + stok opname |
| 12 | Config Center lanjutan + UAT + deploy |
