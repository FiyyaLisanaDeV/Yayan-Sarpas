//! Repository aset.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use sqlx::PgPool;

use sarpras_shared::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AsetRow {
    pub id: i64,
    pub item_id: i64,
    pub item_nama: String,
    pub kategori: String,
    pub ruangan_id: i64,
    pub ruangan_nama: String,
    pub kode_aset: String,
    pub no_seri: Option<String>,
    pub tgl_perolehan: NaiveDate,
    pub nilai: BigDecimal,
    pub umur_ekonomis: Option<i16>,
    pub kondisi: Option<String>,
    pub status: String,
    pub qr_path: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MutasiRow {
    pub id: i64,
    pub aset_id: i64,
    pub kode_aset: String,
    pub dari_ruangan: String,
    pub ke_ruangan: String,
    pub tanggal: NaiveDate,
    pub alasan: Option<String>,
    pub oleh_nama: String,
}

pub struct AsetRepository;

impl AsetRepository {
    const COLS: &'static str = r#"
        SELECT a.id, a.item_id, i.nama AS item_nama, k.kode AS kategori,
               a.ruangan_id, r.nama AS ruangan_nama,
               a.kode_aset, a.no_seri, a.tgl_perolehan, a.nilai,
               a.umur_ekonomis, a.kondisi, a.status, a.qr_path
        FROM aset_tetap a
        JOIN items i ON i.id = a.item_id
        JOIN kategori k ON k.id = i.kategori_id
        JOIN ruangan r ON r.id = a.ruangan_id
    "#;

    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
        search: Option<&str>,
        ruangan_id: Option<i64>,
        status: Option<&str>,
    ) -> AppResult<(Vec<AsetRow>, i64)> {
        let offset = (page - 1) * per_page;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = sqlx::query_as::<_, AsetRow>(&format!(
            r#"{}
               WHERE ($1::text IS NULL OR lower(a.kode_aset) LIKE $1 OR lower(i.nama) LIKE $1)
                 AND ($2::bigint IS NULL OR a.ruangan_id = $2)
                 AND ($3::text IS NULL OR a.status = $3)
               ORDER BY a.id DESC LIMIT $4 OFFSET $5"#,
            Self::COLS
        ))
        .bind(pattern.clone())
        .bind(ruangan_id)
        .bind(status)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM aset_tetap a JOIN items i ON i.id = a.item_id
               WHERE ($1::text IS NULL OR lower(a.kode_aset) LIKE $1 OR lower(i.nama) LIKE $1)
                 AND ($2::bigint IS NULL OR a.ruangan_id = $2)
                 AND ($3::text IS NULL OR a.status = $3)"#,
        )
        .bind(pattern)
        .bind(ruangan_id)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok((rows, total))
    }

    pub async fn find(pool: &PgPool, id: i64) -> AppResult<Option<AsetRow>> {
        Ok(
            sqlx::query_as::<_, AsetRow>(&format!("{} WHERE a.id = $1", Self::COLS))
                .bind(id)
                .fetch_optional(pool)
                .await?,
        )
    }

    pub async fn next_seq(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tahun: i16,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) + 1 FROM aset_tetap WHERE EXTRACT(YEAR FROM tgl_perolehan) = $1",
        )
        .bind(tahun)
        .fetch_one(&mut **tx)
        .await?)
    }

    pub async fn kode_aset_terpakai(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        kode: &str,
    ) -> AppResult<bool> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aset_tetap WHERE kode_aset = $1")
            .bind(kode)
            .fetch_one(&mut **tx)
            .await?;
        Ok(n > 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i64,
        ruangan_id: i64,
        kode_aset: &str,
        no_seri: Option<&str>,
        tgl_perolehan: NaiveDate,
        nilai: &BigDecimal,
        umur_ekonomis: Option<i16>,
        kondisi: Option<&str>,
        status: &str,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO aset_tetap
                 (item_id, ruangan_id, kode_aset, no_seri, tgl_perolehan, nilai,
                  umur_ekonomis, kondisi, status)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id"#,
            item_id,
            ruangan_id,
            kode_aset,
            no_seri,
            tgl_perolehan,
            nilai,
            umur_ekonomis,
            kondisi,
            status
        )
        .fetch_one(&mut **tx)
        .await?)
    }

    pub async fn set_status(pool: &PgPool, id: i64, status: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE aset_tetap SET status = $2 WHERE id = $1",
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_qr(pool: &PgPool, id: i64, path: &str) -> AppResult<()> {
        sqlx::query!("UPDATE aset_tetap SET qr_path = $2 WHERE id = $1", id, path)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn set_ruangan(pool: &PgPool, id: i64, ruangan_id: i64) -> AppResult<()> {
        sqlx::query!(
            "UPDATE aset_tetap SET ruangan_id = $2 WHERE id = $1",
            id,
            ruangan_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_mutasi(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        aset_id: i64,
        dari: i64,
        ke: i64,
        tanggal: NaiveDate,
        alasan: Option<&str>,
        oleh_id: i64,
    ) -> AppResult<i64> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO mutasi_aset (aset_id, dari_ruangan_id, ke_ruangan_id, tanggal, alasan, oleh_id)
               VALUES ($1,$2,$3,$4,$5,$6) RETURNING id"#,
            aset_id,
            dari,
            ke,
            tanggal,
            alasan,
            oleh_id
        )
        .fetch_one(&mut **tx)
        .await?)
    }

    pub async fn list_mutasi(pool: &PgPool, aset_id: i64) -> AppResult<Vec<MutasiRow>> {
        Ok(sqlx::query_as::<_, MutasiRow>(
            r#"SELECT m.id, m.aset_id, a.kode_aset,
                      rd.nama AS dari_ruangan, rk.nama AS ke_ruangan,
                      m.tanggal, m.alasan, u.nama AS oleh_nama
               FROM mutasi_aset m
               JOIN aset_tetap a ON a.id = m.aset_id
               JOIN ruangan rd ON rd.id = m.dari_ruangan_id
               JOIN ruangan rk ON rk.id = m.ke_ruangan_id
               JOIN users u ON u.id = m.oleh_id
               WHERE m.aset_id = $1 ORDER BY m.id DESC"#,
        )
        .bind(aset_id)
        .fetch_all(pool)
        .await?)
    }
}
