-- 0014_supplier_penawaran.sql — manajemen supplier lengkap (Fase 8).
--
-- Spec §3 hanya punya tabel `supplier`. Alur pengadaan membutuhkan:
--   1. `supplier_item`  — katalog: supplier mana menjual item apa + harga referensi
--   2. `penawaran`      — riwayat penawaran per item → dasar perbandingan & penunjukan

CREATE TABLE supplier_item (
    id               BIGSERIAL PRIMARY KEY,
    supplier_id      BIGINT NOT NULL REFERENCES supplier(id),
    item_id          BIGINT NOT NULL REFERENCES items(id),
    harga_referensi  NUMERIC(18,2),
    catatan          TEXT,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX uq_supplier_item ON supplier_item (supplier_id, item_id);
CREATE INDEX idx_supplier_item_item ON supplier_item (item_id);

CREATE TABLE penawaran (
    id                 BIGSERIAL PRIMARY KEY,
    supplier_id        BIGINT NOT NULL REFERENCES supplier(id),
    item_id            BIGINT NOT NULL REFERENCES items(id),
    harga_satuan       NUMERIC(18,2) NOT NULL,
    tanggal_penawaran  DATE NOT NULL,
    sumber             TEXT,
    status             TEXT NOT NULL DEFAULT 'DIAJUKAN'
                           CHECK (status IN ('DIAJUKAN','DIPILIH','TIDAK_DIPILIH')),
    alasan_penunjukan  TEXT,
    catatan            TEXT,
    created_by         BIGINT NOT NULL REFERENCES users(id),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_penawaran_item ON penawaran (item_id, status);
CREATE INDEX idx_penawaran_supplier ON penawaran (supplier_id);

-- Config F12: minimal penawaran per item sebelum boleh masuk PO.
INSERT INTO config_item (group_id, kode, label, tipe, nilai_default, scope, validation_json, approver_roles, urutan)
SELECT id, 'F12', 'Minimal Penawaran per Item', 'number', '3', 'GLOBAL',
       '{"min":0,"max":999999}'::jsonb, '["KABID_YANMED","ADMIN"]'::jsonb, 12
FROM config_group WHERE kode = 'po'
ON CONFLICT (kode) DO NOTHING;

INSERT INTO config_value (config_item_id, scope_type, scope_id, nilai, alasan)
SELECT id, 'GLOBAL', NULL, '3', 'INITIAL_SEED'
FROM config_item WHERE kode = 'F12'
  AND NOT EXISTS (
      SELECT 1 FROM config_value cv
      WHERE cv.config_item_id = config_item.id AND cv.scope_type = 'GLOBAL' AND cv.scope_id IS NULL
  );

INSERT INTO config_audit (config_item_id, scope_type, scope_id, nilai_lama, nilai_baru, changed_by, alasan)
SELECT id, 'GLOBAL', NULL, NULL, '3', 'SYSTEM', 'INITIAL_SEED'
FROM config_item WHERE kode = 'F12'
  AND NOT EXISTS (
      SELECT 1 FROM config_audit ca WHERE ca.config_item_id = config_item.id AND ca.alasan = 'INITIAL_SEED'
  );
