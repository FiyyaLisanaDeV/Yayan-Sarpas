# SARPRAS Mobile (Android)

Aplikasi Android operasional SARPRAS & ALKES (per peran, on point). Menggunakan API backend existing (`crates/api`) dengan kontrak envelope `{ success, data, error, meta }`.

## Status: F1 (MVP klien, operasional per peran)

**Fondasi (F0):**
- Proyek Gradle/Kotlin + Jetpack Compose (Material 3), minSdk 26, target/compileSdk 35.
- Konfigurasi **alamat server** (disimpan, default `http://10.0.2.2:8080` untuk emulator) pada layar masuk.
- Auth: login (`POST /auth/login`), restore sesi (`GET /auth/me`), logout (`POST /auth/logout`), auto-refresh token 401 (`POST /auth/refresh`) via OkHttp authenticator.
- Penyimpanan token terenkripsi (EncryptedSharedPreferences/Keystore) + preferensi server (DataStore).

**Fitur F1 (siap di klien, memakai endpoint REST existing):**
- **Beranda “Tugas Anda” per peran** (ringkasan/badge dari server): unread notifikasi, menunggu persetujuan, PO draft/terkirim, BAST menunggu konfirmasi/siap diserahkan — kartu tugas menuju layar terkait.
- **Notifikasi**: daftar, tandai dibaca / tandai semua dibaca, muat ulang, banner saat menampilkan cache/offline.
- **Persetujuan permintaan** (Kasi/Kabid/Wadir/Admin): inbox, detail, setujui/tolak (**catatan wajib saat tolak**).
- **Pesanan (PO)** (Kabag/Wadir/Kasi/Admin sesuai kewenangan): daftar/detail/rincian, setujui/tolak/kirim (cermin RBAC server).
- **Permintaan** (User Ruangan/Kasi/Admin): daftar (dikelompokkan per batch), detail, ajukan / ajukan ulang / batalkan draft, **buat permintaan** multi-item (pilih item, simpan draft / simpan & ajukan).
- **Penerimaan barang** (Gudang/Kasi/Admin): daftar PO terkirim (barang datang), catat penerimaan (pilih item/sisa, qty terima/tolak, kondisi, **No. DO/Invoice/Faktur Pajak wajib**), unggah **foto dokumen** opsional (`/uploads`, `ref_type=penerimaan`), riwayat.
- **Serah Terima/BAST** (Gudang/Kasi/Admin buat; User Ruangan/Admin konfirmasi): barang siap diserahkan, buat BAST (penerimaan siap, ruangan, penerima), daftar BAST, **konfirmasi** (hanya penerima tercatat atau ADMIN, sesuai server) atau **laporkan masalah (dispute)**.
- **Profil & keluar** (sesi + cache baca dihapus saat logout).
- **Cache baca** (snapshot file lokal per daftar) dengan fallback saat jaringan gagal/offline; mutasi tetap online-first dan divalidasi/diotorisasi di server (RBAC).

**Belum termasuk (lanjutan F1/F2):**
- **Push FCM** & backend `device_tokens`/hook kirim belum diimplementasikan (notifikasi saat ini: polling/muat ulang & badge saat dibuka).
- Revisi edit baris permintaan mendetail, disposisi, pembuatan PO/pengadaan detail, master/config/import, audit/laporan, scan QR & mutasi aset (F2), idempotensi tulis (F2).

Rancangan & peta rilis: `../../.commandcode/plans/android-app-design.md`.

## Prasyarat build

- **JDK 17** (mis. Temurin/Android Studio bundled JBR).
- **Android SDK** (platform 35, build-tools, platform-tools) + `ANDROID_HOME`/`local.properties`.
- Gradle: gunakan **Gradle Wrapper**. File `gradle/wrapper/gradle-wrapper.properties` sudah ada (Gradle 8.7) namun `gradlew` & `gradle-wrapper.jar` **belum disertakan**. Buat sekali via:
  - Android Studio (buka folder `clients/android` lalu Sync), atau
  - `gradle wrapper --gradle-version 8.7` (jika Gradle terpasang).

## Build & jalankan

```bash
cd clients/android
./gradlew :app:assembleDebug        # butuh JDK+SDK+wrapper
./gradlew :app:installDebug         # ke emulator/device terhubung
```

Atau buka `clients/android` di **Android Studio** → Sync Gradle → Run `app`.

> Build/run memerlukan lingkungan dengan JDK & Android SDK (tidak dapat diverifikasi di lingkungan tanpa SDK).

## Koneksi ke server

- Emulator Android: host mesin = `http://10.0.2.2:8080` (default).
- Perangkat fisik (satu jaringan): gunakan IP LAN server, mis. `http://192.168.x.x:8080` (atur di **Alamat Server** saat masuk).
- Server API (`sarpras-api`) harus dapat dijangkau dari perangkat; backend mengizinkan CORS.

## Catatan

- RBAC & aturan bisnis tetap di server; UI hanya cermin kewenangan. Jangan andalkan klien untuk validasi akhir.
- Fitur dikecualikan dari mobile (on point): master/config/import, audit trail, laporan/cetak, pengadaan detail (katalog/penawaran/buat PO) — gunakan web/desktop.
