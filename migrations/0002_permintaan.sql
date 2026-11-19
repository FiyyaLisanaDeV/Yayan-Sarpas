-- 0002_permintaan.sql — PERMINTAAN
-- permintaan, approval_log, attachments, notifikasi, request_item

CREATE TABLE permintaan (
    id           BIGSERIAL PRIMARY KEY,
    tahun        SMALLINT NOT NULL,
    ruangan_id   BIGINT NOT NULL REFERENCES ruangan(id),
    item_id      BIGINT NOT NULL REFERENCES items(id),
    jumlah       NUMERIC(18,2) NOT NULL,
    satuan_id    BIGINT NOT NULL REFERENCES satuan(id),
    keterangan   TEXT,
    status       TEXT NOT NULL DEFAULT 'DRAFT',
    current_step SMALLINT NOT NULL DEFAULT 0,
    sla_deadline TIMESTAMPTZ,
    sla_status   TEXT,
    created_by   BIGINT NOT NULL REFERENCES users(id),
    submitted_at TIMESTAMPTZ,
    closed_at    TIMESTAMPTZ,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    client_id    TEXT
);

CREATE TABLE approval_log (
    id            BIGSERIAL PRIMARY KEY,
    permintaan_id BIGINT NOT NULL REFERENCES permintaan(id),
    step          SMALLINT NOT NULL,
    actor_id      BIGINT NOT NULL REFERENCES users(id),
    aksi          TEXT NOT NULL,
    catatan       TEXT,
    timestamp     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE attachments (
    id          BIGSERIAL PRIMARY KEY,
    ref_type    TEXT NOT NULL,
    ref_id      BIGINT NOT NULL,
    path        TEXT NOT NULL,
    jenis       TEXT,
    uploaded_by BIGINT NOT NULL REFERENCES users(id),
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE notifikasi (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id),
    tipe       TEXT NOT NULL,
    ref_id     BIGINT,
    pesan      TEXT NOT NULL,
    is_read    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE request_item (
    id             BIGSERIAL PRIMARY KEY,
    nama_usulan    TEXT NOT NULL,
    satuan_usulan  TEXT,
    alasan         TEXT,
    ruangan_id     BIGINT NOT NULL REFERENCES ruangan(id),
    requested_by   BIGINT NOT NULL REFERENCES users(id),
    status         TEXT NOT NULL DEFAULT 'PENDING',
    approved_by    BIGINT REFERENCES users(id),
    approved_at    TIMESTAMPTZ,
    item_id_hasil  BIGINT REFERENCES items(id)
);
