# Design System: Sarpras & Alkes RSUD Kota Kendari (Alternatif V2 - Clinical Modern)

Sistem desain alternatif modern berstandar WCAG 2.1 AA untuk tata kelola pengadaan dan persetujuan sarpras/alkes rumah sakit daerah.

## 1. Visual Theme & Atmosphere

- **Clinical Clarity & Institutional Rigor**: Menggabungkan ketelitian klinis medis dengan kejelasan birokrasi pemerintahan daerah.
- **Visual Tone**: Tenang, presisi tinggi, akuntabel, bebas distraksi visual (*zero cognitive clutter*), dengan sinyal visual tegas untuk kondisi darurat (*Cito*).
- **Surface Elevation**: Menggunakan batas garis struktural halus (*1px hairline border*) dipadukan bayangan lembut difus (*diffused elevation*) untuk membedakan level kedalaman tanpa memberatkan mata saat bertugas shift malam.

## 2. Color Palette & Roles (WCAG 2.1 AA Certified)

### Primary Foundation
- **Kemenkes Blue (`#0284c7`)**: Aksen primer interaktif, tombol CTA, dan indikator tahap aktif.
- **Deep Slate Navy (`#0369a1` / `#075985`)**: Teks tautan aktif, hover buttons, dan judul menu samping (rasio kontras 5.8:1 terhadap putih).
- **Soft Azure Wash (`#e0f2fe`)**: Latar seleksi baris aktif, focus ring container, dan background badge netral.
- **Clinical Canvas (`#f8fafc`)**: Latar dasar aplikasi sejuk (Slate 50).
- **Pure Surface (`#ffffff`)**: Latar kartu data, tabel, dan drawer disposisi.
- **Hairline Border (`#e2e8f0` / `#cbd5e1`)**: Pembatas antar kolom tabel dan kontainer formulir.

### Functional Urgency System
- **Cito Emergency & Overdue SLA (`#dc2626`)**: Latar `#fef2f2`, border `#fecaca`, teks `#991b1b` (rasio kontras 6.2:1).
- **Warning & Pending SLA (`#d97706`)**: Latar `#fffbeb`, border `#fde68a`, teks `#92400e` (rasio kontras 5.5:1).
- **Success & BAST Signed (`#16a34a`)**: Latar `#f0fdf4`, border `#bbf7d0`, teks `#166534` (rasio kontras 5.1:1).
- **Progress In-Procurement (`#0284c7`)**: Latar `#e0f2fe`, border `#bae6fd`, teks `#075985`.

## 3. Typography Rules

- **Display & Header**: `Inter` / `Source Sans 3` (semi-bold `600`, line-height `1.3`).
- **Body & Controls**: `14px` (`0.875rem`), line-height `1.5`, regular (`400`) & medium (`500`).
- **Data & Numeric Readout**: `JetBrains Mono` / `ui-monospace` untuk Kode Register, No. Seri Alkes, Nilai Anggaran (Rp), dan Sisa Jam SLA.

## 4. Component Stylings

- **Sidebar Navigation**: Fixed `270px`, pengelompokan menu dengan uppercase label tegas, active indicator garis kiri `3px solid #0284c7`, dan counter badge interaktif.
- **Action Buttons**: Ketinggian sentuh minimum `40px` (desktop), border-radius `6px`, padding `0.55rem 1rem`, transisi hover lembut.
- **Data Tables**: Sticky headers dengan background `#f1f5f9`, hover baris `#f8fafc`, dan aksi kontekstual terintegrasi.
- **Dual-Pane Approval Workspace**: Kolom antrean di kiri dengan indikator urgensi Cito, panel lembar telaah & disposisi di kanan untuk eliminasi modal bertumpuk.
