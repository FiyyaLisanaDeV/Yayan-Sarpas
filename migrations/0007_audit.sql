-- 0007_audit.sql — AUDIT
-- audit_trail, print_log, ttd_elektronik,
-- refresh_tokens (tambahan untuk rotasi refresh JWT + logout)

CREATE TABLE audit_trail (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id),
    tabel       TEXT NOT NULL,
    record_id   BIGINT NOT NULL,
    aksi        TEXT NOT NULL,
    before_json JSONB,
    after_json  JSONB,
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE print_log (
    id            BIGSERIAL PRIMARY KEY,
    ref_type      TEXT NOT NULL,
    ref_id        BIGINT NOT NULL,
    jenis_dokumen TEXT NOT NULL,
    printed_by    BIGINT NOT NULL REFERENCES users(id),
    printed_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ttd_elektronik (
    id         BIGSERIAL PRIMARY KEY,
    ref_type   TEXT NOT NULL,
    ref_id     BIGINT NOT NULL,
    path       TEXT NOT NULL,
    hash       TEXT NOT NULL,
    signed_by  BIGINT NOT NULL REFERENCES users(id),
    signed_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE refresh_tokens (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id),
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
