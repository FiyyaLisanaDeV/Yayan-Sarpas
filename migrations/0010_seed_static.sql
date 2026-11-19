-- 0010_seed_static.sql — seed statis
-- kategori, satuan, ref_status, config_group, hari libur 2026 (SKB 3 Menteri)
-- Catatan: 68 config_item + config_value + config_audit di-seed oleh binary
-- `sarpras-seed` (Rust) agar logika approval kritis & validasi terpusat.

INSERT INTO kategori (kode, nama) VALUES
    ('ALKES', 'Alat Kesehatan'),
    ('SAPRAS', 'Sarana Prasarana');

INSERT INTO satuan (kode, nama) VALUES
    ('BUAH', 'Buah'),
    ('UNIT', 'Unit'),
    ('BOX', 'Box'),
    ('PCS', 'pcs'),
    ('SET', 'set'),
    ('BATANG', 'Batang'),
    ('DOS', 'Dos'),
    ('LEMBAR', 'Lembar'),
    ('KALENG', 'Kaleng'),
    ('BOTOL', 'Botol'),
    ('PINTU', 'pintu');

-- ref_status: kode unik global (§4 state machine)
INSERT INTO ref_status (kode, label, warna, urutan) VALUES
    -- Permintaan
    ('DRAFT', 'Draft', '#9E9E9E', 1),
    ('SUBMITTED', 'Diajukan', '#1E88E5', 2),
    ('VERIFIKASI_KASI', 'Verifikasi Kasi', '#FFC107', 3),
    ('APPROVED_KABID', 'Disetujui Kabid', '#4CAF50', 4),
    ('APPROVED_WADIR', 'Disetujui Wadir', '#4CAF50', 5),
    ('FINAL', 'Final', '#4CAF50', 6),
    ('IN_PO', 'Masuk PO', '#1E88E5', 7),
    ('RECEIVED', 'Diterima', '#4CAF50', 8),
    ('HANDED_OVER', 'Diserahterimakan', '#4CAF50', 9),
    ('CLOSED', 'Ditutup', '#FFFFFF', 10),
    ('RETURNED', 'Dikembalikan', '#F44336', 11),
    -- PO
    ('APPROVED', 'Disetujui', '#4CAF50', 20),
    ('SENT', 'Terkirim', '#1E88E5', 21),
    ('PARTIAL_RECEIVED', 'Diterima Sebagian', '#FFC107', 22),
    -- Serah terima
    ('PENDING', 'Menunggu Konfirmasi', '#FFC107', 30),
    ('CONFIRMED', 'Dikonfirmasi', '#4CAF50', 31),
    ('DISPUTED', 'Berselisih', '#F44336', 32),
    -- Aset
    ('REGISTERED', 'Terdaftar', '#1E88E5', 40),
    ('IN_USE', 'Digunakan', '#4CAF50', 41),
    ('MAINTAINED', 'Pemeliharaan', '#FFC107', 42),
    ('DISPOSED', 'Dihapus', '#F44336', 43);

INSERT INTO config_group (kode, nama, deskripsi, urutan) VALUES
    ('org', 'Organisasi', 'Identitas RS, format dokumen & tampilan', 1),
    ('approval', 'Approval', 'Rantai persetujuan permintaan', 2),
    ('sla', 'SLA', 'Batas waktu layanan & eskalasi', 3),
    ('notif', 'Notifikasi', 'Channel & penerima notifikasi', 4),
    ('permintaan', 'Permintaan', 'Periode & aturan permintaan', 5),
    ('po', 'PO', 'Purchase order & penerimaan', 6),
    ('serah', 'Serah Terima', 'Serah terima, aset & mutasi', 7),
    ('offline', 'Offline', 'Mode offline & sync Tauri', 8),
    ('import', 'Import', 'Import data lama', 9),
    ('cetak', 'Cetak', 'Dokumen, watermark & format nomor', 10),
    ('security', 'Keamanan', 'JWT, password, MFA & rate limit', 11),
    ('retention', 'Retensi', 'Retensi data & backup', 12),
    ('ui', 'UI', 'Tampilan & preferensi UI', 13);

-- Hari libur nasional & cuti bersama 2026 (SKB 3 Menteri No. 1497/2/5 Tahun 2025)
INSERT INTO hari_libur (tanggal, nama, tahun) VALUES
    ('2026-01-01', 'Tahun Baru Masehi 2026', 2026),
    ('2026-01-16', 'Isra Mikraj Nabi Muhammad SAW', 2026),
    ('2026-02-16', 'Cuti Bersama Tahun Baru Imlek 2577 Kongzili', 2026),
    ('2026-02-17', 'Tahun Baru Imlek 2577 Kongzili', 2026),
    ('2026-03-18', 'Cuti Bersama Hari Suci Nyepi Tahun Baru Saka 1948', 2026),
    ('2026-03-19', 'Hari Suci Nyepi Tahun Baru Saka 1948', 2026),
    ('2026-03-20', 'Cuti Bersama Hari Raya Idulfitri 1447 H', 2026),
    ('2026-03-21', 'Hari Raya Idulfitri 1447 H', 2026),
    ('2026-03-22', 'Hari Raya Idulfitri 1447 H', 2026),
    ('2026-03-23', 'Cuti Bersama Hari Raya Idulfitri 1447 H', 2026),
    ('2026-03-24', 'Cuti Bersama Hari Raya Idulfitri 1447 H', 2026),
    ('2026-04-03', 'Wafat Yesus Kristus', 2026),
    ('2026-04-05', 'Hari Paskah', 2026),
    ('2026-05-01', 'Hari Buruh Internasional', 2026),
    ('2026-05-14', 'Kenaikan Yesus Kristus', 2026),
    ('2026-05-15', 'Cuti Bersama Kenaikan Yesus Kristus', 2026),
    ('2026-05-27', 'Hari Raya Iduladha 1447 H', 2026),
    ('2026-05-28', 'Cuti Bersama Hari Raya Iduladha 1447 H', 2026),
    ('2026-05-31', 'Hari Raya Waisak 2570', 2026),
    ('2026-06-01', 'Hari Lahir Pancasila', 2026),
    ('2026-06-16', 'Tahun Baru Islam 1448 H', 2026),
    ('2026-08-17', 'Hari Kemerdekaan Republik Indonesia', 2026),
    ('2026-08-25', 'Maulid Nabi Muhammad SAW', 2026),
    ('2026-12-24', 'Cuti Bersama Hari Raya Natal', 2026),
    ('2026-12-25', 'Hari Raya Natal', 2026);
