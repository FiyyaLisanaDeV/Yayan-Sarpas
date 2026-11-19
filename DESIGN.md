# Design System: Sarpras & Alkes RSUD Kota Kendari

Spesifikasi sistem desain untuk sistem persetujuan dan pengadaan sarana, prasarana, dan alat kesehatan (Sarpras & Alkes) RSUD Kota Kendari.

## 1. Visual Theme & Atmosphere

Sistem tata kelola rumah sakit daerah dengan atmosfer institusional yang bersih, profesional, terpercaya, dan tenang (clinical precision meets administrative clarity). Latar belakang abu-abu sejuk (`#f5f7fa`) dengan kontainer kartu putih bersih berbingkai halus (`#e0e5ec`) menciptakan ruang kerja bebas distraksi bagi staf ruangan, pengelola sarpras, tim gudang, hingga pejabat pengambil keputusan (Kasi, Kabid, Wadir).

## 2. Color Palette & Roles

### Primary Foundation
- **Kemenkes Blue (`#1e88e5`)**: Aksen identitas utama, penanda aksi prioritas, tab aktif, dan status berjalan.
- **Deep Navy Blue (`#1565c0`)**: State interaktif hover/active tombol, judul navigasi samping aktif.
- **Soft Azure Surface (`#e3f2fd`)**: Latar belakang seleksi aktif menu samping, badge status berjalan/informasi, dan fokus input form.
- **Clinical Backdrop (`#f5f7fa`)**: Warna dasar latar kanvas dashboard dan area kerja.
- **Pure Surface (`#ffffff`)**: Latar kartu form, panel sidebar, dialog, dan baris tabel data.
- **Hairline Border (`#e0e5ec`)**: Garis batas pembagi kolom tabel, outline input, dan pemisah kontainer.

### Accent & Interactive
- **Primary CTA (`#1e88e5`)**: Tombol aksi utama (e.g., "+ Buat Permintaan", "Setujui", "Simpan").
- **Secondary Neutral (`#ffffff` border `#e0e5ec`, text `#1c2733`)**: Tombol batal, ekspor, dan filter data.
- **Danger Action (`#f44336`)**: Tombol tolak permintaan, batalkan PO, atau hapus item.

### Typography & Text Hierarchy
- **Midnight Slate (`#1c2733`)**: Teks primer dengan keterbacaan tinggi untuk label kolom, data numerik, dan judul section.
- **Muted Steel (`#64748b`)**: Teks sekunder untuk hint, deskripsi sub-menu, SLA countdown, dan timestamp audit trail.

### Functional States & Badges
- **Success Forest (`#4caf50`)**: Latar `#e8f5e9`, teks `#1b5e20` untuk status FINAL, SELESAI, BAST_SIGNED, DITERIMA.
- **Warning Amber (`#ffc107` / `#f59e0b`)**: Latar `#fff8e1`, teks `#8a6d00` untuk status DRAFT, PENDING_APPROVAL, DEVIASI.
- **Danger Crimson (`#f44336`)**: Latar `#ffebee`, teks `#b71c1c` untuk status RETURNED (Ditolak/Revisi), OVERDUE SLA.
- **Progress Cerulean (`#1e88e5`)**: Latar `#e3f2fd`, teks `#1565c0` untuk status SUBMITTED, IN_PO, ON_DELIVERY.

## 3. Typography Rules

- **Primary Font Family**: `'Segoe UI', system-ui, -apple-system, 'Roboto', sans-serif`.
- **Display & Headings**:
  - `h1` (Judul Halaman): `24px` (`1.5rem`), font-weight `600`, letter-spacing `-0.01em`, line-height `1.3`.
  - `h2` (Judul Modul/Hero): `19.2px` (`1.2rem`), font-weight `600`, line-height `1.35`.
  - `h3` (Sub-bagian Tabel/Kartu): `16px` (`1.0rem`), font-weight `600`, line-height `1.4`.
- **Body & Tabular**:
  - Body Text: `14px` (`0.875rem`), line-height `1.5`, regular (`400`).
  - Small / Metadata / SLA: `12px` (`0.75rem`), line-height `1.4`, regular (`400`) & bold (`700`).
  - Monospace Data (Kode Item, No. BAST, No. PO): font-family `ui-monospace, monospace`, `13px`.

## 4. Component Stylings

- **Sidebar Navigation**: Lebar `264px`, sticky, latar putih bergaris kanan `1px solid #e0e5ec`. Brand badge kubus biru `36x36px` dengan inisial "SA". Item menu dengan tinggi sentuh nyaman, pill badge penanda antrean review berwarna oranye/kuning.
- **Action Buttons**: Border-radius `8px`, padding `0.5rem 0.9rem`, teks tegas putih, hover transition halus `0.15s`. Tombol sekunder berlatar putih dengan border tipis.
- **Card Containers**: Border-radius `10px`, background putih murni, border `1px solid #e0e5ec`, shadow tipis `0 1px 3px rgba(16, 24, 40, 0.06)`.
- **Status Stepper & Timeline**: Bar horizontal tahapan pengadaan (DRAFT -> PENGAJUAN -> VERIFIKASI KASI -> APPROVAL KABID/WADIR -> PO -> PENERIMAAN -> BAST SERAH TERIMA) dengan dot indikator aktif dan garis penghubung.
- **Data Table**: Header abu-abu muda lembut, padding sel `0.75rem 1rem`, hover baris `#f8fafc`, batas bawah antar baris `1px solid #e0e5ec`.
- **Form Fields**: Radius `8px`, border `1px solid #e0e5ec`, focus ring `2px solid #e3f2fd` dengan border color `#1e88e5`.

## 5. Layout Principles

- **Desktop Viewport**: Standard `1280px` hingga `1440px` dengan split sidebar 264px dan fluid content area.
- **Grid & Spacing Scale**: Base spacing `4px` dan `8px` (`0.25rem`, `0.5rem`, `1.0rem`, `1.5rem`).
- **Responsive Behavior**: Pada viewport < `860px`, sidebar berpindah ke stack atas dengan navigasi horizontal collapsible.

## 6. Design System Notes for Stitch Generation

- **Atmosphere Keywords**: Indonesian hospital administration, public healthcare supply chain, clean clinical governance, tabular procurement workflow, audit-ready layout.
- **Component Prompts**: 
  - Standard Hospital Header with RSUD logo, user profile pill, and unread notification bell badge.
  - Multi-tier SLA badge indicators with remaining hours countdown and color-coded urgency.
  - Multi-step approval approval stepper header with reviewer signature stamps.
