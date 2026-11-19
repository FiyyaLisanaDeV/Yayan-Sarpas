-- 0004_config.sql — CONFIG
-- config_group, config_item, config_value, config_audit, config_snapshot,
-- hari_libur, config_change_request (tambahan untuk approval config kritis)

CREATE TABLE config_group (
    id        BIGSERIAL PRIMARY KEY,
    kode      TEXT NOT NULL UNIQUE,
    nama      TEXT NOT NULL,
    deskripsi TEXT,
    urutan    INT NOT NULL DEFAULT 0
);

CREATE TABLE config_item (
    id              BIGSERIAL PRIMARY KEY,
    group_id        BIGINT NOT NULL REFERENCES config_group(id),
    kode            TEXT NOT NULL UNIQUE,
    label           TEXT NOT NULL,
    tipe            TEXT NOT NULL CHECK (tipe IN (
                        'text','number','decimal','boolean','enum','json',
                        'date','duration_days','url','email','color','template')),
    nilai_default   TEXT NOT NULL,
    scope           TEXT NOT NULL DEFAULT 'GLOBAL'
                        CHECK (scope IN ('GLOBAL','KATEGORI','RUANGAN')),
    validation_json JSONB,
    is_sensitive    BOOLEAN NOT NULL DEFAULT FALSE,
    approver_roles  JSONB NOT NULL DEFAULT '["ADMIN"]',
    urutan          INT NOT NULL DEFAULT 0
);

CREATE TABLE config_value (
    id             BIGSERIAL PRIMARY KEY,
    config_item_id BIGINT NOT NULL REFERENCES config_item(id),
    scope_type     TEXT NOT NULL CHECK (scope_type IN ('GLOBAL','KATEGORI','RUANGAN')),
    scope_id       BIGINT,
    nilai          TEXT NOT NULL,
    updated_by     BIGINT REFERENCES users(id),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    alasan         TEXT
);

CREATE UNIQUE INDEX uq_config_value_item_scope
    ON config_value (config_item_id, scope_type, COALESCE(scope_id, 0));

CREATE TABLE config_audit (
    id             BIGSERIAL PRIMARY KEY,
    config_item_id BIGINT NOT NULL REFERENCES config_item(id),
    scope_type     TEXT NOT NULL CHECK (scope_type IN ('GLOBAL','KATEGORI','RUANGAN')),
    scope_id       BIGINT,
    nilai_lama     TEXT,
    nilai_baru     TEXT,
    changed_by     TEXT NOT NULL,
    changed_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    alasan         TEXT
);

CREATE TABLE config_snapshot (
    id             BIGSERIAL PRIMARY KEY,
    ref_type       TEXT NOT NULL,
    ref_id         BIGINT NOT NULL,
    config_item_id BIGINT NOT NULL REFERENCES config_item(id),
    nilai          TEXT NOT NULL
);

CREATE UNIQUE INDEX uq_config_snapshot
    ON config_snapshot (ref_type, ref_id, config_item_id);

CREATE TABLE hari_libur (
    id      BIGSERIAL PRIMARY KEY,
    tanggal DATE NOT NULL,
    nama    TEXT NOT NULL,
    tahun   SMALLINT NOT NULL
);

CREATE UNIQUE INDEX uq_hari_libur_tanggal ON hari_libur (tanggal);

-- Tabel TAMBAHAN (di luar spec §3): tempat menyimpan pending state
-- perubahan config kritis yang butuh approval.
CREATE TABLE config_change_request (
    id             BIGSERIAL PRIMARY KEY,
    config_item_id BIGINT NOT NULL REFERENCES config_item(id),
    scope_type     TEXT NOT NULL DEFAULT 'GLOBAL'
                       CHECK (scope_type IN ('GLOBAL','KATEGORI','RUANGAN')),
    scope_id       BIGINT,
    nilai_baru     TEXT NOT NULL,
    alasan         TEXT,
    status         TEXT NOT NULL DEFAULT 'PENDING'
                       CHECK (status IN ('PENDING','APPROVED','REJECTED')),
    requested_by   BIGINT NOT NULL REFERENCES users(id),
    approved_by    BIGINT REFERENCES users(id),
    approved_at    TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
