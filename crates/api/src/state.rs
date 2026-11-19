//! AppState — pool DB, config app, config loader, rate limiter.

use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;
use sqlx::PgPool;

use sarpras_shared::app_config::AppConfig;
use sarpras_shared::config::ConfigLoader;
use sarpras_shared::error::{AppError, AppResult};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: Arc<AppConfig>,
    pub config: ConfigLoader,
    pub rate: Arc<RateLimiter>,
    pub push: crate::notifikasi::PushHub,
}

impl AppState {
    #[allow(dead_code)]
    pub fn new(pool: PgPool, cfg: AppConfig) -> Self {
        Self {
            config: ConfigLoader::new(pool.clone()),
            pool,
            cfg: Arc::new(cfg),
            rate: Arc::new(RateLimiter::new()),
            push: crate::notifikasi::PushHub::new(),
        }
    }
}

/// Rate limiter in-memory (moka): login 5x/15 menit (K05) & API 100 req/menit/user (§9).
pub struct RateLimiter {
    login: Cache<String, u32>,
    api: Cache<String, u32>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            login: Cache::builder()
                .time_to_live(Duration::from_secs(900))
                .build(),
            api: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .build(),
        }
    }

    /// Cek batas percobaan login. Key: "login:{email}:{ip}".
    pub fn check_login(&self, key: &str, max: u32) -> AppResult<()> {
        let n = self.login.get(key).unwrap_or(0);
        if n >= max {
            Err(AppError::RateLimited)
        } else {
            Ok(())
        }
    }

    /// Catat percobaan login gagal.
    pub fn record_login_failure(&self, key: &str) {
        let n = self.login.get(key).unwrap_or(0);
        self.login.insert(key.to_string(), n + 1);
    }

    /// Fixed-window 60 detik per user. Key: "api:{user_id}:{bucket}".
    pub fn check_api(&self, user_id: i64, max: u32) -> AppResult<()> {
        let bucket = chrono::Utc::now().timestamp() / 60;
        let key = format!("api:{user_id}:{bucket}");
        let n = self.api.get(&key).unwrap_or(0);
        if n >= max {
            Err(AppError::RateLimited)
        } else {
            self.api.insert(key, n + 1);
            Ok(())
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_dibatasi_setelah_batas() {
        let limiter = RateLimiter::new();
        let key = "login:a@b.c:1.2.3.4";
        for _ in 0..5 {
            assert!(limiter.check_login(key, 5).is_ok());
            limiter.record_login_failure(key);
        }
        assert!(limiter.check_login(key, 5).is_err());
    }

    #[test]
    fn api_limit_per_user_terpisah() {
        let limiter = RateLimiter::new();
        assert!(limiter.check_api(1, 2).is_ok());
        assert!(limiter.check_api(1, 2).is_ok());
        assert!(limiter.check_api(1, 2).is_err());
        // user lain tidak terpengaruh
        assert!(limiter.check_api(2, 2).is_ok());
    }
}
