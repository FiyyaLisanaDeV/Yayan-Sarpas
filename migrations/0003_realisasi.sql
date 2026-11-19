-- 0003_realisasi.sql — REALISASI
-- po_header, po_detail, penerimaan, serah_terima, aset_tetap, mutasi_aset, stok_opname

CREATE TABLE po_header (
    id           BIGSERIAL PRIMARY KEY,
    no_po        TEXT NOT NULL UNIQUE,
    tahun        SMALLINT NOT NULL,
    supplier_id  BIGINT NOT NULL REFERENCES supplier(id),
    tanggal_po   DATE NOT NULL,
    nilai_total  NUMERIC(18,2) NOT NULL DEFAULT 0,
    ppn          NUMERIC(18,2) NOT NULL DEFAULT 0,
    status       TEXT NOT NULL DEFAULT 'DRAFT',
    created_by   BIGINT NOT NULL REFERENCES users(id),
    approved_by  BIGINT REFERENCES users(id),
    approved_at  TIMESTAMPTZ,
    catatan      TEXT,
    sla_deadline TIMESTAMPTZ,
    sla_status   TEXT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE po_detail (
    id            BIGSERIAL PRIMARY KEY,
    po_header_id  BIGINT NOT NULL REFERENCES po_header(id),
    permintaan_id BIGINT REFERENCES permintaan(id),
    item_id       BIGINT NOT NULL REFERENCES items(id),
    qty           NUMERIC(18,2) NOT NULL,
    harga_satuan  NUMERIC(18,2) NOT NULL,
    subtotal      NUMERIC(18,2) NOT NULL,
    satuan_id     BIGINT NOT NULL REFERENCES satuan(id)
);

CREATE TABLE penerimaan (
    id               BIGSERIAL PRIMARY KEY,
    po_detail_id     BIGINT NOT NULL REFERENCES po_detail(id),
    no_terima        TEXT NOT NULL UNIQUE,
    tanggal_terima   DATE NOT NULL,
    qty_terima       NUMERIC(18,2) NOT NULL,
    qty_reject       NUMERIC(18,2) NOT NULL DEFAULT 0,
    kondisi          TEXT,
    no_do            TEXT,
    no_invoice       TEXT,
    no_faktur_pajak  TEXT,
    diterima_oleh    BIGINT NOT NULL REFERENCES users(id),
    catatan          TEXT,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE serah_terima (
    id                   BIGSERIAL PRIMARY KEY,
    penerimaan_id        BIGINT NOT NULL REFERENCES penerimaan(id),
    ruangan_id           BIGINT NOT NULL REFERENCES ruangan(id),
    tanggal_serah        DATE NOT NULL,
    qty_serah            NUMERIC(18,2) NOT NULL,
    penerima_ruangan_id  BIGINT NOT NULL REFERENCES users(id),
    pemberi_id           BIGINT NOT NULL REFERENCES users(id),
    no_ba                TEXT,
    file_ba              TEXT,
    catatan              TEXT,
    status_konfirmasi    TEXT NOT NULL DEFAULT 'PENDING',
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE aset_tetap (
    id             BIGSERIAL PRIMARY KEY,
    item_id        BIGINT NOT NULL REFERENCES items(id),
    ruangan_id     BIGINT NOT NULL REFERENCES ruangan(id),
    kode_aset      TEXT NOT NULL UNIQUE,
    no_seri        TEXT,
    tgl_perolehan  DATE NOT NULL,
    nilai          NUMERIC(18,2) NOT NULL,
    umur_ekonomis  SMALLINT,
    kondisi        TEXT,
    status         TEXT NOT NULL DEFAULT 'REGISTERED',
    qr_path        TEXT
);

CREATE TABLE mutasi_aset (
    id               BIGSERIAL PRIMARY KEY,
    aset_id          BIGINT NOT NULL REFERENCES aset_tetap(id),
    dari_ruangan_id  BIGINT NOT NULL REFERENCES ruangan(id),
    ke_ruangan_id    BIGINT NOT NULL REFERENCES ruangan(id),
    tanggal          DATE NOT NULL,
    alasan           TEXT,
    oleh_id          BIGINT NOT NULL REFERENCES users(id)
);

CREATE TABLE stok_opname (
    id          BIGSERIAL PRIMARY KEY,
    tahun       SMALLINT NOT NULL,
    periode     TEXT NOT NULL,
    item_id     BIGINT NOT NULL REFERENCES items(id),
    ruangan_id  BIGINT NOT NULL REFERENCES ruangan(id),
    stok_fisik  NUMERIC(18,2) NOT NULL,
    stok_sistem NUMERIC(18,2) NOT NULL,
    selisih     NUMERIC(18,2) NOT NULL,
    catatan     TEXT,
    oleh_id     BIGINT NOT NULL REFERENCES users(id)
);
