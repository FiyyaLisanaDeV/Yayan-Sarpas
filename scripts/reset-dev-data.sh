#!/usr/bin/env bash
# Bersihkan residu uji/manual pada database DEV.
# TIDAK menghapus config, master data, atau user — hanya jejak transaksi/uji.
set -euo pipefail

DB_URL="${DATABASE_URL:-postgres://sarpras:sarpras_dev_pw@localhost:5432/sarpras_dev}"
UPLOAD_DIR="${UPLOAD_DIR:-./data/uploads}"

echo "Database : $DB_URL"
echo "Upload   : $UPLOAD_DIR"
echo
echo "Yang akan dihapus:"
echo "  - config_change_request (semua)"
echo "  - config_audit KECUALI INITIAL_SEED"
echo "  - audit_trail (semua)"
echo "  - refresh_tokens (semua)"
echo "  - attachments (semua) + isi direktori upload"
echo
read -r -p "Lanjutkan? [y/N] " jawab
case "$jawab" in
  y|Y) ;;
  *) echo "Dibatalkan."; exit 0 ;;
esac

psql "$DB_URL" -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;
DELETE FROM config_change_request;
DELETE FROM config_audit WHERE alasan IS DISTINCT FROM 'INITIAL_SEED';
DELETE FROM audit_trail;
DELETE FROM refresh_tokens;
DELETE FROM attachments;
COMMIT;
SQL

if [ -d "$UPLOAD_DIR" ]; then
  find "$UPLOAD_DIR" -mindepth 1 -delete
  echo "Isi $UPLOAD_DIR dibersihkan."
fi

echo
echo "Ringkasan sisa data:"
psql "$DB_URL" -tAc "SELECT '  config_item='||(SELECT count(*) FROM config_item)
  ||' config_audit='||(SELECT count(*) FROM config_audit)
  ||' audit_trail='||(SELECT count(*) FROM audit_trail)
  ||' change_request='||(SELECT count(*) FROM config_change_request)
  ||' ruangan='||(SELECT count(*) FROM ruangan)
  ||' items='||(SELECT count(*) FROM items);"
echo "Selesai."
