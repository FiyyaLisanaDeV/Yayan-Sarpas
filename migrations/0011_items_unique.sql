-- 0011_items_unique.sql — idempotensi seed item.
--
-- Excel sumber memuat nama barang yang sama di kategori berbeda
-- (mis. "Lemari" ada di ALKES dan SAPRAS), tetapi tidak boleh duplikat
-- di dalam satu kategori. Index ini membuat seed dapat dijalankan
-- berulang tanpa menggandakan data (ON CONFLICT DO NOTHING).

CREATE UNIQUE INDEX uq_items_nama_kategori ON items (nama, kategori_id);
