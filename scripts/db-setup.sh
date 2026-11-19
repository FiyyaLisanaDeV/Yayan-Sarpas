#!/usr/bin/env bash
# Setup database untuk SARPRAS & ALKES (tanpa Docker, PostgreSQL native via Homebrew)
# Idempotent: aman dijalankan berulang kali.
set -euo pipefail

DB_USER="${DB_USER:-sarpras}"
DB_PASSWORD="${DB_PASSWORD:-sarpras_dev_pw}"
DB_DEV="${DB_DEV:-sarpras_dev}"
DB_TEST="${DB_TEST:-sarpras_test}"

echo "==> Cek PostgreSQL berjalan..."
if ! pg_isready -q; then
  echo "ERROR: PostgreSQL belum berjalan. Jalankan: brew services start postgresql@16"
  exit 1
fi

echo "==> Buat role '${DB_USER}' jika belum ada..."
if ! psql -d postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='${DB_USER}'" | grep -q 1; then
  psql -d postgres -v ON_ERROR_STOP=1 -c "CREATE ROLE ${DB_USER} LOGIN PASSWORD '${DB_PASSWORD}' CREATEDB;"
  echo "    Role dibuat."
else
  echo "    Role sudah ada."
fi

for db in "${DB_DEV}" "${DB_TEST}"; do
  echo "==> Buat database '${db}' jika belum ada..."
  if ! psql -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='${db}'" | grep -q 1; then
    createdb -O "${DB_USER}" "${db}"
    echo "    Database dibuat (owner: ${DB_USER})."
  else
    echo "    Database sudah ada."
  fi
done

echo "OK: role '${DB_USER}' + database '${DB_DEV}' & '${DB_TEST}' siap."
echo "DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:5432/${DB_DEV}"
