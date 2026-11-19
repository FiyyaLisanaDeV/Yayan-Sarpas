-- Revisi permintaan: nomor revisi bertambah setiap submit ulang setelah penolakan.
ALTER TABLE permintaan ADD COLUMN IF NOT EXISTS revisi_ke INTEGER NOT NULL DEFAULT 1;
