-- 0008_indexes_triggers.sql — index & trigger
-- Index di FK, status, tahun, updated_at + trigger updated_at otomatis

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_permintaan_updated BEFORE UPDATE ON permintaan
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_po_header_updated BEFORE UPDATE ON po_header
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_penerimaan_updated BEFORE UPDATE ON penerimaan
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_serah_terima_updated BEFORE UPDATE ON serah_terima
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_config_value_updated BEFORE UPDATE ON config_value
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- index master / FK
CREATE INDEX idx_users_ruangan ON users (ruangan_id);
CREATE INDEX idx_users_role ON users (role);
CREATE INDEX idx_items_satuan ON items (satuan_id);
CREATE INDEX idx_items_kategori ON items (kategori_id);

-- index permintaan
CREATE INDEX idx_permintaan_tahun ON permintaan (tahun);
CREATE INDEX idx_permintaan_ruangan ON permintaan (ruangan_id);
CREATE INDEX idx_permintaan_item ON permintaan (item_id);
CREATE INDEX idx_permintaan_status ON permintaan (status);
CREATE INDEX idx_permintaan_created_by ON permintaan (created_by);
CREATE INDEX idx_permintaan_updated ON permintaan (updated_at);
CREATE INDEX idx_approval_log_permintaan ON approval_log (permintaan_id);
CREATE INDEX idx_attachments_ref ON attachments (ref_type, ref_id);
CREATE INDEX idx_notifikasi_user_read ON notifikasi (user_id, is_read);
CREATE INDEX idx_request_item_ruangan ON request_item (ruangan_id);

-- index realisasi
CREATE INDEX idx_po_header_tahun ON po_header (tahun);
CREATE INDEX idx_po_header_supplier ON po_header (supplier_id);
CREATE INDEX idx_po_header_status ON po_header (status);
CREATE INDEX idx_po_header_updated ON po_header (updated_at);
CREATE INDEX idx_po_detail_header ON po_detail (po_header_id);
CREATE INDEX idx_po_detail_permintaan ON po_detail (permintaan_id);
CREATE INDEX idx_po_detail_item ON po_detail (item_id);
CREATE INDEX idx_penerimaan_po_detail ON penerimaan (po_detail_id);
CREATE INDEX idx_penerimaan_updated ON penerimaan (updated_at);
CREATE INDEX idx_serah_terima_penerimaan ON serah_terima (penerimaan_id);
CREATE INDEX idx_serah_terima_ruangan ON serah_terima (ruangan_id);
CREATE INDEX idx_aset_tetap_item ON aset_tetap (item_id);
CREATE INDEX idx_aset_tetap_ruangan ON aset_tetap (ruangan_id);
CREATE INDEX idx_aset_tetap_status ON aset_tetap (status);
CREATE INDEX idx_mutasi_aset_aset ON mutasi_aset (aset_id);
CREATE INDEX idx_stok_opname_tahun ON stok_opname (tahun);
CREATE INDEX idx_stok_opname_item ON stok_opname (item_id);

-- index config
CREATE INDEX idx_config_item_group ON config_item (group_id);
CREATE INDEX idx_config_value_item ON config_value (config_item_id);
CREATE INDEX idx_config_audit_item ON config_audit (config_item_id);
CREATE INDEX idx_config_change_request_status ON config_change_request (status);
CREATE INDEX idx_hari_libur_tahun ON hari_libur (tahun);

-- index sla
CREATE INDEX idx_sla_tracking_ref ON sla_tracking (ref_type, ref_id);
CREATE INDEX idx_sla_tracking_deadline ON sla_tracking (deadline);

-- index sync & import
CREATE INDEX idx_import_row_log_batch ON import_row_log (batch_id);

-- index audit
CREATE INDEX idx_audit_trail_tabel ON audit_trail (tabel, record_id);
CREATE INDEX idx_audit_trail_timestamp ON audit_trail (timestamp);
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens (token_hash);
