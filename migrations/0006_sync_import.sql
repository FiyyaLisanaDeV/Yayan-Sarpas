-- 0006_sync_import.sql — SYNC & IMPORT
-- sync_conflict, import_batch, import_row_log

CREATE TABLE sync_conflict (
    id            BIGSERIAL PRIMARY KEY,
    client_id     TEXT NOT NULL,
    tabel         TEXT NOT NULL,
    record_id     BIGINT NOT NULL,
    field         TEXT NOT NULL,
    client_value  TEXT,
    server_value  TEXT,
    resolved_by   BIGINT REFERENCES users(id),
    resolved_at   TIMESTAMPTZ
);

CREATE TABLE import_batch (
    id         BIGSERIAL PRIMARY KEY,
    tipe       TEXT NOT NULL,
    file_path  TEXT NOT NULL,
    total_row  INT NOT NULL DEFAULT 0,
    sukses     INT NOT NULL DEFAULT 0,
    gagal      INT NOT NULL DEFAULT 0,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE import_row_log (
    id        BIGSERIAL PRIMARY KEY,
    batch_id  BIGINT NOT NULL REFERENCES import_batch(id),
    row_no    INT NOT NULL,
    status    TEXT NOT NULL,
    error_msg TEXT,
    raw_json  JSONB
);
