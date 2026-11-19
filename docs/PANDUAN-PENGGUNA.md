# Panduan Pengguna — SARPRAS & ALKES

> Panduan untuk pengguna akhir. Sesuai aplikasi aktual (build final).
> UI 100% Bahasa Indonesia, zona waktu WITA.
>
> **Beranda = daftar tugas Anda.** Setelah login, layar pertama menjawab
> "apa yang harus saya kerjakan hari ini?" sesuai peran — bukan statistik sistem.

## 1. Peran dan hak akses

Menu sidebar **hanya** menampilkan yang menjadi kewenangan peran Anda — menu
lain tidak muncul (bukan sekadar dinonaktifkan).

| Peran | Yang bisa dilakukan | Menu yang muncul |
|---|---|---|
| **User Ruangan** | Mengajukan permintaan ruangannya; konfirmasi serah terima | Beranda, Notifikasi, Buat Permintaan, Permintaan Saya, Serah Terima |
| **Kasi Sarpras** | Verifikasi langkah 1, penawaran, PO, master item | Beranda, Notifikasi, Buat/Permintaan Saya, Persetujuan, Harga & Supplier, Pesanan (PO), Penerimaan, Serah Terima, Laporan Realisasi, Master Item/Supplier |
| **Kabid Yanmed** | Menyetujui langkah 2 | Beranda, Notifikasi, Permintaan Saya, Persetujuan |
| **Wadir Yanmed** | Menyetujui langkah 3 (final), approval PO, laporan | Beranda, Notifikasi, Permintaan Saya, Persetujuan, Pesanan (PO), Laporan Realisasi |
| **Petugas Gudang** | Catat barang datang, buat BAST | Beranda, Notifikasi, Pesanan (PO), Penerimaan Barang, Serah Terima |
| **Kabag Keuangan** | Approval PO, realisasi | Beranda, Notifikasi, Pesanan (PO), Laporan Realisasi |
| **Admin Aset** | Data aset, mutasi, QR & stiker | Beranda, Notifikasi, Data Aset |
| **Administrator** | Semua + pengguna + pengaturan | Semua menu |
| **Auditor** | Melihat laporan & jejak audit (read-only) | Beranda, Notifikasi, Laporan Realisasi, Jejak Audit, Master Pengguna, Riwayat Pengaturan |

> Menu mengikuti matriks peran WORKFLOW §3.2/3.3 — menu operasional di luar
> kewenangan peran tidak ditampilkan (auditor tidak melihat Persetujuan/PO/
> Serah Terima/Data Aset; kasi tidak melihat Master Ruangan/Data Aset; kabag
> tidak mengelola Harga & Supplier). Tombol aksi hanya tampil bila API peran
> mengizinkannya.

## 2. Masuk dan keluar

1. Buka aplikasi (web/desktop) → halaman **Masuk Aplikasi**.
2. Isi **Nama Pengguna** & **Password** → **Masuk**.
   - Nama pengguna diberikan Administrator, mis. `admin`, `kasi`, `gudang`.
   - Daftar lengkap akun Anda: menu **Master Pengguna** (khusus Administrator).
3. Setelah 5× salah dalam 15 menit, login terkunci sementara (tunggu 15 menit).
4. Lupa password → minta Administrator untuk **Reset Password**
   (Master Pengguna → Reset Password).
5. Keluar: tombol **Keluar** di pojok kiri bawah.

> Akun awal (`admin` / password default dari seed) — **wajib diganti**
> setelah login pertama.

## 3. Mengajukan permintaan (User Ruangan)

1. Klik menu **Buat Permintaan** di sidebar (atau tombol
   **+ Buat Permintaan** di beranda).
2. **Ruangan** otomatis ruangan Anda (user ruangan). Kasi/Admin memilih ruangan.
3. **Cari Item** — ketik nama/kategori (mis. `AC`, `ALKES`) untuk menyaring
   246 item; pilih item → satuan terisi otomatis.
4. Isi **Jumlah** (harus > 0) dan keterangan bila perlu. **+ Tambah Baris**
   untuk item lain dalam satu pengajuan.
5. **Simpan Draft** (bisa dilanjutkan nanti) atau **Ajukan Permintaan**
   (langsung masuk antrean verifikasi).
6. Setelah diajukan: baris berubah menjadi **Menunggu verifikasi Kasi**, muncul
   hitungan SLA, dan Kasi Sarpras menerima notifikasi.

**Membaca status.** Setiap pengajuan menampilkan **stepper lima tahap**
(Draft → Kasi → Kabid → Wadir → Final) dengan tahap aktif ditandai, ditambah
teks "Menunggu: …" dan sisa waktu SLA (hijau/kuning/merah). Satu baris = satu
pengajuan; bila berisi beberapa item, item lain diringkas "+N item lain" dan
dirinci di bawahnya.

Catatan:
- Permintaan hanya bisa diajukan pada periode buka (lihat pengumuman
  manajemen; urgent di luar periode hanya via Kasi).
- Draft bisa **dibatalkan**; setelah diajukan, perubahan/batal dibatasi
  sesuai kebijakan (config E10/E11).

## 4. Menyetujui / menolak (Kasi → Kabid → Wadir)

1. Menu **Persetujuan** menampilkan badge jumlah yang menunggu Anda.
2. Klik salah satu baris → **Setujui** atau **Tolak**.
3. Dialog menampilkan **seluruh item dalam satu pengajuan** — keputusan
   berlaku untuk satu pengajuan sekaligus.
4. **Menolak wajib memberi catatan** (agar pemohon tahu yang harus diperbaiki);
   permintaan kembali ke pemohon sebagai draft.
5. Rantai berjenjang: Kasi → Kabid → Wadir. Setiap langkah punya batas SLA
   (default 5 hari kerja); lewat batas akan muncul peringatan dan eskalasi
   ke atasan.

## 4b. Revisi setelah ditolak

1. Saat permintaan ditolak, status menjadi **Ditolak — perlu perbaikan** dan
   Anda menerima notifikasi berisi catatan penolakan.
2. Di **Permintaan Saya**, baris tersebut memunculkan dua tombol:
   - **Ajukan Ulang** — langsung mengirim ulang apa adanya (status kembali ke
     verifikasi Kasi, nomor **Revisi** bertambah).
   - **Perbaiki** — membuka detail; klik **Ubah** pada baris untuk mengubah
     jumlah/keterangan, simpan, lalu **Ajukan Ulang Revisi**.
3. Semua jejak tetap terlihat di **Riwayat Pengajuan**: Diajukan → Ditolak
   (dengan catatan) → Diajukan ulang.

## 4c. Disposisi pimpinan

1. Wadir/Admin membuka **Detail Permintaan** → bagian **Disposisi Pimpinan**.
2. Tulis arahan (mis. "Setujui dengan catatan: cek anggaran") dan opsional
   pilih **pelaksana** yang ditunjuk menindaklanjuti.
3. Disposisi tidak mengubah posisi approval — tercatat di riwayat, dan
   pemohon + pelaksana menerima notifikasi.

## 5. Notifikasi

- Ikon lonceng di kanan atas menampilkan jumlah belum dibaca (refresh tiap
  30 detik).
- Klik lonceng → **Notifikasi** untuk daftar lengkap; tombol
  **Tandai semua dibaca** untuk menandai sekaligus.
- Notifikasi muncul saat: permintaan masuk, disetujui, ditolak, dan
  peringatan SLA.

## 6. Master data (Administrator / Kasi)

- **Master Ruangan** (Admin): tambah/ubah/nonaktifkan ruangan. Ruangan yang
  masih dipakai user aktif tidak bisa dinonaktifkan.
- **Master Item & Satuan** (Admin, Kasi): tambah/ubah/nonaktifkan item;
  `threshold_aset` menentukan kapan item jadi aset.
- **Master User** (Admin): buat user, pilih role & ruangan, reset password,
  nonaktifkan. Admin terakhir tidak bisa dinonaktifkan.

## 7. Pengaturan Sistem (Administrator)

- **Pengaturan Sistem** — 96 parameter dikelompokkan (Organisasi, Approval,
  SLA, PO, dst). Kolom **Nilai Aktif** + **Sumber** menunjukkan nilai efektif.
- Ubah parameter → dialog menampilkan **simulasi dampak** (mis. berapa record
  SLA terdampak) sebelum menyimpan.
- **12 parameter kritis** (ditandai "Kritis") tidak langsung berlaku:
  perubahan menunggu persetujuan approver di tab **Persetujuan**.
- **Riwayat Pengaturan** menampilkan seluruh riwayat perubahan (permanen).

## 8. Import data lama (Administrator)

Menu **Import Data** (grup Pengaturan di sidebar) memindahkan rekap Excel tahun
berjalan ke sistem sebagai **permintaan historis** — berguna agar pelaporan
langsung punya data tanpa entri ulang.

1. **Pratinjau** — sistem membaca `REKAPAN*.xlsx` dan menampilkan:
   baris terbaca, **cocok master**, tidak cocok, dan dilewati parser.
   Bila ada "tidak cocok", daftar nama ruangan/item yang belum ada di master
   ditampilkan; lengkapi master lalu muat ulang pratinjau.
2. Pilih **tahun anggaran** → klik **Impor N Baris**.
3. Baris tersimpan sebagai permintaan **CLOSED** (riwayat selesai).
4. **Riwayat Import** menampilkan tiap batch; tombol **Log Baris** memperlihatkan
   status per baris (`SUKSES` / `DUPLIKAT` / `GAGAL` beserta alasannya).

Aman dijalankan berulang: baris identik yang sudah pernah diimpor otomatis
ditandai **DUPLIKAT** dan tidak menggandakan data.

## 9. Pengadaan: supplier → penawaran → PO

Alur pengadaan setelah permintaan disetujui FINAL:

1. **Master Supplier** — tambah supplier (nama, NPWP, kontak, alamat), lalu klik
   **Katalog** untuk mencatat item apa saja yang disediakan + harga referensi.
2. **Harga & Supplier** — catat penawaran harga per supplier per item.
   Tabel perbandingan menandai harga **termurah** dan jumlah penawaran; minimal
   **3 penawaran** per item (config F12) sebelum boleh masuk PO.
3. Klik **Bandingkan** → pilih supplier → isi alasan → **Tunjuk Supplier Terpilih**.
4. **Pesanan (PO)** → **+ Buat PO**, pilih supplier, centang permintaan FINAL,
   lalu **Buat PO**. Harga diambil otomatis dari penawaran yang ditunjuk; PPN
   dihitung sesuai config F01.
5. PO berstatus Draft → **Setujui** (bila nilai ≥ threshold, hanya Kabag/Wadir/Admin)
   → **Kirim**.

## 10. Penerimaan & serah terima

1. Barang datang → **Penerimaan Barang** → pilih PO yang berstatus
   **Terkirim ke supplier** → **Catat penerimaan**. Dialog menampilkan item
   beserta **sisa yang belum diterima** (qty terisi otomatis). Isi qty terima,
   qty ditolak/rusak, kondisi, serta **no. DO / Invoice / Faktur Pajak**
   (dokumen wajib sesuai config F07 — tombol simpan akan menolak bila kosong).
2. Penerimaan boleh **parsial** (config F04); qty tidak boleh melebihi sisa
   (+ toleransi F05). PO otomatis menjadi **Diterima Sebagian** atau **Selesai**.
3. **Serah Terima** → pilih barang siap diserahkan → **Serahkan** → pilih
   ruangan penerima dan **nama petugas ruangan yang menerima** → **Buat BAST**.
4. Petugas ruangan yang namanya tercatat login → **Serah Terima** → tombol
   **Konfirmasi** (hanya muncul untuk penerima yang bersangkutan).
5. Bila tidak sesuai, ruangan dapat **dispute** dan Kasi menyelesaikan.

## 11. Aset

- Aset otomatis dibuat setelah serah terima **CONFIRMED** bila nilainya mencapai
  threshold (config G03 atau threshold item).
- **Registrasi aset** manual tersedia; kode aset mengikuti format A05.
- **Mutasi** memindahkan aset antar ruangan beserta riwayatnya.
- **QR label** menghasilkan payload `kode_aset|nama|ruangan` (config G05).
- Status aset: Terdaftar → Digunakan → Pemeliharaan → Dihapus.

## 12. Dashboard realisasi

**Dashboard Realisasi** menampilkan nilai PO, nilai diterima, nilai
diserahterimakan, jumlah/nilai aset, progres serah terima, PO per status, dan
supplier teratas per tahun anggaran.

## 13. Audit & riwayat

- **Audit Log** (Admin/Auditor): semua aksi create/update/delete/approve
  beserta nilai sebelum/sesudah. Tidak bisa dihapus.
- Riwayat approval per pengajuan tersedia di Inbox Approval (detail).

## 14. Aplikasi Android (operasional, per peran)

Aplikasi Android (`clients/android/`, F1 klien) ditujukan untuk tugas operasional harian (on point), bukan pengganti seluruh web/desktop.

- **Masuk & alamat server:** isi **Nama Pengguna**, **Kata Sandi**, dan **Alamat Server** (disimpan). Contoh: emulator `http://10.0.2.2:8080`, perangkat fisik `http://<IP-server>:8080`. Perangkat harus dapat menjangkau server API.
- **Beranda (Tugas Anda):** kartu tugas & badge ringkasan sesuai peran (data dari server). **Notifikasi:** daftar, tandai dibaca/semua dibaca, muat ulang (push FCM belum aktif; buka/muat ulang untuk terbaru). **Profil & keluar** (menghapus sesi & cache baca perangkat).
- **Fitur per peran (F1):**
  - **User Ruangan:** buat/ajukan & pantau permintaan, konfirmasi BAST atau laporkan masalah (dispute).
  - **Kasi/Kabid/Wadir (approver):** persetujuan permintaan (setujui/tolak, catatan wajib saat tolak); Kasi/Wadir juga tindak PO sesuai kewenangan.
  - **Kabag/Wadir (+Kasi/Admin):** persetujuan/kirim PO sesuai kewenangan & status.
  - **Petugas Gudang:** catat penerimaan barang (qty/kondisi, **No. DO/Invoice/Faktur Pajak wajib**, foto dokumen opsional), buat BAST/serah terima.
  - **Admin:** akses tugas operasional yang relevan di mobile; **Admin Aset/Auditor:** operasional mobile terbatas (tetap gunakan web/desktop untuk fungsi utama perannya).
- **Tetap di web/desktop:** master data, pengaturan sistem & approval config, import, pengadaan detail (katalog/penawaran/buat PO), audit trail, laporan/cetak, registrasi/mutasi aset penuh, stok opname.
- **Catatan:** mutasi online-first dan divalidasi server (RBAC). Saat offline/gagal jaringan, daftar dapat tampil dari cache baca (bertanda data terakhir).

## 9. FAQ

**Permintaan ditolak — apa selanjutnya?**
Buka Permintaan Anda; permintaan kembali menjadi draft dengan catatan
penolakan. Perbaiki lalu ajukan ulang.

**Kenapa tombol Ajukan tidak ada di baris draft?**
Draft itu milik pengguna lain. Anda hanya bisa mengajukan draft milik Anda.

**Apa arti warna pada tanggal SLA?**
Kuning = mendekati batas (H-1/H-2); merah = lewat batas.

**Bisakah satu pengajuan berisi banyak item?**
Ya — gunakan **+ Tambah Baris**. Satu pengajuan = satu keputusan approval.

**Notifikasi tidak muncul?**
Aplikasi web memeriksa tiap 30 detik; di Android buka tab Notifikasi/muat ulang
(push FCM belum aktif). Pastikan Anda login sebagai role penerima (mis. Kasi
untuk permintaan baru).
