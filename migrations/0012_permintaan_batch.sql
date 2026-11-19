-- 0012_permintaan_batch.sql — pengelompokan baris permintaan.
--
-- Form permintaan bersifat multi-row: satu kali kirim berisi banyak item.
-- Karena tabel `permintaan` menyimpan satu baris per item (sesuai spesifikasi
-- §3), baris-baris dari satu pengiriman perlu penanda bersama agar dapat
-- ditampilkan, disubmit, dan (Fase 3) di-approve sebagai satu kesatuan.

ALTER TABLE permintaan ADD COLUMN batch_id UUID;

CREATE INDEX idx_permintaan_batch ON permintaan (batch_id);
