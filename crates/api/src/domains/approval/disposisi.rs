//! Disposisi pimpinan — catatan arahan (dan opsional penunjukan pelaksana)
//! pada satu pengajuan. Tidak mengubah langkah approval.

use sqlx::PgPool;
use uuid::Uuid;

use sarpras_shared::error::{AppError, AppResult};

use crate::domains::approval::repository::ApprovalRepository;
use crate::middleware::record_audit;
use crate::state::AppState;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct DisposisiRow {
    pub id: i64,
    pub batch_id: Uuid,
    pub dari_user_id: i64,
    pub dari_nama: String,
    pub dari_role: String,
    pub untuk_user_id: Option<i64>,
    pub untuk_nama: Option<String>,
    pub status_asal: String,
    pub isi: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct DisposisiService;

impl DisposisiService {
    /// Buat disposisi pada sebuah pengajuan.
    /// RBAC diperiksa di handler (WADIR_YANMED, ADMIN).
    #[tracing::instrument(skip(state, actor, isi), fields(batch_id = %batch_id))]
    pub async fn buat(
        state: &AppState,
        actor: &sarpras_shared::auth::AuthUser,
        batch_id: Uuid,
        isi: &str,
        untuk_user_id: Option<i64>,
    ) -> AppResult<DisposisiRow> {
        let isi = isi.trim();
        if isi.is_empty() {
            return Err(AppError::Validation("Isi disposisi wajib diisi".into()));
        }

        let ids = ApprovalRepository::batch_ids(&state.pool, batch_id).await?;
        if ids.is_empty() {
            return Err(AppError::NotFound("Permintaan".into()));
        }

        // Pelaksana (bila ditunjuk) harus user aktif.
        if let Some(uid) = untuk_user_id {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND is_active)",
            )
            .bind(uid)
            .fetch_one(&state.pool)
            .await?;
            if !ok {
                return Err(AppError::Validation(
                    "Pelaksana yang ditunjuk tidak ditemukan atau nonaktif".into(),
                ));
            }
        }

        // Status asal disimpan sebagai snapshot (tidak mengubah alur).
        let status_asal: String = sqlx::query_scalar("SELECT status FROM permintaan WHERE id = $1")
            .bind(ids[0])
            .fetch_one(&state.pool)
            .await?;

        let id: i64 = sqlx::query_scalar!(
            r#"INSERT INTO disposisi (batch_id, dari_user_id, untuk_user_id, status_asal, isi)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id"#,
            batch_id,
            actor.id,
            untuk_user_id,
            &status_asal,
            isi
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, batch_id = %batch_id, "gagal INSERT disposisi");
            e
        })?;

        // Audit pada setiap baris pengajuan.
        for id in &ids {
            record_audit(
                &state.pool,
                actor.id,
                "permintaan",
                *id,
                "DISPOSISI",
                Some(&serde_json::json!({"status": status_asal})),
                Some(&serde_json::json!({"isi": isi, "untuk_user_id": untuk_user_id})),
            )
            .await?;
        }

        // Notifikasi: pemohon + pelaksana (bila ditunjuk).
        let pemohon: i64 = sqlx::query_scalar("SELECT created_by FROM permintaan WHERE id = $1")
            .bind(ids[0])
            .fetch_one(&state.pool)
            .await?;
        crate::notifikasi::kirim_ke_user(
            state,
            pemohon,
            "DISPOSISI",
            Some(ids[0]),
            &format!("Disposisi pimpinan pada pengajuan Anda: {isi}"),
        )
        .await?;
        if let Some(uid) = untuk_user_id {
            if uid != pemohon {
                crate::notifikasi::kirim_ke_user(
                    state,
                    uid,
                    "DISPOSISI",
                    Some(ids[0]),
                    &format!("Anda ditunjuk menindaklanjuti pengajuan: {isi}"),
                )
                .await?;
            }
        }

        Self::ambil_satu(&state.pool, id).await
    }

    /// Daftar disposisi pada satu pengajuan (terbaru dulu).
    #[tracing::instrument(skip(state), fields(batch_id = %batch_id))]
    pub async fn list(
        state: &AppState,
        batch_id: Uuid,
        hanya_milik: Option<i64>,
    ) -> AppResult<Vec<DisposisiRow>> {
        // Validasi batch ada; USER_RUANGAN hanya melihat pengajuannya sendiri.
        let ids = ApprovalRepository::batch_ids(&state.pool, batch_id).await?;
        if ids.is_empty() {
            return Err(AppError::NotFound("Permintaan".into()));
        }
        if let Some(uid) = hanya_milik {
            let pemohon: i64 =
                sqlx::query_scalar("SELECT created_by FROM permintaan WHERE id = $1")
                    .bind(ids[0])
                    .fetch_one(&state.pool)
                    .await?;
            if pemohon != uid {
                return Err(AppError::Forbidden);
            }
        }

        let rows = sqlx::query_as::<_, DisposisiRow>(
            r#"SELECT d.id, d.batch_id, d.dari_user_id, u.nama AS dari_nama, u.role AS dari_role,
                      d.untuk_user_id, pu.nama AS untuk_nama,
                      d.status_asal, d.isi, d.created_at
               FROM disposisi d
               JOIN users u ON u.id = d.dari_user_id
               LEFT JOIN users pu ON pu.id = d.untuk_user_id
               WHERE d.batch_id = $1
               ORDER BY d.created_at DESC, d.id DESC"#,
        )
        .bind(batch_id)
        .fetch_all(&state.pool)
        .await?;

        Ok(rows)
    }

    async fn ambil_satu(pool: &PgPool, id: i64) -> AppResult<DisposisiRow> {
        Ok(sqlx::query_as::<_, DisposisiRow>(
            r#"SELECT d.id, d.batch_id, d.dari_user_id, u.nama AS dari_nama, u.role AS dari_role,
                      d.untuk_user_id, pu.nama AS untuk_nama,
                      d.status_asal, d.isi, d.created_at
               FROM disposisi d
               JOIN users u ON u.id = d.dari_user_id
               LEFT JOIN users pu ON pu.id = d.untuk_user_id
               WHERE d.id = $1"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?)
    }
}
