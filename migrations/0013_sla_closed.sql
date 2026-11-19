-- 0013_sla_closed.sql — penutupan SLA.
--
-- sla_tracking diisi saat baris permintaan masuk sebuah step approval.
-- Tanpa kolom penutup, semua tracking lama tetap terlihat "aktif" sehingga
-- job eskalasi akan terus menandai record yang sudah selesai diproses.

ALTER TABLE sla_tracking ADD COLUMN closed_at TIMESTAMPTZ;

-- Job eskalasi hanya memindai tracking yang masih terbuka.
CREATE INDEX idx_sla_tracking_open ON sla_tracking (status, deadline)
    WHERE closed_at IS NULL;
