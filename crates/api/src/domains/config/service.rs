//! Service config — update nilai (langsung / via change request), approval, audit.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use sarpras_shared::config::validate::validate_value;
use sarpras_shared::config::{ConfigContext, ConfigLoader};
use sarpras_shared::error::{AppError, AppResult};

use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConfigItemRow {
    pub id: i64,
    pub group_kode: String,
    pub group_nama: String,
    pub kode: String,
    pub label: String,
    pub tipe: String,
    pub nilai_default: String,
    pub scope: String,
    pub validation_json: Option<Value>,
    pub is_sensitive: bool,
    pub approver_roles: Value,
    pub urutan: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChangeRequestRow {
    pub id: i64,
    pub kode: String,
    pub label: String,
    pub tipe: String,
    pub nilai_baru: String,
    pub alasan: Option<String>,
    pub status: String,
    pub requested_by: i64,
    pub requested_nama: String,
    pub approved_by: Option<i64>,
    pub approved_nama: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 12 kode config kritis (butuh approval) — §7 SEED INSTRUKSI.
/// Harus sinkron dengan `config_items::KRITIS` di crate seed.
pub const KRITIS: &[&str] = &[
    "B01", "B02", "B03", "B04", "C01", "C02", "C07", "F01", "F02", "G03", "K06", "L01",
];

/// Apakah config ini kritis (butuh approval)?
/// Daftar eksplisit 12 kode — BUKAN sekadar "punya approver non-ADMIN",
/// karena banyak config non-kritis tetap mencantumkan role approver.
pub fn is_kritis(kode: &str) -> bool {
    KRITIS.contains(&kode)
}

pub struct ConfigService;

/// Parse nilai string config menjadi tipe JSON sesuai tipenya (mirror
/// `sarpras_shared::config::validate::parse_nilai`, dipakai untuk output API).
pub fn json_nilai(tipe: &str, raw: &str) -> Value {
    match tipe {
        "number" => raw
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "decimal" => raw
            .parse::<f64>()
            .map(|v| serde_json::json!(v))
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "boolean" => raw
            .parse::<bool>()
            .map(Value::Bool)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "json" => serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string())),
        _ => Value::String(raw.to_string()),
    }
}

impl ConfigService {
    pub async fn list_items(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        group: Option<&str>,
        q: Option<&str>,
    ) -> AppResult<(Vec<ConfigItemRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = q.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as!(
            ConfigItemRow,
            r#"SELECT ci.id, cg.kode AS group_kode, cg.nama AS group_nama,
                      ci.kode, ci.label, ci.tipe, ci.nilai_default, ci.scope,
                      ci.validation_json, ci.is_sensitive, ci.approver_roles, ci.urutan
               FROM config_item ci JOIN config_group cg ON cg.id = ci.group_id
               WHERE ($3::text IS NULL OR cg.kode = $3)
                 AND ($4::text IS NULL OR lower(ci.kode) LIKE $4 OR lower(ci.label) LIKE $4)
               ORDER BY cg.urutan, ci.kode
               LIMIT $1 OFFSET $2"#,
            per_page,
            offset,
            group,
            pattern
        )
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM config_item ci JOIN config_group cg ON cg.id = ci.group_id
               WHERE ($1::text IS NULL OR cg.kode = $1)
                 AND ($2::text IS NULL OR lower(ci.kode) LIKE $2 OR lower(ci.label) LIKE $2)"#,
            group,
            pattern
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok((rows, total))
    }

    /// Daftar config beserta nilai efektif GLOBAL dalam SATU query.
    /// Menghindari N+1 request dari UI Config Center (yang akan menabrak
    /// rate limit API 100 req/menit).
    pub async fn list_items_with_values(
        pool: &PgPool,
        group: Option<&str>,
        q: Option<&str>,
    ) -> AppResult<Vec<Value>> {
        let pattern = q.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query!(
            r#"SELECT ci.id, cg.kode AS group_kode, cg.nama AS group_nama,
                      ci.kode, ci.label, ci.tipe, ci.nilai_default, ci.scope,
                      ci.validation_json, ci.is_sensitive, ci.approver_roles, ci.urutan,
                      COALESCE(cv.nilai, ci.nilai_default) AS "nilai_efektif!",
                      CASE WHEN cv.id IS NULL THEN 'DEFAULT' ELSE 'GLOBAL' END AS "source_efektif!"
               FROM config_item ci
               JOIN config_group cg ON cg.id = ci.group_id
               LEFT JOIN config_value cv
                      ON cv.config_item_id = ci.id
                     AND cv.scope_type = 'GLOBAL'
                     AND cv.scope_id IS NULL
               WHERE ($1::text IS NULL OR cg.kode = $1)
                 AND ($2::text IS NULL OR lower(ci.kode) LIKE $2 OR lower(ci.label) LIKE $2)
               ORDER BY cg.urutan, ci.kode"#,
            group,
            pattern
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "group": r.group_kode,
                    "group_nama": r.group_nama,
                    "kode": r.kode,
                    "label": r.label,
                    "tipe": r.tipe,
                    "nilai_default": r.nilai_default,
                    "scope": r.scope,
                    "validation": r.validation_json,
                    "is_sensitive": r.is_sensitive,
                    "approver_roles": r.approver_roles,
                    "urutan": r.urutan,
                    "nilai": json_nilai(&r.tipe, &r.nilai_efektif),
                    "source": r.source_efektif,
                })
            })
            .collect())
    }

    pub async fn list_groups(pool: &PgPool) -> AppResult<Vec<Value>> {
        let rows = sqlx::query!(
            r#"SELECT cg.id, cg.kode, cg.nama, cg.deskripsi, cg.urutan, COUNT(ci.id) AS jumlah_item
               FROM config_group cg LEFT JOIN config_item ci ON ci.group_id = cg.id
               GROUP BY cg.id ORDER BY cg.urutan"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id, "kode": r.kode, "nama": r.nama, "deskripsi": r.deskripsi,
                    "urutan": r.urutan, "jumlah_item": r.jumlah_item
                })
            })
            .collect())
    }

    pub async fn list_audit(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        kode: Option<&str>,
    ) -> AppResult<(Vec<Value>, i64)> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query!(
            r#"SELECT ca.id, ci.kode, ci.label, ca.scope_type, ca.scope_id,
                      ca.nilai_lama, ca.nilai_baru, ca.changed_by, ca.changed_at, ca.alasan
               FROM config_audit ca JOIN config_item ci ON ci.id = ca.config_item_id
               WHERE ($3::text IS NULL OR ci.kode = $3)
               ORDER BY ca.changed_at DESC
               LIMIT $1 OFFSET $2"#,
            per_page,
            offset,
            kode
        )
        .fetch_all(pool)
        .await?;
        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM config_audit ca JOIN config_item ci ON ci.id = ca.config_item_id
               WHERE ($1::text IS NULL OR ci.kode = $1)"#,
            kode
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.id, "kode": r.kode, "label": r.label, "scope_type": r.scope_type, "scope_id": r.scope_id,
                "nilai_lama": r.nilai_lama, "nilai_baru": r.nilai_baru,
                "changed_by": r.changed_by, "changed_at": r.changed_at, "alasan": r.alasan,
            }))
            .collect();
        Ok((items, total))
    }

    /// Update nilai config GLOBAL.
    /// - Kritis (butuh approval) → buat config_change_request PENDING.
    /// - Non-kritis → langsung update + config_audit + invalidate cache.
    pub async fn update_value(
        state: &AppState,
        actor_id: i64,
        kode: &str,
        nilai_baru: &str,
        alasan: Option<&str>,
    ) -> AppResult<Value> {
        let item = Self::get_item(state, kode).await?;
        let (item_id, tipe, validation, _approver_roles) = item;

        validate_value(&tipe, &validation, nilai_baru)?;

        if is_kritis(kode) {
            let rec = sqlx::query!(
                "INSERT INTO config_change_request (config_item_id, scope_type, scope_id, nilai_baru, alasan, requested_by)
                 VALUES ($1, 'GLOBAL', NULL, $2, $3, $4) RETURNING id",
                item_id,
                nilai_baru,
                alasan,
                actor_id
            )
            .fetch_one(&state.pool)
            .await?;
            tracing::info!(
                kode,
                request_id = rec.id,
                "config kritis: change request PENDING dibuat"
            );
            return Ok(serde_json::json!({
                "pesan": format!("Config '{kode}' butuh approval. Change request #{0} menunggu persetujuan.", rec.id),
                "change_request_id": rec.id,
                "status": "PENDING"
            }));
        }

        Self::apply_value(state, kode, actor_id, item_id, nilai_baru, alasan).await?;
        Ok(
            serde_json::json!({"pesan": format!("Config '{kode}' berhasil diperbarui"), "status": "APPLIED"}),
        )
    }

    /// Terapkan nilai ke config_value + tulis config_audit + invalidate cache.
    async fn apply_value(
        state: &AppState,
        kode: &str,
        actor_id: i64,
        item_id: i64,
        nilai_baru: &str,
        alasan: Option<&str>,
    ) -> AppResult<()> {
        let nilai_lama: Option<String> = sqlx::query_scalar!(
            "SELECT nilai FROM config_value WHERE config_item_id = $1 AND scope_type = 'GLOBAL' AND scope_id IS NULL",
            item_id
        )
        .fetch_optional(&state.pool)
        .await?;

        match nilai_lama {
            Some(_) => {
                sqlx::query!(
                    "UPDATE config_value SET nilai = $2, updated_by = $3, alasan = $4
                     WHERE config_item_id = $1 AND scope_type = 'GLOBAL' AND scope_id IS NULL",
                    item_id,
                    nilai_baru,
                    actor_id,
                    alasan
                )
                .execute(&state.pool)
                .await?;
            }
            None => {
                sqlx::query!(
                    "INSERT INTO config_value (config_item_id, scope_type, scope_id, nilai, updated_by, alasan)
                     VALUES ($1, 'GLOBAL', NULL, $2, $3, $4)",
                    item_id,
                    nilai_baru,
                    actor_id,
                    alasan
                )
                .execute(&state.pool)
                .await?;
            }
        }

        sqlx::query!(
            "INSERT INTO config_audit (config_item_id, scope_type, scope_id, nilai_lama, nilai_baru, changed_by, alasan)
             VALUES ($1, 'GLOBAL', NULL, $2, $3, $4, $5)",
            item_id,
            nilai_lama,
            nilai_baru,
            actor_id.to_string(),
            alasan
        )
        .execute(&state.pool)
        .await?;

        // Invalidate cache moka untuk kode ini.
        state.config.invalidate_value(kode, "GLOBAL", None).await;
        Ok(())
    }

    pub async fn list_change_requests(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        status: Option<&str>,
    ) -> AppResult<(Vec<ChangeRequestRow>, i64)> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query_as!(
            ChangeRequestRow,
            r#"SELECT ccr.id AS "id!", ci.kode AS "kode!", ci.label AS "label!", ci.tipe AS "tipe!",
                      ccr.nilai_baru AS "nilai_baru!", ccr.alasan, ccr.status AS "status!",
                      ccr.requested_by AS "requested_by!", u.nama AS "requested_nama!",
                      ccr.approved_by AS "approved_by?", au.nama AS "approved_nama?",
                      ccr.approved_at AS "approved_at?", ccr.created_at AS "created_at!"
               FROM config_change_request ccr
               JOIN config_item ci ON ci.id = ccr.config_item_id
               JOIN users u ON u.id = ccr.requested_by
               LEFT JOIN users au ON au.id = ccr.approved_by
               WHERE ($3::text IS NULL OR ccr.status = $3)
               ORDER BY ccr.created_at DESC
               LIMIT $1 OFFSET $2"#,
            per_page,
            offset,
            status
        )
        .fetch_all(pool)
        .await?;
        let total = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM config_change_request WHERE ($1::text IS NULL OR status = $1)",
            status
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);
        Ok((rows, total))
    }

    /// Approve / reject change request.
    pub async fn decide_request(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        request_id: i64,
        approve: bool,
    ) -> AppResult<Value> {
        let row = sqlx::query!(
            r#"SELECT ccr.id, ccr.config_item_id, ccr.nilai_baru, ccr.alasan, ccr.status,
                      ci.kode, ci.approver_roles
               FROM config_change_request ccr JOIN config_item ci ON ci.id = ccr.config_item_id
               WHERE ccr.id = $1"#,
            request_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Change request".into()))?;

        if row.status != "PENDING" {
            return Err(AppError::Conflict(format!(
                "Change request sudah diproses (status {})",
                row.status
            )));
        }

        // Otorisasi: role actor harus ada di approver_roles.
        let approver_roles: Vec<String> = row
            .approver_roles
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        if !approver_roles.iter().any(|r| r == actor.role.as_str()) {
            return Err(AppError::Forbidden);
        }

        if approve {
            Self::apply_value(
                state,
                &row.kode,
                actor.id,
                row.config_item_id,
                &row.nilai_baru,
                row.alasan.as_deref(),
            )
            .await?;
            sqlx::query!(
                "UPDATE config_change_request SET status = 'APPROVED', approved_by = $2, approved_at = now() WHERE id = $1",
                request_id,
                actor.id
            )
            .execute(&state.pool)
            .await?;
            Ok(
                serde_json::json!({"pesan": format!("Config '{}' disetujui dan diterapkan", row.kode), "status": "APPROVED"}),
            )
        } else {
            sqlx::query!(
                "UPDATE config_change_request SET status = 'REJECTED', approved_by = $2, approved_at = now() WHERE id = $1",
                request_id,
                actor.id
            )
            .execute(&state.pool)
            .await?;
            Ok(
                serde_json::json!({"pesan": format!("Change request '{}' ditolak", row.kode), "status": "REJECTED"}),
            )
        }
    }

    /// Baca metadata config_item (tipe, validation, approver) — tanpa cache loader.
    async fn get_item(
        state: &AppState,
        kode: &str,
    ) -> AppResult<(i64, String, Option<Value>, Vec<String>)> {
        let row = sqlx::query!(
            "SELECT id, tipe, validation_json, approver_roles FROM config_item WHERE kode = $1",
            kode
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Config '{kode}'")))?;

        let approver_roles: Vec<String> = row
            .approver_roles
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok((row.id, row.tipe, row.validation_json, approver_roles))
    }
}

/// Helper: resolusi effective value dengan context opsional dari query.
pub async fn resolve_effective(
    loader: &ConfigLoader,
    kode: &str,
    ruangan_id: Option<i64>,
    kategori_id: Option<i64>,
) -> AppResult<Value> {
    let ctx = ConfigContext {
        ref_type: None,
        ref_id: None,
        ruangan_id,
        kategori_id,
    };
    let resolved = loader.resolve(kode, &ctx).await?;
    Ok(serde_json::json!({
        "kode": resolved.kode,
        "label": resolved.label,
        "tipe": resolved.tipe,
        "nilai": resolved.nilai,
        "source": resolved.source,
        "scope_type": resolved.scope_type,
        "scope_id": resolved.scope_id,
        "is_sensitive": resolved.is_sensitive,
        "approver_roles": resolved.approver_roles,
    }))
}
