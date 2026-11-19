//! ConfigLoader — resolusi nilai config 5 lapis:
//!   1. config_snapshot (ref_type, ref_id)     → source SNAPSHOT
//!   2. config_value scope=RUANGAN             → source RUANGAN
//!   3. config_value scope=KATEGORI            → source KATEGORI
//!   4. config_value scope=GLOBAL              → source GLOBAL
//!   5. config_item.nilai_default              → source DEFAULT
//!
//! Cache moka (TTL 5 menit):
//!   - key item       : "item:{kode}"
//!   - key nilai scope: "cfg:{kode}:{scope_type}:{scope_id|*}"
//!
//! Invalidate per-key saat config_value INSERT/UPDATE; invalidate total saat startup.

use std::time::Duration;

use moka::future::Cache;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};

pub const CACHE_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConfigSource {
    Snapshot,
    Ruangan,
    Kategori,
    Global,
    Default,
}

/// Konteks resolusi: identitas transaksi + ruangan + kategori.
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    pub ref_type: Option<String>,
    pub ref_id: Option<i64>,
    pub ruangan_id: Option<i64>,
    pub kategori_id: Option<i64>,
}

impl ConfigContext {
    pub fn global() -> Self {
        Self::default()
    }
}

/// Hasil resolusi — dikembalikan ke API termasuk source untuk debug & UI.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedConfig {
    pub kode: String,
    pub label: String,
    pub tipe: String,
    pub nilai: Value,
    pub source: ConfigSource,
    pub scope_type: Option<String>,
    pub scope_id: Option<i64>,
    pub is_sensitive: bool,
    pub approver_roles: Vec<String>,
    #[serde(skip)]
    pub raw: String,
}

#[derive(Debug, Clone)]
struct CachedItem {
    id: i64,
    kode: String,
    label: String,
    tipe: String,
    nilai_default: String,
    is_sensitive: bool,
    approver_roles: Vec<String>,
}

#[derive(Debug, Clone)]
enum ScopeCache {
    Found(String),
    Missing,
}

#[derive(Clone)]
pub struct ConfigLoader {
    pool: PgPool,
    cache: Cache<String, CachedItem>,
    scope_cache: Cache<String, ScopeCache>,
}

fn scope_key(kode: &str, scope_type: &str, scope_id: Option<i64>) -> String {
    let id = scope_id
        .map(|i| i.to_string())
        .unwrap_or_else(|| "*".to_string());
    format!("cfg:{kode}:{scope_type}:{id}")
}

impl ConfigLoader {
    pub fn new(pool: PgPool) -> Self {
        let item_cache = Cache::builder()
            .time_to_live(Duration::from_secs(CACHE_TTL_SECONDS))
            .max_capacity(10_000)
            .build();
        let scope_cache = Cache::builder()
            .time_to_live(Duration::from_secs(CACHE_TTL_SECONDS))
            .max_capacity(10_000)
            .build();
        Self {
            pool,
            cache: item_cache,
            scope_cache,
        }
    }

    /// Invalidate TOTAL (dipanggil saat startup; cache baru selalu kosong,
    /// disediakan untuk dipanggil ulang bila diperlukan).
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
        self.scope_cache.invalidate_all();
    }

    /// Invalidate per-key saat config_value INSERT/UPDATE/DELETE.
    pub async fn invalidate_value(&self, kode: &str, scope_type: &str, scope_id: Option<i64>) {
        self.scope_cache
            .invalidate(&scope_key(kode, scope_type, scope_id))
            .await;
        tracing::debug!(kode, scope_type, "cache config di-invalidate");
    }

    async fn get_item(&self, kode: &str) -> AppResult<CachedItem> {
        if let Some(item) = self.cache.get(kode).await {
            return Ok(item);
        }
        let row = sqlx::query!(
            r#"SELECT id, kode, label, tipe, nilai_default,
                      is_sensitive, approver_roles
               FROM config_item WHERE kode = $1"#,
            kode
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Config '{kode}'")))?;

        let item = CachedItem {
            id: row.id,
            kode: row.kode,
            label: row.label,
            tipe: row.tipe,
            nilai_default: row.nilai_default,
            is_sensitive: row.is_sensitive,
            approver_roles: row
                .approver_roles
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        };
        self.cache.insert(kode.to_string(), item.clone()).await;
        Ok(item)
    }

    /// Nilai pada satu scope (cached, termasuk hasil "tidak ada").
    async fn scope_value(
        &self,
        item: &CachedItem,
        scope_type: &str,
        scope_id: Option<i64>,
    ) -> AppResult<Option<String>> {
        let key = scope_key(&item.kode, scope_type, scope_id);
        if let Some(v) = self.scope_cache.get(&key).await {
            return Ok(match v {
                ScopeCache::Found(s) => Some(s),
                ScopeCache::Missing => None,
            });
        }
        let value = sqlx::query_scalar!(
            "SELECT nilai FROM config_value
             WHERE config_item_id = $1 AND scope_type = $2 AND scope_id IS NOT DISTINCT FROM $3",
            item.id,
            scope_type,
            scope_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let cached = value.map(ScopeCache::Found).unwrap_or(ScopeCache::Missing);
        self.scope_cache.insert(key, cached.clone()).await;
        Ok(match cached {
            ScopeCache::Found(v) => Some(v),
            ScopeCache::Missing => None,
        })
    }

    /// Resolusi penuh 5 lapis → ResolvedConfig (nilai + source).
    pub async fn resolve(&self, kode: &str, ctx: &ConfigContext) -> AppResult<ResolvedConfig> {
        let item = self.get_item(kode).await?;
        let make = |raw: String,
                    source: ConfigSource,
                    scope_type: Option<String>,
                    scope_id: Option<i64>| {
            ResolvedConfig {
                kode: item.kode.clone(),
                label: item.label.clone(),
                tipe: item.tipe.clone(),
                nilai: crate::config::validate::parse_nilai(&item.tipe, &raw)
                    .unwrap_or(Value::String(raw.clone())),
                source,
                scope_type,
                scope_id,
                is_sensitive: item.is_sensitive,
                approver_roles: item.approver_roles.clone(),
                raw,
            }
        };

        // 1. Snapshot transaksi (tidak di-cache — immutable & konteks-spesifik)
        if let (Some(ref_type), Some(ref_id)) = (ctx.ref_type.as_deref(), ctx.ref_id) {
            let snap = sqlx::query_scalar!(
                "SELECT nilai FROM config_snapshot
                 WHERE ref_type = $1 AND ref_id = $2 AND config_item_id = $3",
                ref_type,
                ref_id,
                item.id
            )
            .fetch_optional(&self.pool)
            .await?;
            if let Some(v) = snap {
                return Ok(make(v, ConfigSource::Snapshot, None, None));
            }
        }

        // 2. RUANGAN
        if let Some(rid) = ctx.ruangan_id {
            if let Some(v) = self.scope_value(&item, "RUANGAN", Some(rid)).await? {
                return Ok(make(
                    v,
                    ConfigSource::Ruangan,
                    Some("RUANGAN".into()),
                    Some(rid),
                ));
            }
        }

        // 3. KATEGORI
        if let Some(kid) = ctx.kategori_id {
            if let Some(v) = self.scope_value(&item, "KATEGORI", Some(kid)).await? {
                return Ok(make(
                    v,
                    ConfigSource::Kategori,
                    Some("KATEGORI".into()),
                    Some(kid),
                ));
            }
        }

        // 4. GLOBAL
        if let Some(v) = self.scope_value(&item, "GLOBAL", None).await? {
            return Ok(make(v, ConfigSource::Global, Some("GLOBAL".into()), None));
        }

        // 5. DEFAULT
        Ok(make(
            item.nilai_default.clone(),
            ConfigSource::Default,
            None,
            None,
        ))
    }

    // --- Typed getters ---

    pub async fn get_string(&self, kode: &str, ctx: &ConfigContext) -> AppResult<String> {
        let r = self.resolve(kode, ctx).await?;
        Ok(r.raw)
    }

    pub async fn get_i64(&self, kode: &str, ctx: &ConfigContext) -> AppResult<i64> {
        let r = self.resolve(kode, ctx).await?;
        r.raw
            .parse::<i64>()
            .map_err(|_| AppError::Config(format!("config '{kode}' bukan number: '{}'", r.raw)))
    }

    pub async fn get_f64(&self, kode: &str, ctx: &ConfigContext) -> AppResult<f64> {
        let r = self.resolve(kode, ctx).await?;
        r.raw
            .parse::<f64>()
            .map_err(|_| AppError::Config(format!("config '{kode}' bukan decimal: '{}'", r.raw)))
    }

    pub async fn get_bool(&self, kode: &str, ctx: &ConfigContext) -> AppResult<bool> {
        let r = self.resolve(kode, ctx).await?;
        r.raw
            .parse::<bool>()
            .map_err(|_| AppError::Config(format!("config '{kode}' bukan boolean: '{}'", r.raw)))
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        kode: &str,
        ctx: &ConfigContext,
    ) -> AppResult<T> {
        let r = self.resolve(kode, ctx).await?;
        serde_json::from_str(&r.raw)
            .map_err(|_| AppError::Config(format!("config '{kode}' bukan json valid: '{}'", r.raw)))
    }

    // --- Snapshot ---

    /// Freeze nilai config ke config_snapshot saat transaksi dimulai.
    /// Snapshot TIDAK ditimpa bila sudah ada (histori tidak berubah).
    pub async fn snapshot(
        &self,
        ref_type: &str,
        ref_id: i64,
        kodes: &[&str],
        resolve_ctx: &ConfigContext,
    ) -> AppResult<Vec<String>> {
        let mut saved = Vec::new();
        for kode in kodes {
            let item = self.get_item(kode).await?;
            let resolved = self.resolve(kode, resolve_ctx).await?;
            sqlx::query!(
                "INSERT INTO config_snapshot (ref_type, ref_id, config_item_id, nilai)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (ref_type, ref_id, config_item_id) DO NOTHING",
                ref_type,
                ref_id,
                item.id,
                resolved.raw
            )
            .execute(&self.pool)
            .await?;
            saved.push(kode.to_string());
        }
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_scope_konsisten() {
        assert_eq!(scope_key("C01", "GLOBAL", None), "cfg:C01:GLOBAL:*");
        assert_eq!(scope_key("C01", "RUANGAN", Some(3)), "cfg:C01:RUANGAN:3");
    }
}
