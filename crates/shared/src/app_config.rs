//! AppConfig — konfigurasi aplikasi dari .env + environment variable.
//! figment menyusun nilai dari default → .env (dotenvy) → OS environment.

use figment::providers::{Env, Serialized};
use figment::Figment;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct AppConfig {
    pub database_url: String,
    pub test_database_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
    pub upload_dir: String,
    pub max_upload_mb: u64,
    pub admin_initial_password: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<figment::Error>> {
        // Muat .env ke environment (tidak menimpa OS env yang sudah ada).
        let _ = dotenvy::dotenv();

        Figment::new()
            .merge(Serialized::defaults(Self::default()))
            .merge(Env::raw().only(&[
                "DATABASE_URL",
                "TEST_DATABASE_URL",
                "JWT_SECRET",
                "BIND_ADDR",
                "UPLOAD_DIR",
                "MAX_UPLOAD_MB",
                "ADMIN_INITIAL_PASSWORD",
            ]))
            .extract()
            .map_err(Box::new)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.jwt_secret.len() < 32 {
            return Err("JWT_SECRET terlalu pendek (min 32 karakter)".to_string());
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://sarpras:sarpras_dev_pw@localhost:5432/sarpras_dev"
                .to_string(),
            test_database_url: "postgres://sarpras:sarpras_dev_pw@localhost:5432/sarpras_test"
                .to_string(),
            jwt_secret: "dev-only-secret-ganti-sebelum-produksi-0123456789".to_string(),
            bind_addr: "127.0.0.1:8080".to_string(),
            upload_dir: "./data/uploads".to_string(),
            max_upload_mb: 10,
            admin_initial_password: "admin123".to_string(),
        }
    }
}
