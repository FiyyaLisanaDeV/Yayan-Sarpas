//! Repository user.

use serde::Serialize;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub nama: String,
    pub nip: String,
    pub email: String,
    pub username: String,
    pub role: String,
    pub ruangan_id: Option<i64>,
    pub ruangan_kode: Option<String>,
    pub ruangan_nama: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct UserRepository;

impl UserRepository {
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        search: Option<&str>,
        role: Option<&str>,
        include_inactive: bool,
    ) -> AppResult<(Vec<UserRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as!(
            UserRow,
            r#"SELECT u.id AS "id!", u.nama AS "nama!", u.nip AS "nip!", u.email AS "email!",
                      u.username AS "username!", u.role AS "role!", u.ruangan_id,
                      r.kode AS "ruangan_kode?", r.nama AS "ruangan_nama?",
                      u.is_active AS "is_active!", u.created_at AS "created_at!"
               FROM users u LEFT JOIN ruangan r ON r.id = u.ruangan_id
               WHERE ($3::text IS NULL OR lower(u.nama) LIKE $3 OR lower(u.email) LIKE $3 OR lower(u.nip) LIKE $3 OR lower(u.username) LIKE $3)
                 AND ($4::text IS NULL OR u.role = $4)
                 AND ($5::bool OR u.is_active)
               ORDER BY u.nama
               LIMIT $1 OFFSET $2"#,
            per_page,
            offset,
            pattern,
            role,
            include_inactive
        )
        .fetch_all(pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM users u
               WHERE ($1::text IS NULL OR lower(u.nama) LIKE $1 OR lower(u.email) LIKE $1 OR lower(u.nip) LIKE $1 OR lower(u.username) LIKE $1)
                 AND ($2::text IS NULL OR u.role = $2)
                 AND ($3::bool OR u.is_active)"#,
            pattern,
            role,
            include_inactive
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<UserRow>> {
        Ok(sqlx::query_as!(
            UserRow,
            r#"SELECT u.id AS "id!", u.nama AS "nama!", u.nip AS "nip!", u.email AS "email!",
                      u.username AS "username!", u.role AS "role!", u.ruangan_id,
                      r.kode AS "ruangan_kode?", r.nama AS "ruangan_nama?",
                      u.is_active AS "is_active!", u.created_at AS "created_at!"
               FROM users u LEFT JOIN ruangan r ON r.id = u.ruangan_id
               WHERE u.id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await?)
    }

    // Argumen terstruktur untuk INSERT; nama field eksplisit lebih penting
    // daripada menambahkan struct satu-pakai.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &PgPool,
        nama: &str,
        nip: &str,
        email: &str,
        username: &str,
        password_hash: &str,
        role: &str,
        ruangan_id: Option<i64>,
    ) -> AppResult<i64> {
        let rec = sqlx::query!(
            "INSERT INTO users (nama, nip, email, username, password_hash, role, ruangan_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
            nama,
            nip,
            email,
            username,
            password_hash,
            role,
            ruangan_id
        )
        .fetch_one(pool)
        .await?;
        Ok(rec.id)
    }

    pub async fn update(
        pool: &PgPool,
        id: i64,
        nama: &str,
        role: &str,
        ruangan_id: Option<i64>,
        is_active: bool,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE users SET nama = $2, role = $3, ruangan_id = $4, is_active = $5 WHERE id = $1",
            id,
            nama,
            role,
            ruangan_id,
            is_active
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_password(pool: &PgPool, id: i64, password_hash: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE users SET password_hash = $2 WHERE id = $1",
            id,
            password_hash
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn count_active_admin(pool: &PgPool) -> AppResult<i64> {
        Ok(
            sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE role = 'ADMIN' AND is_active")
                .fetch_one(pool)
                .await?
                .unwrap_or(0),
        )
    }

    pub async fn count_users(pool: &PgPool) -> AppResult<i64> {
        Ok(sqlx::query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await?
            .unwrap_or(0))
    }

    pub async fn find_login(pool: &PgPool, identifier: &str) -> AppResult<Option<LoginRow>> {
        Ok(sqlx::query_as!(
            LoginRow,
            "SELECT id, nama, nip, email, username, password_hash, role, ruangan_id, is_active
             FROM users
             WHERE lower(username) = lower($1) OR lower(email) = lower($1)",
            identifier
        )
        .fetch_optional(pool)
        .await?)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct LoginRow {
    pub id: i64,
    pub nama: String,
    pub nip: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub ruangan_id: Option<i64>,
    pub is_active: bool,
}
