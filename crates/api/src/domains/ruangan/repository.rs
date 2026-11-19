//! Repository ruangan — akses DB (sqlx compile-time checked).

use serde::Serialize;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RuanganRow {
    pub id: i64,
    pub kode: String,
    pub nama: String,
    pub is_active: bool,
}

pub struct RuanganRepository;

impl RuanganRepository {
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        search: Option<&str>,
        include_inactive: bool,
    ) -> AppResult<(Vec<RuanganRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as!(
            RuanganRow,
            r#"SELECT id, kode, nama, is_active FROM ruangan
               WHERE ($3::text IS NULL OR lower(kode) LIKE $3 OR lower(nama) LIKE $3)
                 AND ($4::bool OR is_active)
               ORDER BY kode
               LIMIT $1 OFFSET $2"#,
            per_page,
            offset,
            pattern,
            include_inactive
        )
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM ruangan
               WHERE ($1::text IS NULL OR lower(kode) LIKE $1 OR lower(nama) LIKE $1)
                 AND ($2::bool OR is_active)"#,
            pattern,
            include_inactive
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<RuanganRow>> {
        Ok(sqlx::query_as!(
            RuanganRow,
            "SELECT id, kode, nama, is_active FROM ruangan WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?)
    }

    pub async fn find_by_kode(pool: &PgPool, kode: &str) -> AppResult<Option<RuanganRow>> {
        Ok(sqlx::query_as!(
            RuanganRow,
            "SELECT id, kode, nama, is_active FROM ruangan WHERE kode = $1",
            kode
        )
        .fetch_optional(pool)
        .await?)
    }

    pub async fn create(pool: &PgPool, kode: &str, nama: &str) -> AppResult<i64> {
        let rec = sqlx::query!(
            "INSERT INTO ruangan (kode, nama) VALUES ($1, $2) RETURNING id",
            kode,
            nama
        )
        .fetch_one(pool)
        .await?;
        Ok(rec.id)
    }

    pub async fn update(pool: &PgPool, id: i64, kode: &str, nama: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE ruangan SET kode = $2, nama = $3 WHERE id = $1",
            id,
            kode,
            nama
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(pool: &PgPool, id: i64) -> AppResult<()> {
        sqlx::query!("UPDATE ruangan SET is_active = FALSE WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
