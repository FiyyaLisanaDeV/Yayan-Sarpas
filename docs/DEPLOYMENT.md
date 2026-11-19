# Deployment — SARPRAS & ALKES

> Binary tunggal + PostgreSQL 16 native, tanpa Docker. Target: VPS/cloud,
> HTTPS, database tidak terekspos publik.

## 1. Prasyarat server

- OS: Linux (systemd) atau macOS; arsitektur sesuai binary.
- **PostgreSQL 16** (native; pada macOS `brew install postgresql@16`).
- Rust stable (hanya untuk build; runtime tidak butuh Rust).
- Node.js (hanya untuk build SPA; runtime tidak butuh Node).

## 2. Build release

```bash
# 1. Build SPA
cd clients/web-spa && npm install && npm run build && cd ../..

# 2. Build binary tunggal (SPA disajikan oleh binary ini)
cargo build --release -p sarpras-api

# Hasil: target/release/sarpras-api
# Paket deploy = binary + (opsional) clients/web-spa/dist bila ingin SPA
# terpisah dari binary — tetapi default: dist di-serve langsung.
```

Catatan: binary menyajikan `clients/web-spa/dist` **jika direktori itu ada
relatif terhadap binary** (`../../clients/web-spa/dist` dari crate api).
Untuk deploy rapi, sertakan struktur yang sama atau set direktori statis
sesuai lingkungan build.

## 3. Konfigurasi produksi (`.env`)

```bash
DATABASE_URL=postgres://sarpras:PASSWORD_KUAT@localhost:5432/sarpras_prod
JWT_SECRET=<openssl rand -hex 32>        # WAJIB ganti, min 32 karakter
BIND_ADDR=127.0.0.1:8080                 # reverse proxy di depannya
UPLOAD_DIR=./data/uploads
MAX_UPLOAD_MB=10
ADMIN_INITIAL_PASSWORD=<password awal, lalu ganti>
```

- `JWT_SECRET` harus sama di semua restart (token tidak valid setelah ganti).
- Simpan `.env` dengan permission ketat (`chmod 600`).
- Database produksi: buat role & DB via `scripts/db-setup.sh`
  (sesuaikan `DB_USER`/`DB_PASSWORD`) atau SQL manual.

## 4. Migrasi & seed produksi

```bash
# Migrasi dijalankan OTOMATIS saat startup binary (idempoten, _sqlx_migrations).
# Seed data awal:
cargo run --release -p sarpras-seed
#   → 95 config + ruangan/item dari REKAPAN*.xlsx + admin awal
```

Pada deploy baru: jalankan `sarpras-seed` sekali, lalu `sarpras-api`.
Migrasi berikutnya otomatis ter-aplikasi saat binary baru start.

## 5. systemd unit (`/etc/systemd/system/sarpras.service`)

```ini
[Unit]
Description=SARPRAS & ALKES API
After=network.target postgresql.service

[Service]
User=sarpras
WorkingDirectory=/opt/sarpras
EnvironmentFile=/opt/sarpras/.env
ExecStart=/opt/sarpras/sarpras-api
Restart=always
RestartSec=3
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now sarpras
journalctl -u sarpras -f
```

## 6. HTTPS (reverse proxy — contoh Caddy)

```
# /etc/caddy/Caddyfile
sarpras.rsud-kendari.go.id {
    reverse_proxy 127.0.0.1:8080
}
```

- Database hanya listen di `localhost` (`pg_hba.conf`: jangan izinkan akses eksternal).
- WebSocket `/notifikasi/ws` didukung reverse proxy standar (tanpa konfigurasi khusus di Caddy).
- Tighten CORS saat produksi (saat ini permissive untuk dev).

## 7. Backup & retensi

```bash
# Backup DB harian
pg_dump -Fc "$DATABASE_URL" > /backup/sarpras_$(date +%F).dump

# File upload WAJIB ikut (storage lokal)
tar czf /backup/uploads_$(date +%F).tgz -C /opt/sarpras data/uploads

# Retensi (config L04, default 365 hari):
find /backup -name '*.dump' -mtime +365 -delete
```

- `audit_trail` & `config_audit` retensi **permanen** (L01/L03) — jangan di-truncate.
- Uji restore berkala: `pg_restore -d sarpras_prod < backup.dump`.

## 8. Upgrade (rolling)

1. Backup DB (lihat §7).
2. `cargo build --release -p sarpras-api` + build SPA.
3. Ganti binary, restart service.
4. Migrasi baru ter-aplikasi otomatis saat startup; bila gagal, lihat
   `journalctl` dan rollback binary + restore backup.

Migrasi didesain aditif (0001→0013, dst). Jangan edit file migrasi yang sudah
ter-aplikasi di produksi.

## 9. Monitoring dasar

```bash
# Health (tanpa auth)
curl http://127.0.0.1:8080/health
# {"success":true,"data":{"status":"ok","service":"sarpras-api","version":"0.1.0","database":"ok"}}

# Log (env filter)
RUST_LOG=sarpras_api=info,sarpras_worker=info,sqlx=warn
```

Yang perlu dipantau: `health` (database), log worker `sla_checker` (08:00 WITA),
penggunaan disk `data/uploads` (L05 alert storage 80%).

## 10. Klien Android (F1, distribusi internal)

- Build release/debug APK dari `clients/android/` (JDK 17, Android SDK 35). Wrapper
  jar belum disertakan — generate via Android Studio Sync atau `gradle wrapper`.
- Server API (`sarpras-api`) harus dapat dijangkau perangkat (LAN/VPN). Gunakan
  **HTTPS** di produksi; alamat server dikonfigurasi pengguna di layar masuk.
- CORS saat ini permissive (dev); ketatkan sesuai domain di produksi. Mutasi
  tervalidasi/terotorisasi di server (RBAC) — klien bukan sumber kebenaran.
- Push notifikasi (FCM) memerlukan penambahan backend `device_tokens` + hook kirim
  dan konfigurasi Firebase (belum diimplementasikan; saat ini polling/muat ulang).
- Distribusi asumsi internal (APK/MDM/internal testing). Lampiran/foto penerimaan
  tersimpan di `data/uploads/` server — sertakan dalam backup (§7).
