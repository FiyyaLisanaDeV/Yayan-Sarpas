-- 0001_master.sql — tabel MASTER
-- users, ruangan, kategori, satuan, items, supplier, ref_status

CREATE TABLE users (
    id            BIGSERIAL PRIMARY KEY,
    nama          TEXT NOT NULL,
    nip           TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN (
                      'USER_RUANGAN','KASI_SARPRAS','KABID_YANMED','WADIR_YANMED',
                      'PETUGAS_GUDANG','KABAG_KEUANGAN','ADMIN_ASET','ADMIN','AUDITOR')),
    ruangan_id    BIGINT,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    mfa_secret    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ruangan (
    id        BIGSERIAL PRIMARY KEY,
    kode      TEXT NOT NULL UNIQUE,
    nama      TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE kategori (
    id   BIGSERIAL PRIMARY KEY,
    kode TEXT NOT NULL UNIQUE CHECK (kode IN ('ALKES','SAPRAS')),
    nama TEXT NOT NULL
);

CREATE TABLE satuan (
    id   BIGSERIAL PRIMARY KEY,
    kode TEXT NOT NULL UNIQUE,
    nama TEXT NOT NULL
);

CREATE TABLE items (
    id             BIGSERIAL PRIMARY KEY,
    nama           TEXT NOT NULL,
    satuan_id      BIGINT NOT NULL REFERENCES satuan(id),
    kategori_id    BIGINT NOT NULL REFERENCES kategori(id),
    threshold_aset NUMERIC(18,2),
    is_active      BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE supplier (
    id        BIGSERIAL PRIMARY KEY,
    nama      TEXT NOT NULL,
    npwp      TEXT,
    kontak    TEXT,
    alamat    TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE ref_status (
    id     BIGSERIAL PRIMARY KEY,
    kode   TEXT NOT NULL UNIQUE,
    label  TEXT NOT NULL,
    warna  TEXT,
    urutan INT NOT NULL DEFAULT 0
);
