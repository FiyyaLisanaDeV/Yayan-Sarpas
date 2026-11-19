-- Disposisi pimpinan: catatan arahan (dan opsional penunjukan pelaksana)
-- pada satu pengajuan. Tidak mengubah langkah approval — tercatat + notifikasi.
CREATE TABLE IF NOT EXISTS disposisi (
    id BIGSERIAL PRIMARY KEY,
    batch_id UUID NOT NULL,
    dari_user_id BIGINT NOT NULL REFERENCES users(id),
    untuk_user_id BIGINT REFERENCES users(id),
    status_asal TEXT NOT NULL,
    isi TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_disposisi_batch ON disposisi (batch_id);
