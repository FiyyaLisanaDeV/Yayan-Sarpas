# Panduan Pengembang — SARPRAS & ALKES

> Untuk kontributor. Setup, konvensi, testing, gotcha, dan proses fase.

## 1. Setup dari nol

```bash
# Prasyarat: Rust stable, Node 20+, PostgreSQL 16 (Homebrew/native)
./scripts/db-setup.sh                  # role sarpras + DB dev & test
cp .env.example .env                   # sesuaikan JWT_SECRET dll
cargo run -p sarpras-seed              # migrasi + 95 config + Excel + admin
cargo run -p sarpras-api               # http://127.0.0.1:8080 (binary tunggal)
```

Dev loop frontend:

```bash
cd clients/web-spa && npm install && npm run dev   # :5173, proxy ke :8080
cd clients/desktop-tauri && npm install && npm run tauri dev
```

Android (opsional, butuh JDK 17 + Android SDK platform 35):

```bash
# buka clients/android di Android Studio → Sync Gradle → Run, atau:
cd clients/android
# (sekali, bila wrapper jar belum ada) gradle wrapper --gradle-version 8.7
./gradlew :app:assembleDebug
```

Alamat server diatur di layar masuk (emulator `http://10.0.2.2:8080`). Kode F1:
`data/model/Models.kt`, `data/remote/SarprasApi.kt`, `data/repo/MobileRepository.kt`,
`ui/common`, `ui/home|notifications|approval|permintaan|po|penerimaan|serahterima`.

## 2. Struktur & layer

```
crates/shared    lib: error/envelope/auth/rbac/config-loader/app-config
crates/api       bin: router + middleware + domains/ + notifikasi + uploads
crates/worker    lib: jobs (sla_checker, notif_cleanup, config_cache_warm)
crates/seed      bin: migrasi + seed + import Excel
clients/web-spa  Svelte 5 (basis UI web)
clients/desktop-tauri  shell Tauri → ../web-spa/dist
clients/android  Kotlin + Jetpack Compose (Material 3), Retrofit/OkHttp, DataStore + EncryptedSharedPreferences
migrations/      SQL 0001..0017 (jangan edit yang sudah ter-aplikasi)
```

Per domain backend: **handler → service → repository** (lihat `docs/ARSITEKTUR.md` §3).
Domain kecil (kategori, satuan) satu file tiga layer. Android memakai REST endpoint
existing (envelope `{success,data,error,meta}`), auth Bearer + auto-refresh, cache
baca file-snapshot (`MobileRepository`), dan RBAC/validasi tetap di server.

## 3. Konvensi wajib (gate CI)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings   # 0 warning
cargo test --workspace
cd clients/web-spa && npx svelte-check --tsconfig ./tsconfig.json && npm run build
```

- **Tidak ada `unwrap()`** di jalur produksi → `AppError` + `?`.
- **sqlx compile-time checked**: `sqlx::query!` / `query_as!` / `query_scalar!`
  (butuh `DATABASE_URL` saat compile — disediakan `.cargo/config.toml`).
- Semua mutasi menulis `audit_trail`; transisi alur menulis `approval_log`
  + `sla_tracking` + snapshot config.
- RBAC: `ensure(actor.role, &[...])` di handler; ADMIN lolos otomatis.
- Master: soft delete (`is_active`); guard referensi sebelum nonaktifkan.
- Bahasa Indonesia untuk UI & pesan error; komentar kode boleh campuran.

## 4. Menambah endpoint baru (pola)

1. `crates/api/src/domains/<x>/repository.rs` — query `sqlx::query!`.
2. `service.rs` — aturan bisnis + audit + notifikasi + snapshot.
3. `handler.rs` — extractor, RBAC, serialize; `router()` baru.
4. Daftarkan di `router.rs` + `domains/mod.rs`.
5. Test di `crates/api/tests/integration.rs` (fixture via `support::`).
6. UI di `clients/web-spa/src/screens/` + rute di `router.svelte.ts` +
   `App.svelte`.

## 5. Testing

- **Unit**: dalam modul (`#[cfg(test)]`) — config loader, RBAC, parser, JWT.
- **Integration**: `crates/api/tests/integration.rs`, memakai DB `sarpras_test`.
  Test **serial** (satu DB dipakai bersama — mutex global di `support::setup`).
  Setiap test membersihkan tabel anak→induk lalu seed config minimal.
- Menambah config yang dipakai test → tambahkan ke `defs` di
  `seed_config_items` (test).
- `sqlx::migrate!` meng-embed file saat compile — **file migrasi baru butuh
  rebuild** (kadang `touch` file sumber bila tidak terdeteksi).

## 6. Gotcha yang sudah ditemukan (jangan diulang)

| Masalah | Solusi |
|---|---|
| sqlx + LEFT JOIN → kolom nullable salah infer | Anotasi eksplisit `AS "kolom?"` / `"kolom!"` |
| moka `sync::Cache` tidak ada | Fitur crate: `features = ["future","sync"]` |
| `serde(flatten)` di `Query` | Tidak didukung `serde_urlencoded` — tulis field eksplisit |
| calamine vs zip 2.x | Pakai calamine ≥ 0.28 |
| `BigDecimal` di query sqlx | Fitur `bigdecimal`; bandingkan dengan `Zero`/`sign` |
| Route dinamis `{id}` di axum 0.7 | Sintaks `:id` (0.7), bukan `{id}` |
| Handler butuh `ConnectInfo` saat test | Pakai `Option<ConnectInfo>` |
| Migrasi baru tak ter-aplikasi | Rebuild seed (`cargo build -p sarpras-seed`) |
| Test paralel bentrok di DB test | Mutex serial di `support::setup` |
| `INSERT INTO users` langsung di tes/seed | Wajib isi `username` (NOT NULL sejak migrasi 15); login pakai `username` atau `email` |

## 7. Alur fase (WORKFLOW §10)

Satu fase satu waktu; akhir fase: ringkasan + instruksi uji + **STOP review**.
Status terkini:

| Fase | Status |
|---|---|
| 1 Fondasi | ✅ |
| 2 Permintaan | ✅ |
| 3 Approval + SLA | ✅ |
| 5 Import Excel | ✅ (271 baris → permintaan CLOSED, idempoten) |
| 8 Supplier & PO | ✅ (katalog, penawaran, penunjukan, PO) |
| 9 Penerimaan & Serah Terima | ✅ |
| 10 Aset & Mutasi | ✅ |
| 11 Dashboard realisasi | ✅ (dashboard); stok opname tersisa |
| 6 Cetak PDF/Excel | ⬜ berikutnya |
| 12 UAT & Deploy | ⬜ |
| 4 Offline sync | ⏸ ditunda (gated) |
| Android F1 (klien) | ✅ operasional per peran (notifikasi, approval, permintaan, PO, penerimaan+foto, BAST/konfirmasi-dispute, cache baca). **Belum:** backend `device_tokens`+FCM, idempotensi tulis, F2 (scan QR aset/mutasi, dll.). |

Sebelum mulai fase: baca `WORKFLOW.md` §4 (detail tahap) + §8 (snapshot config)
+ `docs/` terkait. Keputusan yang dikunci ada di `WORKFLOW.md` §13. Rencana mobile:
`.commandcode/plans/android-app-design.md`.

**Gotcha mobile (F1):** build butuh JDK+SDK (wrapper jar belum disertakan, generate
via Android Studio/`gradle wrapper`); mutasi online-first & RBAC di server (jangan
duplikasi aturan bisnis di klien); cache baca = snapshot file sederhana (fallback
saat offline) bukan sumber kebenaran; upload dokumen penerimaan via multipart
`/uploads` (`ref_type=penerimaan`, `ref_id` = id penerimaan) — foto opsional. Push
butuh perubahan backend (tabel/endpoint device token + FCM).

## 8. Konfigurasi lingkungan

- `.env` di root (figment: default → `.env` → env OS).
- `.cargo/config.toml` menyediakan `DATABASE_URL` untuk compile-time sqlx
  (tidak menimpa env yang sudah ada).
- `RUST_LOG=sarpras_api=info,sarpras_worker=info,sqlx=warn` untuk log.
- Database test: `scripts/db-setup.sh` membuat `sarpras_test`;
  `TEST_DATABASE_URL` dipakai integration test.

## 9. Menambah config baru

1. `crates/seed/src/config_items.rs` — tambah ke `CONFIG_ITEMS` (kode unik,
   grup valid, approver).
2. Bila kritis: daftarkan di `KRITIS` di file yang sama **dan** di
   `crates/api/src/domains/config/service.rs` (harus sinkron!).
3. Bila butuh enum options: `enum_options(kode)`.
4. Jalankan `cargo run -p sarpras-seed` (idempoten).
5. Konsumsi lewat `ConfigLoader` (`get_i64/get_bool/get_json/…`) — **jangan
   baca `config_value` langsung di handler** (langgar cache & resolusi 5 lapis).

## 10. Design tokens & a11y (SPA)

- **Palet "petrol teal"** (`--brand #00695C`, `--brand-dark #004D43`) dipilih
  dengan alasan domain: RSUD Kota Kendari, kota teluk — bukan refleks
  "kesehatan = putih + biru kalem". Semua pasangan teks/permukaan dihitung
  WCAG AA; jangan ganti warna tanpa menjalankan skrip kontras.
- **Label status terpusat**: `lib/labels.ts` — kode mentah (`IN_PO`, `CLOSED`)
  tidak boleh tampil ke pengguna.
- **A11y wajib**: ring fokus `:focus-visible` 2px `--brand-dark` (9.8:1);
  search input wajib `aria-label`; dialog overlay wajib
  `role="dialog" aria-modal aria-labelledby` + fokus awal + tutup Esc
  (lihat `Penerimaan.svelte` / `SerahTerima.svelte` sebagai pola).
- **Kontrol form minimal 1rem** — mencegah iOS Safari auto-zoom saat fokus.
- `.btn-link` didefinisikan **sekali** di `app.css`; jangan definisikan ulang
  di dalam komponen.
