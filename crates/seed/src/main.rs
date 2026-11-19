//! sarpras-seed — migrasi + seeding data awal.
//! Dijalankan sekali (idempoten) setelah database dibuat via scripts/db-setup.sh:
//!   cargo run -p sarpras-seed

mod config_items;
mod excel_import;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{info, warn};

const DEFAULT_DATABASE_URL: &str = "postgres://sarpras:sarpras_dev_pw@localhost:5432/sarpras_dev";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sarpras_seed=info,sqlx=warn".into()),
        )
        .init();

    let _ = dotenvy::dotenv();
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let admin_password =
        std::env::var("ADMIN_INITIAL_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("Terhubung ke database.");

    info!("[1/4] Menjalankan migrasi...");
    sqlx::migrate!("../../migrations").run(&pool).await?;

    info!(
        "[2/4] Seeding {} config item + nilai GLOBAL + audit kritis...",
        config_items::CONFIG_ITEMS.len()
    );
    let (items, values, audits) = seed_config(&pool).await?;
    info!("config: {items} item, {values} nilai GLOBAL, {audits} audit INITIAL_SEED");

    info!("[3/4] Import ruangan & item dari Excel...");
    let report = excel_import::import_ruangan_items(&pool).await?;
    info!("sumber: {}", report.sumber);
    info!(
        "ruangan baru: {}, item baru: {}, duplikat dilewati: {}, baris dilewati: {}, satuan default: {}, placeholder dibersihkan: {}",
        report.ruangan_baru,
        report.item_baru,
        report.duplikat,
        report.dilewati,
        report.satuan_default,
        report.placeholder_dihapus
    );

    info!("[4/4] Membuat admin awal...");
    seed_admin(&pool, &admin_password).await?;

    let total_ruangan: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ruangan")
        .fetch_one(&pool)
        .await?;
    let total_item: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM items")
        .fetch_one(&pool)
        .await?;
    let total_libur: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hari_libur")
        .fetch_one(&pool)
        .await?;
    let total_config: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM config_item")
        .fetch_one(&pool)
        .await?;
    let total_placeholder: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM items WHERE nama LIKE '%(PLACEHOLDER)%')
              + (SELECT COUNT(*) FROM ruangan WHERE nama LIKE '%(PLACEHOLDER)%')",
    )
    .fetch_one(&pool)
    .await?;

    info!(
        "Seed selesai. Ringkasan: {total_ruangan} ruangan, {total_item} item, {total_libur} hari libur 2026, {total_config} config."
    );
    if total_placeholder > 0 {
        warn!(
            "Masih ada {total_placeholder} baris PLACEHOLDER — data asli belum masuk sepenuhnya."
        );
    }
    if admin_password == "admin123" {
        warn!("ADMIN_INITIAL_PASSWORD masih default — WAJIB ganti setelah login pertama!");
    }
    Ok(())
}

async fn seed_config(pool: &PgPool) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> {
    let mut group_ids: HashMap<String, i64> = HashMap::new();
    for row in sqlx::query("SELECT id, kode FROM config_group")
        .fetch_all(pool)
        .await?
    {
        let id: i64 = row.try_get("id")?;
        let kode: String = row.try_get("kode")?;
        group_ids.insert(kode, id);
    }

    let mut n_items = 0u64;
    let mut n_values = 0u64;
    let mut n_audits = 0u64;

    for def in config_items::CONFIG_ITEMS {
        let group_id = group_ids.get(def.group).copied().ok_or_else(|| {
            format!(
                "config_group '{}' tidak ditemukan untuk kode {}",
                def.group, def.kode
            )
        })?;

        let approver_roles = config_items::approver_roles(def.approver);
        let validation = config_items::validation_for(def.kode, def.tipe);

        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM config_item WHERE kode = $1")
                .bind(def.kode)
                .fetch_optional(pool)
                .await?;

        let item_id: i64 = match existing {
            Some(id) => id,
            None => {
                let rec = sqlx::query(
                    "INSERT INTO config_item (group_id, kode, label, tipe, nilai_default, scope, validation_json, approver_roles, urutan)
                     VALUES ($1, $2, $3, $4, $5, 'GLOBAL', $6, $7, $8)
                     RETURNING id",
                )
                .bind(group_id)
                .bind(def.kode)
                .bind(def.label)
                .bind(def.tipe)
                .bind(def.nilai_default)
                .bind(validation)
                .bind(serde_json::to_value(&approver_roles)?)
                .bind(1i32)
                .fetch_one(pool)
                .await?;
                let id: i64 = rec.try_get("id")?;
                n_items += 1;
                id
            }
        };

        // Seed config_value GLOBAL = nilai_default (jika belum ada)
        let value_exists: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM config_value WHERE config_item_id = $1 AND scope_type = 'GLOBAL' AND scope_id IS NULL",
        )
        .bind(item_id)
        .fetch_optional(pool)
        .await?;

        if value_exists.is_none() {
            sqlx::query(
                "INSERT INTO config_value (config_item_id, scope_type, scope_id, nilai, updated_by, alasan)
                 VALUES ($1, 'GLOBAL', NULL, $2, NULL, 'INITIAL_SEED')",
            )
            .bind(item_id)
            .bind(def.nilai_default)
            .execute(pool)
            .await?;
            n_values += 1;
        }

        // Config kritis: config_audit "INITIAL_SEED" oleh "SYSTEM" (jika belum ada)
        if config_items::KRITIS.contains(&def.kode) {
            let audit_exists: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM config_audit WHERE config_item_id = $1 AND changed_by = 'SYSTEM' AND alasan = 'INITIAL_SEED'",
            )
            .bind(item_id)
            .fetch_optional(pool)
            .await?;

            if audit_exists.is_none() {
                sqlx::query(
                    "INSERT INTO config_audit (config_item_id, scope_type, scope_id, nilai_lama, nilai_baru, changed_by, alasan)
                     VALUES ($1, 'GLOBAL', NULL, NULL, $2, 'SYSTEM', 'INITIAL_SEED')",
                )
                .bind(item_id)
                .bind(def.nilai_default)
                .execute(pool)
                .await?;
                n_audits += 1;
            }
        }
    }

    Ok((n_items, n_values, n_audits))
}

async fn seed_admin(pool: &PgPool, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("admin@rsud-kendari.local")
        .fetch_optional(pool)
        .await?;

    if exists.is_some() {
        info!("Admin awal sudah ada — dilewati.");
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("gagal hash password admin: {e}"))?
        .to_string();

    sqlx::query(
        "INSERT INTO users (nama, nip, email, username, password_hash, role)
         VALUES ('Administrator', '0000000001', 'admin@rsud-kendari.local', 'admin', $1, 'ADMIN')",
    )
    .bind(hash)
    .execute(pool)
    .await?;

    info!("Admin awal dibuat: username 'admin' (role ADMIN)");
    Ok(())
}
