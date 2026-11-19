//! Integration test: skenario kritis end-to-end terhadap DB test.
//!
//! Prasyarat: `scripts/db-setup.sh` sudah dijalankan (DB `sarpras_test` ada).
//! Jalankan: `cargo test -p sarpras-api --test integration`
//!
//! Test ini memakai DB test terpisah dan membersihkan data sebelum mulai.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;

use sarpras_shared::app_config::AppConfig;
use sarpras_shared::config::ConfigLoader;

const TEST_DB: &str = "postgres://sarpras:sarpras_dev_pw@localhost:5432/sarpras_test";

mod support {
    use super::*;
    use std::sync::OnceLock;
    use tokio::sync::{Mutex, MutexGuard};

    /// Semua test berbagi satu DB test, jadi eksekusi WAJIB serial.
    fn lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Guard yang dipegang selama test berjalan (menjamin serialisasi).
    pub struct TestContext {
        pub pool: PgPool,
        _guard: MutexGuard<'static, ()>,
    }

    /// Siapkan DB test: serialisasi + migrasi + reset data transaksional + seed minimal.
    pub async fn setup() -> TestContext {
        let guard = lock().lock().await;

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(TEST_DB)
            .await
            .expect("koneksi DB test gagal — jalankan scripts/db-setup.sh");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrasi DB test gagal");

        // Bersihkan data uji (urutan penting karena FK).
        sqlx::query("DELETE FROM refresh_tokens")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM config_change_request")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM config_audit")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM config_snapshot")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM config_value")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM audit_trail")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM attachments")
            .execute(&pool)
            .await
            .ok();
        // Urutan penting: hapus tabel anak dulu agar FK tidak menghalangi.
        // Fase 8-10: aset, serah terima, penerimaan, PO, supplier, penawaran.
        for sql in [
            "DELETE FROM mutasi_aset",
            "DELETE FROM aset_tetap",
            "DELETE FROM serah_terima",
            "DELETE FROM penerimaan",
            "DELETE FROM po_detail",
            "DELETE FROM po_header",
            "DELETE FROM penawaran",
            "DELETE FROM supplier_item",
            "DELETE FROM supplier",
        ] {
            sqlx::query(sql).execute(&pool).await.ok();
        }
        // Fase 5: import batch & row log.
        sqlx::query("DELETE FROM import_row_log")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM import_batch")
            .execute(&pool)
            .await
            .ok();
        // Fase 2: transaksi permintaan & notifikasi.
        sqlx::query("DELETE FROM sla_tracking")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM notifikasi")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM approval_log")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM disposisi")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM permintaan")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM refresh_tokens")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM users").execute(&pool).await.ok();
        sqlx::query("DELETE FROM items").execute(&pool).await.ok();
        sqlx::query("DELETE FROM ruangan").execute(&pool).await.ok();

        seed_config_items(&pool).await;
        TestContext {
            pool,
            _guard: guard,
        }
    }

    /// Seed minimal config_item + config_value untuk test (idempoten).
    /// TIDAK early-return: config_value dihapus setiap test, jadi harus dibuat ulang.
    async fn seed_config_items(pool: &PgPool) {
        let group_id: i64 = sqlx::query_scalar("SELECT id FROM config_group WHERE kode = 'sla'")
            .fetch_one(pool)
            .await
            .expect("config_group 'sla' tidak ada — migrasi seed statis gagal");

        // (kode, grup, label, tipe, nilai_default, validation_json, approver)
        // C01 & G03 kritis; A01 non-kritis; K* dibutuhkan auth;
        // E*/C02/C03 untuk Fase 2; B* & C06/C07 untuk Fase 3.
        let defs: [(&str, &str, &str, &str, &str, Value, &str); 29] = [
            (
                "B01",
                "approval",
                "Urutan Step",
                "json",
                r#"["KASI","KABID","WADIR"]"#,
                json!({"schema":{}}),
                r#"["WADIR_YANMED","ADMIN"]"#,
            ),
            (
                "B02",
                "approval",
                "Role per Step",
                "json",
                r#"{"1":"KASI_SARPRAS","2":"KABID_YANMED","3":"WADIR_YANMED"}"#,
                json!({"schema":{}}),
                r#"["WADIR_YANMED","ADMIN"]"#,
            ),
            (
                "B03",
                "approval",
                "Boleh Skip Level",
                "boolean",
                "false",
                json!({}),
                r#"["WADIR_YANMED","ADMIN"]"#,
            ),
            (
                "B05",
                "approval",
                "Catatan Wajib Reject",
                "boolean",
                "true",
                json!({}),
                r#"["ADMIN"]"#,
            ),
            (
                "C06",
                "sla",
                "Pengingat H-",
                "number",
                "1",
                json!({"min":0,"max":999999}),
                r#"["ADMIN"]"#,
            ),
            (
                "C07",
                "sla",
                "Eskalasi Setelah",
                "number",
                "2",
                json!({"min":0,"max":999999}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "C01",
                "sla",
                "SLA Default (hari)",
                "number",
                "5",
                json!({"min":0,"max":999999}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "C02",
                "sla",
                "SLA per Langkah",
                "json",
                r#"{"SUBMITTED":5,"VERIFIKASI_KASI":5,"APPROVED_KABID":5,"APPROVED_WADIR":5,"PO_APPROVAL":5}"#,
                json!({"schema":{}}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "C03",
                "sla",
                "Basis Hari",
                "enum",
                "KERJA",
                json!({"options":["KERJA","KALENDER"]}),
                r#"["ADMIN"]"#,
            ),
            (
                "E01",
                "permintaan",
                "Periode Buka Bulan",
                "number",
                "1",
                json!({"min":0,"max":999999}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "E02",
                "permintaan",
                "Periode Buka Tanggal",
                "number",
                "1",
                json!({"min":0,"max":999999}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "E03",
                "permintaan",
                "Periode Tutup Bulan",
                "number",
                "12",
                json!({"min":0,"max":999999}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "E04",
                "permintaan",
                "Periode Tutup Tanggal",
                "number",
                "31",
                json!({"min":0,"max":999999}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "E06",
                "permintaan",
                "Maks Item",
                "number",
                "0",
                json!({"min":0,"max":999999}),
                r#"["ADMIN"]"#,
            ),
            (
                "E07",
                "permintaan",
                "Maks Qty",
                "number",
                "0",
                json!({"min":0,"max":999999}),
                r#"["ADMIN"]"#,
            ),
            (
                "E08",
                "permintaan",
                "Lampiran Wajib",
                "boolean",
                "false",
                json!({}),
                r#"["ADMIN"]"#,
            ),
            (
                "E09",
                "permintaan",
                "Urgent di Luar Periode",
                "boolean",
                "true",
                json!({}),
                r#"["KABID_YANMED","ADMIN"]"#,
            ),
            (
                "E11",
                "permintaan",
                "Batas Batal",
                "enum",
                "SEBELUM_APPROVAL_WADIR",
                json!({"options":["SEBELUM_VERIFIKASI","SEBELUM_APPROVAL_KABID","SEBELUM_APPROVAL_WADIR","TIDAK_BISA"]}),
                r#"["ADMIN"]"#,
            ),
            (
                "F11",
                "po",
                "Format Nomor PO",
                "text",
                "PO/{kategori}/{tahun}/{seq:04}",
                json!({"max_length":500}),
                r#"["ADMIN"]"#,
            ),
            (
                "G03",
                "serah",
                "Threshold Nilai Aset",
                "decimal",
                "1000000.00",
                json!({"min":0}),
                r#"["KABAG_KEUANGAN","ADMIN"]"#,
            ),
            (
                "J04",
                "cetak",
                "Format Nomor BA Serah",
                "text",
                "BAST/{tahun}/{seq:04}",
                json!({"max_length":500}),
                r#"["ADMIN"]"#,
            ),
            (
                "A01",
                "org",
                "Nama Rumah Sakit",
                "text",
                "RSUD Kota Kendari",
                json!({"max_length":500}),
                r#"["ADMIN"]"#,
            ),
            (
                "K01",
                "security",
                "JWT Access (menit)",
                "number",
                "15",
                json!({"min":0,"max":999999}),
                r#"["ADMIN"]"#,
            ),
            (
                "K02",
                "security",
                "Refresh Token (jam)",
                "number",
                "24",
                json!({"min":0,"max":999999}),
                r#"["ADMIN"]"#,
            ),
            (
                "K05",
                "security",
                "Maks Percobaan Login",
                "number",
                "5",
                json!({"min":0,"max":999999}),
                r#"["ADMIN"]"#,
            ),
            (
                "F01",
                "po",
                "PPN Persen",
                "decimal",
                "11",
                json!({"min":0,"max":100}),
                r#"["WADIR_YANMED","ADMIN"]"#,
            ),
            (
                "F02",
                "po",
                "Toleransi Penerimaan",
                "decimal",
                "0",
                json!({"min":0,"max":100}),
                r#"["ADMIN"]"#,
            ),
            (
                "F03",
                "po",
                "Threshold Nilai PO",
                "number",
                "0",
                json!({"min":0,"max":999999999}),
                r#"["WADIR_YANMED","ADMIN"]"#,
            ),
            (
                "F11",
                "po",
                "Format No PO",
                "text",
                "PO/{KATEGORI}/{TAHUN}/{NOMOR}",
                json!({"max_length":100}),
                r#"["ADMIN"]"#,
            ),
        ];

        for (kode, group, label, tipe, nilai, validation, approver) in defs {
            let gid: i64 = sqlx::query_scalar("SELECT id FROM config_group WHERE kode = $1")
                .bind(group)
                .fetch_one(pool)
                .await
                .unwrap_or(group_id);

            let item_id: i64 = sqlx::query_scalar(
                r#"INSERT INTO config_item (group_id, kode, label, tipe, nilai_default, scope, validation_json, approver_roles)
                   VALUES ($1, $2, $3, $4, $5, 'GLOBAL', $6, $7)
                   ON CONFLICT (kode) DO UPDATE SET
                       label = EXCLUDED.label,
                       tipe = EXCLUDED.tipe,
                       nilai_default = EXCLUDED.nilai_default,
                       validation_json = EXCLUDED.validation_json,
                       approver_roles = EXCLUDED.approver_roles
                   RETURNING id"#,
            )
            .bind(gid)
            .bind(kode)
            .bind(label)
            .bind(tipe)
            .bind(nilai)
            .bind(validation)
            .bind(serde_json::from_str::<Value>(approver).unwrap())
            .fetch_one(pool)
            .await
            .expect("insert config_item gagal");

            sqlx::query(
                r#"INSERT INTO config_value (config_item_id, scope_type, scope_id, nilai, alasan)
                   VALUES ($1, 'GLOBAL', NULL, $2, 'TEST_SEED')
                   ON CONFLICT DO NOTHING"#,
            )
            .bind(item_id)
            .bind(nilai)
            .execute(pool)
            .await
            .ok();
        }
    }

    /// Buat file Excel uji berisi sheet ALKES + SAPRAS (format rekapan RS).
    pub fn buat_excel_import(path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use rust_xlsxwriter::Workbook;

        let mut wb = Workbook::new();

        // Sheet ALKES: 2 baris (ruangan di baris pertama, lalu forward-fill).
        let s1 = wb.add_worksheet();
        s1.set_name("ALKES")?;
        s1.write_string(0, 0, "NO")?;
        s1.write_string(0, 1, "RUANGAN")?;
        s1.write_string(0, 2, "NAMA ALKES")?;
        s1.write_string(0, 3, "JUMLAH PERMINTAAN RUANGAN")?;
        s1.write_string(0, 4, "KETERANGAN")?;
        s1.write_string(1, 0, "1.")?;
        s1.write_string(1, 1, "Laboratorium Patologi Anatomi")?;
        s1.write_string(1, 2, "Grossing set")?;
        s1.write_number(1, 3, 1.0)?;
        // baris lanjutan tanpa ruangan → forward-fill
        s1.write_string(2, 2, "Bantex")?;
        s1.write_string(2, 3, "10 box")?;
        // legenda → stop
        s1.write_string(3, 1, "Kuning : belum ter input di link")?;

        // Sheet SAPRAS kosong (parsing tetap jalan tanpa error).
        let s2 = wb.add_worksheet();
        s2.set_name("SAPRAS")?;
        s2.write_string(0, 0, "NO")?;
        s2.write_string(0, 1, "RUANGAN")?;
        s2.write_string(0, 2, "NAMA BARANG")?;
        s2.write_string(0, 3, "JUMLAH PERMINTAAN RUANGAN")?;

        wb.save(path)?;
        Ok(())
    }

    /// Bangun router test dengan state terhubung DB test.
    pub fn build_app(pool: PgPool) -> axum::Router {
        let cfg = AppConfig {
            database_url: TEST_DB.to_string(),
            test_database_url: TEST_DB.to_string(),
            jwt_secret: "test-secret-0123456789-0123456789-abcdef".to_string(),
            bind_addr: "127.0.0.1:0".to_string(),
            upload_dir: "/tmp/sarpras-test-uploads".to_string(),
            max_upload_mb: 10,
            admin_initial_password: "Admin123".to_string(),
        };
        let state = sarpras_api::state::AppState {
            pool: pool.clone(),
            cfg: std::sync::Arc::new(cfg),
            config: ConfigLoader::new(pool),
            rate: std::sync::Arc::new(sarpras_api::state::RateLimiter::new()),
            push: sarpras_api::notifikasi::PushHub::new(),
        };
        sarpras_api::router::build_router(state)
    }

    /// Kirim request JSON, kembalikan (status, body JSON).
    pub async fn send(
        app: &axum::Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            builder = builder.header("Authorization", format!("Bearer {t}"));
        }
        let req = match body {
            Some(v) => builder
                .header("Content-Type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, json)
    }

    /// Buat admin pertama via /auth/register lalu login → access token.
    pub async fn bootstrap_admin(app: &axum::Router, email: &str) -> String {
        let (status, _) = send(
            app,
            "POST",
            "/auth/register",
            None,
            Some(json!({
                "nama": "Admin Uji", "nip": "9990001", "email": email, "password": "Admin123"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "registrasi bootstrap harus sukses");

        let (status, body) = send(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": email, "password": "Admin123"})),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "login harus sukses — respons: {body}"
        );
        body["data"]["access_token"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn alur_auth_lengkap() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());

    // 1. Registrasi bootstrap admin
    let token = support::bootstrap_admin(&app, "admin-test@x.local").await;

    // 2. Registrasi kedua harus ditolak (sudah ada user)
    let (status, _) = support::send(
        &app,
        "POST",
        "/auth/register",
        None,
        Some(json!({"nama":"Kedua","nip":"9990002","email":"b@x.local","password":"Admin123"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 3. /auth/me
    let (status, body) = support::send(&app, "GET", "/auth/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["role"], "ADMIN");

    // 4. Tanpa token → 401
    let (status, _) = support::send(&app, "GET", "/auth/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 5. Login password salah → 401
    let (status, _) = support::send(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email":"admin-test@x.local","password":"Salah123"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_dengan_username() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());

    // Bootstrap: username otomatis dari prefix email (admin-usr).
    support::bootstrap_admin(&app, "admin-usr@x.local").await;

    // 1. Login dengan username → sukses.
    let (status, body) = support::send(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"username":"admin-usr","password":"Admin123"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "login username harus sukses: {body}"
    );
    assert!(body["data"]["access_token"].as_str().is_some());

    // 2. Username kosong & tanpa email → 422.
    let (status, _) = support::send(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"password":"Admin123"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // 3. Username salah → 401.
    let (status, _) = support::send(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"username":"tidak-ada","password":"Admin123"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn config_kritis_butuh_approval_dan_tercatat_di_audit() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-cfg@x.local").await;

    // 1. Ubah config KRITIS G03 → PENDING (bukan langsung diterapkan)
    let (status, body) = support::send(
        &app,
        "PUT",
        "/config/values/G03",
        Some(&token),
        Some(json!({"nilai":"2500000.00","alasan":"Uji kritis"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "PENDING");
    let cr_id = body["data"]["change_request_id"].as_i64().unwrap();

    // 2. Nilai efektif BELUM berubah
    let (_, body) = support::send(&app, "GET", "/config/values/G03", Some(&token), None).await;
    assert_eq!(body["data"]["nilai"], 1000000.0);

    // 3. Change request muncul di daftar PENDING
    let (status, body) = support::send(
        &app,
        "GET",
        "/config/change-requests?status=PENDING",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"][0]["kode"], "G03");

    // 4. Approve → nilai diterapkan
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/config/change-requests/{cr_id}/approve"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 5. Nilai sudah berubah
    let (_, body) = support::send(&app, "GET", "/config/values/G03", Some(&token), None).await;
    assert_eq!(body["data"]["nilai"], 2500000.0);

    // 6. config_audit mencatat nilai lama & baru
    let (status, body) =
        support::send(&app, "GET", "/config/audit?kode=G03", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    let audit = &body["data"][0];
    assert_eq!(audit["nilai_lama"], "1000000.00");
    assert_eq!(audit["nilai_baru"], "2500000.00");
}

#[tokio::test]
async fn config_non_kritis_langsung_diterapkan() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-nk@x.local").await;

    let (status, body) = support::send(
        &app,
        "PUT",
        "/config/values/A01",
        Some(&token),
        Some(json!({"nilai":"RSUD Baru","alasan":"Uji non-kritis"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "APPLIED");

    let (_, body) = support::send(&app, "GET", "/config/values/A01", Some(&token), None).await;
    assert_eq!(body["data"]["nilai"], "RSUD Baru");
}

#[tokio::test]
async fn validasi_config_menolak_nilai_tidak_valid() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-val@x.local").await;

    // C01 adalah number: "abc" harus ditolak
    let (status, body) = support::send(
        &app,
        "PUT",
        "/config/values/C01",
        Some(&token),
        Some(json!({"nilai":"abc"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "VAL_001");

    // Melampaui batas max (number max 999999)
    let (status, _) = support::send(
        &app,
        "PUT",
        "/config/values/C01",
        Some(&token),
        Some(json!({"nilai":"5000000"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn crud_master_ruangan_dengan_audit() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-rg@x.local").await;

    // CREATE
    let (status, body) = support::send(
        &app,
        "POST",
        "/ruangan",
        Some(&token),
        Some(json!({"kode":"POLI-A","nama":"Poli A"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = body["data"]["id"].as_i64().unwrap();

    // DUPLIKAT → conflict
    let (status, _) = support::send(
        &app,
        "POST",
        "/ruangan",
        Some(&token),
        Some(json!({"kode":"POLI-A","nama":"Duplikat"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // UPDATE
    let (status, body) = support::send(
        &app,
        "PUT",
        &format!("/ruangan/{id}"),
        Some(&token),
        Some(json!({"kode":"POLI-A","nama":"Poli A Lantai 2"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["nama"], "Poli A Lantai 2");

    // LIST
    let (status, body) = support::send(&app, "GET", "/ruangan", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["meta"]["total"], 1);

    // SOFT DELETE
    let (status, _) = support::send(
        &app,
        "DELETE",
        &format!("/ruangan/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Setelah soft delete, list default kosong
    let (_, body) = support::send(&app, "GET", "/ruangan", Some(&token), None).await;
    assert_eq!(body["meta"]["total"], 0);

    // Audit trail mencatat CREATE/UPDATE/DELETE
    let (status, body) = support::send(
        &app,
        "GET",
        "/audit-trail?tabel=ruangan",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let aksi: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["aksi"].as_str().unwrap())
        .collect();
    assert!(aksi.contains(&"CREATE"));
    assert!(aksi.contains(&"UPDATE"));
    assert!(aksi.contains(&"DELETE"));
}

#[tokio::test]
async fn validasi_input_menolak_field_kosong() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-inv@x.local").await;

    let (status, body) = support::send(
        &app,
        "POST",
        "/ruangan",
        Some(&token),
        Some(json!({"kode":"","nama":""})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "VAL_001");
}

#[tokio::test]
async fn opsi_item_mengembalikan_seluruh_katalog() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-opt@x.local").await;
    let pool = ctx.pool.clone();

    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(&pool)
        .await
        .unwrap();

    // 120 item — melebihi batas per_page maksimum (100).
    for n in 0..120 {
        sqlx::query("INSERT INTO items (nama, satuan_id, kategori_id) VALUES ($1, $2, $3)")
            .bind(format!("Item Opsi {n:03}"))
            .bind(satuan_id)
            .bind(kategori_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    // Satu item nonaktif tidak boleh ikut.
    sqlx::query(
        "INSERT INTO items (nama, satuan_id, kategori_id, is_active) VALUES ('Item Nonaktif', $1, $2, false)",
    )
    .bind(satuan_id)
    .bind(kategori_id)
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = support::send(&app, "GET", "/items/options", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    let data = body["data"].as_array().unwrap();
    assert_eq!(
        data.len(),
        120,
        "seluruh item aktif harus terkirim, bukan 100"
    );
    assert!(data
        .iter()
        .all(|i| i["nama"].as_str().unwrap() != "Item Nonaktif"));
    // Field yang dibutuhkan form ikut tersedia.
    assert!(data[0]["satuan_id"].is_number());
    assert!(data[0]["kategori"].is_string());
}

#[tokio::test]
async fn health_dan_endpoint_publik() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());

    let (status, body) = support::send(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "ok");
}

/// Resolusi config 5 lapis: RUANGAN → KATEGORI → GLOBAL → DEFAULT,
/// plus snapshot yang mengalahkan semuanya.
///
/// Catatan: config_value disisipkan langsung ke DB (bukan lewat service),
/// sehingga cache TIDAK ter-invalidate. Karena itu setiap tahap memakai
/// ConfigLoader baru agar yang diuji benar-benar logika resolusi.
#[tokio::test]
async fn resolusi_config_berlapis_dan_snapshot() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let _token = support::bootstrap_admin(&app, "admin-res@x.local").await;

    let pool = ctx.pool.clone();

    let ruangan_id: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('RES-A','Ruangan Resolusi') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar("SELECT id FROM config_item WHERE kode = 'A01'")
        .fetch_one(&pool)
        .await
        .unwrap();

    use sarpras_shared::config::{ConfigContext, ConfigLoader, ConfigSource};

    let scope_ctx = ConfigContext {
        ref_type: None,
        ref_id: None,
        ruangan_id: Some(ruangan_id),
        kategori_id: Some(kategori_id),
    };

    // 1) Hanya GLOBAL → source GLOBAL
    let r = ConfigLoader::new(pool.clone())
        .resolve("A01", &scope_ctx)
        .await
        .unwrap();
    assert_eq!(r.source, ConfigSource::Global);
    assert_eq!(r.raw, "RSUD Kota Kendari");

    // 2) Tambah nilai KATEGORI → source KATEGORI
    sqlx::query(
        "INSERT INTO config_value (config_item_id, scope_type, scope_id, nilai) VALUES ($1,'KATEGORI',$2,'Nilai Kategori')",
    )
    .bind(item_id)
    .bind(kategori_id)
    .execute(&pool)
    .await
    .unwrap();

    let r = ConfigLoader::new(pool.clone())
        .resolve("A01", &scope_ctx)
        .await
        .unwrap();
    assert_eq!(r.source, ConfigSource::Kategori);
    assert_eq!(r.raw, "Nilai Kategori");

    // 3) Tambah nilai RUANGAN → source RUANGAN (lebih spesifik menang)
    sqlx::query(
        "INSERT INTO config_value (config_item_id, scope_type, scope_id, nilai) VALUES ($1,'RUANGAN',$2,'Nilai Ruangan')",
    )
    .bind(item_id)
    .bind(ruangan_id)
    .execute(&pool)
    .await
    .unwrap();

    let r = ConfigLoader::new(pool.clone())
        .resolve("A01", &scope_ctx)
        .await
        .unwrap();
    assert_eq!(r.source, ConfigSource::Ruangan);
    assert_eq!(r.raw, "Nilai Ruangan");

    // 4) Tanpa konteks ruangan/kategori → kembali ke GLOBAL
    let r = ConfigLoader::new(pool.clone())
        .resolve("A01", &ConfigContext::global())
        .await
        .unwrap();
    assert_eq!(r.source, ConfigSource::Global);
    assert_eq!(r.raw, "RSUD Kota Kendari");

    // 5) Snapshot transaksi mengalahkan semua scope
    sqlx::query(
        "INSERT INTO config_snapshot (ref_type, ref_id, config_item_id, nilai)
         VALUES ('permintaan', 1, $1, 'Nilai Snapshot')",
    )
    .bind(item_id)
    .execute(&pool)
    .await
    .unwrap();

    let snap_ctx = ConfigContext {
        ref_type: Some("permintaan".to_string()),
        ref_id: Some(1),
        ruangan_id: Some(ruangan_id),
        kategori_id: Some(kategori_id),
    };
    let r = ConfigLoader::new(pool.clone())
        .resolve("A01", &snap_ctx)
        .await
        .unwrap();
    assert_eq!(r.source, ConfigSource::Snapshot);
    assert_eq!(r.raw, "Nilai Snapshot");

    // 6) Kode tak dikenal → NotFound
    assert!(ConfigLoader::new(pool.clone())
        .resolve("ZZZ9", &ConfigContext::global())
        .await
        .is_err());
}

/// Cache moka harus di-invalidate saat config_value berubah,
/// sehingga nilai baru langsung terlihat tanpa menunggu TTL.
#[tokio::test]
async fn cache_config_invalidated_setelah_update() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-cache@x.local").await;

    // Baca awal → masuk cache
    let (_, body) = support::send(&app, "GET", "/config/values/A01", Some(&token), None).await;
    assert_eq!(body["data"]["nilai"], "RSUD Kota Kendari");

    // Ubah via API (A01 non-kritis → langsung diterapkan)
    let (status, _) = support::send(
        &app,
        "PUT",
        "/config/values/A01",
        Some(&token),
        Some(json!({"nilai":"RSUD Setelah Cache","alasan":"uji invalidasi"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Baca lagi → nilai baru (cache ter-invalidate)
    let (_, body) = support::send(&app, "GET", "/config/values/A01", Some(&token), None).await;
    assert_eq!(body["data"]["nilai"], "RSUD Setelah Cache");
}

/// Fase 2: draft multi-row → submit → status SUBMITTED + snapshot + notifikasi.
#[tokio::test]
async fn alur_permintaan_draft_dan_submit() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-prm@x.local").await;
    let pool = ctx.pool.clone();

    // Fixture: ruangan + item + satuan (satuan sudah dari seed statis).
    let ruangan_id: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('PRM-A','Ruangan Uji Permintaan') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ('Item Uji Permintaan', $1, $2) RETURNING id",
    )
    .bind(satuan_id)
    .bind(kategori_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // 1) Buat draft 2 baris
    let (status, body) = support::send(
        &app,
        "POST",
        "/permintaan",
        Some(&token),
        Some(json!({
            "ruangan_id": ruangan_id,
            "items": [
                {"item_id": item_id, "jumlah": 5, "satuan_id": satuan_id, "keterangan": "baris 1"},
                {"item_id": item_id, "jumlah": 2.5, "satuan_id": satuan_id, "keterangan": "baris 2"}
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create draft harus sukses: {body}");
    assert_eq!(body["data"]["status"], "DRAFT");
    assert_eq!(body["data"]["jumlah_baris"], 2);
    let batch_id = body["data"]["batch_id"].as_str().unwrap().to_string();

    // 2) Submit
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/permintaan/batch/{batch_id}/submit"),
        Some(&token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit harus sukses: {body}");
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["status"], "SUBMITTED");
    assert!(rows[0]["sla_deadline"].is_string());

    // 3) Snapshot config ter-freeze untuk setiap baris
    let snap: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM config_snapshot WHERE ref_type = 'permintaan'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(snap, 12, "2 baris × 6 kode config snapshot");

    // 4) Audit CREATE + SUBMIT tercatat
    let audits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_trail WHERE tabel = 'permintaan'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audits, 4, "2 CREATE + 2 SUBMIT");

    // 5) Submit ulang ditolak (bukan DRAFT lagi)
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/permintaan/batch/{batch_id}/submit"),
        Some(&token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn permintaan_validasi_jumlah_dan_item() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-prm2@x.local").await;
    let pool = ctx.pool.clone();

    let ruangan_id: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('PRM-B','Ruangan Uji 2') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Items kosong → 422
    let (status, body) = support::send(
        &app,
        "POST",
        "/permintaan",
        Some(&token),
        Some(json!({"ruangan_id": ruangan_id, "items": []})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "VAL_001");

    // Jumlah 0 → 422
    let (status, _) = support::send(
        &app,
        "POST",
        "/permintaan",
        Some(&token),
        Some(json!({
            "ruangan_id": ruangan_id,
            "items": [{"item_id": 1, "jumlah": 0, "satuan_id": satuan_id}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Item tidak dikenal → 422
    let (status, _) = support::send(
        &app,
        "POST",
        "/permintaan",
        Some(&token),
        Some(json!({
            "ruangan_id": ruangan_id,
            "items": [{"item_id": 999999, "jumlah": 1, "satuan_id": satuan_id}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn permintaan_batal_draft() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-prm3@x.local").await;
    let pool = ctx.pool.clone();

    let ruangan_id: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('PRM-C','Ruangan Uji 3') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'SAPRAS'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ('Item Uji Batal', $1, $2) RETURNING id",
    )
    .bind(satuan_id)
    .bind(kategori_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (_, body) = support::send(
        &app,
        "POST",
        "/permintaan",
        Some(&token),
        Some(json!({
            "ruangan_id": ruangan_id,
            "items": [{"item_id": item_id, "jumlah": 1, "satuan_id": satuan_id}]
        })),
    )
    .await;
    let batch_id = body["data"]["batch_id"].as_str().unwrap().to_string();

    let (status, body) = support::send(
        &app,
        "DELETE",
        &format!("/permintaan/batch/{batch_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["dihapus"], 1);

    let sisa: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permintaan")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sisa, 0);
}

#[tokio::test]
async fn notifikasi_terkirim_ke_kasi() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let token = support::bootstrap_admin(&app, "admin-ntf@x.local").await;
    let pool = ctx.pool.clone();

    // Buat user KASI agar menerima notifikasi SUBMIT.
    let hash = sarpras_shared::auth::password::hash_password("Kasi1234").unwrap();
    sqlx::query(
        "INSERT INTO users (nama, nip, email, username, password_hash, role) VALUES ('Kasi Uji','K001','kasi-uji@x.local','kasi-uji',$1,'KASI_SARPRAS')",
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();

    let ruangan_id: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('PRM-D','Ruangan Uji 4') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ('Item Uji Notif', $1, $2) RETURNING id",
    )
    .bind(satuan_id)
    .bind(kategori_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (_, body) = support::send(
        &app,
        "POST",
        "/permintaan",
        Some(&token),
        Some(json!({
            "ruangan_id": ruangan_id,
            "items": [{"item_id": item_id, "jumlah": 1, "satuan_id": satuan_id}]
        })),
    )
    .await;
    let batch_id = body["data"]["batch_id"].as_str().unwrap().to_string();

    support::send(
        &app,
        "POST",
        &format!("/permintaan/batch/{batch_id}/submit"),
        Some(&token),
        Some(json!({})),
    )
    .await;

    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifikasi n JOIN users u ON u.id = n.user_id
         WHERE u.role = 'KASI_SARPRAS' AND n.tipe = 'SUBMIT'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 1, "Kasi harus menerima 1 notifikasi SUBMIT");
}

// ================================================================ Fase 3

/// Buat user dengan role tertentu dan kembalikan access token-nya.
async fn buat_user_role(app: &axum::Router, pool: &PgPool, role: &str, email: &str) -> String {
    let hash = sarpras_shared::auth::password::hash_password("Pass1234").unwrap();
    sqlx::query(
        "INSERT INTO users (nama, nip, email, username, password_hash, role) VALUES ($1, $2, $3, $6, $4, $5)",
    )
    .bind(format!("User {role}"))
    .bind(format!("NIP-{email}"))
    .bind(email)
    .bind(&hash)
    .bind(role)
    .bind(email.split('@').next().unwrap_or("user"))
    .execute(pool)
    .await
    .unwrap();

    let (status, body) = support::send(
        app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": email, "password": "Pass1234"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login {role}: {body}");
    body["data"]["access_token"].as_str().unwrap().to_string()
}

/// Fixture: ruangan + item + permintaan 2 baris yang sudah SUBMITTED.
async fn siapkan_permintaan(app: &axum::Router, pool: &PgPool, token: &str, kode: &str) -> String {
    let ruangan_id: i64 =
        sqlx::query_scalar("INSERT INTO ruangan (kode, nama) VALUES ($1, $2) RETURNING id")
            .bind(kode)
            .bind(format!("Ruangan {kode}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(format!("Item {kode}"))
    .bind(satuan_id)
    .bind(kategori_id)
    .fetch_one(pool)
    .await
    .unwrap();

    let (_, body) = support::send(
        app,
        "POST",
        "/permintaan",
        Some(token),
        Some(json!({
            "ruangan_id": ruangan_id,
            "items": [
                {"item_id": item_id, "jumlah": 2, "satuan_id": satuan_id},
                {"item_id": item_id, "jumlah": 1, "satuan_id": satuan_id}
            ]
        })),
    )
    .await;
    let batch_id = body["data"]["batch_id"].as_str().unwrap().to_string();

    support::send(
        app,
        "POST",
        &format!("/permintaan/batch/{batch_id}/submit"),
        Some(token),
        Some(json!({})),
    )
    .await;

    batch_id
}

#[tokio::test]
async fn revisi_permintaan_setelah_ditolak() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-rev@x.local").await;
    let kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-rev@x.local").await;

    let batch_id = siapkan_permintaan(&app, &pool, &admin, "REV-A").await;

    // 1. Kasi menolak → RETURNED dengan catatan.
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/tolak"),
        Some(&kasi),
        Some(json!({"catatan": "spesifikasi belum lengkap"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = support::send(
        &app,
        "GET",
        &format!("/permintaan/batch/{batch_id}"),
        Some(&admin),
        None,
    )
    .await;
    let baris = body["data"].as_array().unwrap();
    assert_eq!(baris[0]["status"], "RETURNED");
    assert_eq!(baris[0]["revisi_ke"], 1);
    let baris_id = baris[0]["id"].as_i64().unwrap();
    let satuan_id = baris[0]["satuan"]["id"].as_i64().unwrap();

    // 2. RETURNED boleh diedit (jumlah diubah).
    let (status, body) = support::send(
        &app,
        "PUT",
        &format!("/permintaan/{baris_id}"),
        Some(&admin),
        Some(json!({"jumlah": "5", "satuan_id": satuan_id, "keterangan": "sudah dilengkapi"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "edit RETURNED harus bisa: {body}");

    // 3. Submit ulang → SUBMITTED, revisi_ke naik.
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/permintaan/batch/{batch_id}/submit"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = support::send(
        &app,
        "GET",
        &format!("/permintaan/batch/{batch_id}"),
        Some(&admin),
        None,
    )
    .await;
    let baris = body["data"].as_array().unwrap();
    assert_eq!(baris[0]["status"], "SUBMITTED");
    assert_eq!(baris[0]["revisi_ke"], 2);

    // 4. Riwayat memuat REJECT lalu SUBMIT/RESUBMIT.
    let (_, body) = support::send(
        &app,
        "GET",
        &format!("/approval/{batch_id}/riwayat"),
        Some(&admin),
        None,
    )
    .await;
    let aksi: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["aksi"].as_str().unwrap())
        .collect();
    assert!(aksi.contains(&"REJECT"), "riwayat harus memuat REJECT");
    assert!(
        aksi.contains(&"SUBMIT") || aksi.contains(&"RESUBMIT"),
        "riwayat harus memuat submit ulang"
    );

    // 5. Kasi kembali melihat antrean revisi.
    let (_, body) = support::send(&app, "GET", "/approval/inbox", Some(&kasi), None).await;
    assert_eq!(body["meta"]["total"], 2, "Kasi harus melihat revisi masuk");
}

#[tokio::test]
async fn disposisi_pimpinan_tercatat_dan_diteruskan() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-dsp@x.local").await;
    let wadir = buat_user_role(&app, &pool, "WADIR_YANMED", "wadir-dsp@x.local").await;
    let kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-dsp@x.local").await;
    let kasi_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE email = 'kasi-dsp@x.local'")
        .fetch_one(&pool)
        .await
        .unwrap();

    let batch_id = siapkan_permintaan(&app, &pool, &admin, "DSP-A").await;

    // 1. Isi kosong → 422.
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/disposisi"),
        Some(&wadir),
        Some(json!({"isi": "   "})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // 2. Wadir membuat disposisi + menunjuk pelaksana.
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/disposisi"),
        Some(&wadir),
        Some(json!({"isi": "Setujui dengan catatan: cek anggaran ke Kabag Keuangan", "untuk_user_id": kasi_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "buat disposisi: {body}");
    assert_eq!(body["data"]["dari"]["role"], "WADIR_YANMED");
    assert_eq!(body["data"]["untuk"]["id"], kasi_id);

    // 3. Disposisi tidak mengubah status pengajuan.
    let (_, body) = support::send(
        &app,
        "GET",
        &format!("/permintaan/batch/{batch_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(body["data"][0]["status"], "SUBMITTED");

    // 4. Daftar disposisi memuat 1 entri.
    let (status, body) = support::send(
        &app,
        "GET",
        &format!("/approval/{batch_id}/disposisi"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    // 5. Kasi (bukan pimpinan) tidak boleh membuat disposisi.
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/disposisi"),
        Some(&kasi),
        Some(json!({"isi": "coba"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 6. Notifikasi ke pemohon dan pelaksana.
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifikasi WHERE tipe = 'DISPOSISI'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 2, "pemohon dan pelaksana harus menerima notifikasi");
}

#[tokio::test]
async fn approval_berjenjang_tiga_step_sampai_final() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-apr@x.local").await;

    let kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-apr@x.local").await;
    let kabid = buat_user_role(&app, &pool, "KABID_YANMED", "kabid-apr@x.local").await;
    let wadir = buat_user_role(&app, &pool, "WADIR_YANMED", "wadir-apr@x.local").await;

    let batch_id = siapkan_permintaan(&app, &pool, &admin, "APR-A").await;

    // Rantai diambil dari config B01/B02.
    let (status, body) = support::send(&app, "GET", "/approval/rantai", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 3);

    // Kasi melihat 2 baris; Kabid belum punya antrean.
    let (_, body) = support::send(&app, "GET", "/approval/inbox", Some(&kasi), None).await;
    assert_eq!(body["meta"]["total"], 2, "Kasi harus melihat 2 baris");
    let (_, body) = support::send(&app, "GET", "/approval/inbox", Some(&kabid), None).await;
    assert_eq!(body["meta"]["total"], 0, "Kabid belum punya antrean");

    // Step 1 — KASI
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/setujui"),
        Some(&kasi),
        Some(json!({"catatan": "dokumen lengkap"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "kasi setujui: {body}");
    for r in body["data"].as_array().unwrap() {
        assert_eq!(r["status"], "VERIFIKASI_KASI");
    }

    // Step 2 — KABID
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/setujui"),
        Some(&kabid),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for r in body["data"].as_array().unwrap() {
        assert_eq!(r["status"], "APPROVED_KABID");
    }

    // Step 3 — WADIR → FINAL
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/setujui"),
        Some(&wadir),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for r in body["data"].as_array().unwrap() {
        assert_eq!(r["status"], "FINAL");
    }

    // Riwayat: 3 approve × 2 baris.
    let (status, body) = support::send(
        &app,
        "GET",
        &format!("/approval/{batch_id}/riwayat"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let logs = body["data"].as_array().unwrap();
    // 2 baris: 1 SUBMIT + 3 APPROVE per baris.
    assert_eq!(logs.len(), 8, "2 baris × (SUBMIT + 3 step)");
    let approve: Vec<_> = logs.iter().filter(|l| l["aksi"] == "APPROVE").collect();
    assert_eq!(approve.len(), 6, "6 entri APPROVE (2 baris × 3 step)");
    assert!(
        logs.iter().any(|l| l["aksi"] == "SUBMIT"),
        "riwayat memuat SUBMIT"
    );

    // Semua tracking SLA tertutup setelah FINAL.
    let terbuka: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sla_tracking WHERE closed_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(terbuka, 0, "SLA harus tertutup saat FINAL");
}

#[tokio::test]
async fn approval_reject_wajib_catatan_dan_kembali_returned() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-rej@x.local").await;
    let kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-rej@x.local").await;

    let batch_id = siapkan_permintaan(&app, &pool, &admin, "REJ-A").await;

    // Tanpa catatan → VAL_001 karena B05 = true.
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/tolak"),
        Some(&kasi),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "VAL_001");

    // Dengan catatan → RETURNED.
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/tolak"),
        Some(&kasi),
        Some(json!({"catatan": "spesifikasi kurang jelas"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for r in body["data"].as_array().unwrap() {
        assert_eq!(r["status"], "RETURNED");
    }

    // Catatan tersimpan pada riwayat.
    let (_, body) = support::send(
        &app,
        "GET",
        &format!("/approval/{batch_id}/riwayat"),
        Some(&admin),
        None,
    )
    .await;
    let logs = body["data"].as_array().unwrap();
    let tolak: Vec<_> = logs.iter().filter(|l| l["aksi"] == "REJECT").collect();
    assert_eq!(tolak.len(), 2, "2 baris ditolak");
    assert_eq!(tolak[0]["catatan"], "spesifikasi kurang jelas");
}

#[tokio::test]
async fn approval_menolak_role_yang_salah_step() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-rl@x.local").await;
    let _kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-rl@x.local").await;
    let kabid = buat_user_role(&app, &pool, "KABID_YANMED", "kabid-rl@x.local").await;

    let batch_id = siapkan_permintaan(&app, &pool, &admin, "RL-A").await;

    // KABID tidak boleh menyetujui step KASI.
    let (status, _) = support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/setujui"),
        Some(&kabid),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn job_sla_menandai_overdue_dan_eskalasi() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-sla@x.local").await;
    let kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-sla@x.local").await;
    let _kabid = buat_user_role(&app, &pool, "KABID_YANMED", "kabid-sla@x.local").await;

    let batch_id = siapkan_permintaan(&app, &pool, &admin, "SLA-A").await;

    // Kasi menyetujui → masuk step KABID → tracking SLA terbuka.
    support::send(
        &app,
        "POST",
        &format!("/approval/{batch_id}/setujui"),
        Some(&kasi),
        Some(json!({})),
    )
    .await;

    let terbuka: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sla_tracking WHERE closed_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(terbuka > 0, "harus ada tracking terbuka setelah approve");

    // Mundurkan deadline melewati ambang eskalasi C07 (2 hari).
    sqlx::query(
        "UPDATE sla_tracking SET deadline = now() - interval '5 days' WHERE closed_at IS NULL",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Jalankan job seperti scheduler harian.
    sarpras_worker::jobs::sla_checker(pool.clone()).await;

    // Satu run memproses ON_TIME → OVERDUE lalu langsung ESCALATED
    // (deadline 5 hari lalu melewati ambang C07 = 2 hari).
    let on_time_sisa: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sla_tracking WHERE closed_at IS NULL AND status = 'ON_TIME'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        on_time_sisa, 0,
        "tidak boleh ada tracking lewat deadline yang masih ON_TIME"
    );

    let escalated: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sla_tracking WHERE status = 'ESCALATED' AND escalated_at IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(escalated > 0, "tracking harus menjadi ESCALATED");

    let n_overdue: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifikasi WHERE tipe = 'SLA_OVERDUE'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(n_overdue > 0, "pemohon harus menerima SLA_OVERDUE");

    let n_eskalasi: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifikasi WHERE tipe = 'SLA_ESCALATED'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(n_eskalasi > 0, "atasan harus menerima SLA_ESCALATED");
}

// ================================================================ Fase 5

/// Import Excel rekapan → permintaan historis (CLOSED) + idempotensi.
#[tokio::test]
async fn import_excel_membuat_permintaan_historis_dan_idempoten() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-imp@x.local").await;

    // Fixture: ruangan + item yang akan dicocokkan parser.
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ruangan (kode, nama) VALUES ('IMP-A','Laboratorium Patologi Anatomi')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO items (nama, satuan_id, kategori_id) VALUES ('Grossing set', $1, $2)")
        .bind(satuan_id)
        .bind(kategori_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO items (nama, satuan_id, kategori_id) VALUES ('Bantex', $1, $2)")
        .bind(satuan_id)
        .bind(kategori_id)
        .execute(&pool)
        .await
        .unwrap();

    let path = "/tmp/sarpras-test-import.xlsx";
    support::buat_excel_import(path).expect("gagal membuat excel uji");

    // 1) Preview
    let (status, body) = support::send(
        &app,
        "GET",
        &format!("/import/preview?file_path={path}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview: {body}");
    assert_eq!(body["data"]["total_baris"], 2);
    assert_eq!(body["data"]["cocok"], 2, "kedua baris harus cocok master");

    // 2) Import commit
    let (status, body) = support::send(
        &app,
        "POST",
        "/import/excel",
        Some(&admin),
        Some(json!({"file_path": path, "tahun": 2026})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "import: {body}");
    assert_eq!(body["data"]["sukses"], 2);
    assert_eq!(body["data"]["gagal"], 0);
    assert_eq!(body["data"]["duplikat"], 0);

    // 3) Baris masuk sebagai CLOSED dengan client_id IMPORT.
    let closed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM permintaan WHERE status = 'CLOSED' AND client_id = 'IMPORT'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(closed, 2);

    // 4) Row log tercatat SUKSES.
    let logs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM import_row_log WHERE status = 'SUKSES'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(logs, 2);

    // 5) Import ulang → semua DUPLIKAT, tidak menambah data.
    let (status, body) = support::send(
        &app,
        "POST",
        "/import/excel",
        Some(&admin),
        Some(json!({"file_path": path, "tahun": 2026})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["sukses"], 0);
    assert_eq!(body["data"]["duplikat"], 2, "import ulang harus idempoten");

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permintaan")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(total, 2, "jumlah permintaan tidak bertambah");
}

#[tokio::test]
async fn import_menolak_file_tidak_ada() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let admin = support::bootstrap_admin(&app, "admin-imp2@x.local").await;

    let (status, body) = support::send(
        &app,
        "POST",
        "/import/excel",
        Some(&admin),
        Some(json!({"file_path": "/tmp/tidak-ada-file.xlsx"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NF_001");
}

#[tokio::test]
async fn import_hanya_untuk_admin() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-imp3@x.local").await;
    let kasi = buat_user_role(&app, &pool, "KASI_SARPRAS", "kasi-imp3@x.local").await;

    let (status, _) = support::send(&app, "GET", "/import/batch", Some(&kasi), None).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Kasi tidak boleh akses import"
    );

    let (status, _) = support::send(&app, "GET", "/import/batch", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
}

// ================================================================ Fase 8-10

/// Siapkan ruangan + item + permintaan FINAL untuk alur pengadaan.
async fn siapkan_pengadaan(pool: &PgPool, kode: &str) -> (i64, i64, i64) {
    let ruangan_id: i64 =
        sqlx::query_scalar("INSERT INTO ruangan (kode, nama) VALUES ($1,$2) RETURNING id")
            .bind(format!("PG-{kode}"))
            .bind(format!("Ruangan PG {kode}"))
            .fetch_one(pool)
            .await
            .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ($1,$2,$3) RETURNING id",
    )
    .bind(format!("Item PG {kode}"))
    .bind(satuan_id)
    .bind(kategori_id)
    .fetch_one(pool)
    .await
    .unwrap();

    let permintaan_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO permintaan (tahun, ruangan_id, item_id, jumlah, satuan_id, status, current_step, created_by)
           VALUES (2026,$1,$2,10,$3,'FINAL',4,(SELECT id FROM users ORDER BY id LIMIT 1)) RETURNING id"#,
    )
    .bind(ruangan_id)
    .bind(item_id)
    .bind(satuan_id)
    .fetch_one(pool)
    .await
    .unwrap();

    (ruangan_id, item_id, permintaan_id)
}

#[tokio::test]
async fn alur_supplier_penawaran_po_sampai_terkirim() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-po@x.local").await;

    let mut suppliers = Vec::new();
    for nama in ["PT Alfa", "PT Beta", "PT Gamma"] {
        let (status, body) = support::send(
            &app,
            "POST",
            "/supplier",
            Some(&admin),
            Some(json!({"nama": nama})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "buat supplier: {body}");
        suppliers.push(body["data"]["id"].as_i64().unwrap());
    }

    let (_ruangan, item_id, permintaan_id) = siapkan_pengadaan(&pool, "A").await;
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();

    for sid in &suppliers {
        let (status, _) = support::send(
            &app,
            "POST",
            &format!("/supplier/{sid}/items"),
            Some(&admin),
            Some(json!({"item_id": item_id, "harga_referensi": 1200000})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let harga = [1_200_000, 1_150_000, 1_300_000];
    let mut penawaran_ids = Vec::new();
    for (i, sid) in suppliers.iter().enumerate() {
        let (status, body) = support::send(
            &app,
            "POST",
            "/penawaran",
            Some(&admin),
            Some(json!({
                "supplier_id": sid, "item_id": item_id,
                "harga_satuan": harga[i], "sumber": "surat"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "penawaran: {body}");
        penawaran_ids.push(body["data"]["id"].as_i64().unwrap());
    }

    let (status, body) =
        support::send(&app, "GET", "/penawaran?per_page=10", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"][0]["jumlah_penawaran"], 3);
    assert_eq!(body["data"][0]["ada_terpilih"], false);

    // PO sebelum penunjukan harus ditolak.
    let (status, body) = support::send(
        &app,
        "POST",
        "/po",
        Some(&admin),
        Some(json!({
            "supplier_id": suppliers[1], "tahun": 2026,
            "details": [{"permintaan_id": permintaan_id, "item_id": item_id,
                         "qty": 10, "satuan_id": satuan_id}]
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "harus tolak tanpa penunjukan: {body}"
    );

    // Tunjuk supplier termurah.
    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/penawaran/{}/pilih", penawaran_ids[1]),
        Some(&admin),
        Some(json!({"alasan": "harga terendah"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "pilih: {body}");
    assert_eq!(body["data"]["status"], "DIPILIH");

    // Buat PO (harga otomatis dari penawaran terpilih).
    let (status, body) = support::send(
        &app,
        "POST",
        "/po",
        Some(&admin),
        Some(json!({
            "supplier_id": suppliers[1], "tahun": 2026,
            "details": [{"permintaan_id": permintaan_id, "item_id": item_id,
                         "qty": 10, "satuan_id": satuan_id}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "buat PO: {body}");
    let po_id = body["data"]["id"].as_i64().unwrap();
    let nilai: f64 = body["data"]["nilai_total"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((nilai - 12_765_000.0).abs() < 1.0, "nilai_total: {nilai}");

    let st: String = sqlx::query_scalar("SELECT status FROM permintaan WHERE id = $1")
        .bind(permintaan_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st, "IN_PO");

    // Detail PO memuat header + baris (regresi: alias qty_diterima pada query dinamis).
    let (status, body) =
        support::send(&app, "GET", &format!("/po/{po_id}"), Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK, "detail PO: {body}");
    assert_eq!(body["data"]["details"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"]["details"][0]["qty_diterima"], "0");
    assert!(body["data"]["header"]["no_po"].as_str().is_some());

    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/po/{po_id}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "APPROVED");

    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/po/{po_id}/kirim"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "SENT");

    let snap: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM config_snapshot WHERE ref_type='po_header' AND ref_id=$1",
    )
    .bind(po_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(snap, 3, "snapshot PO: F01,F03,F11");
}

#[tokio::test]
async fn penerimaan_parsial_closed_dan_serah_terima() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-rcv@x.local").await;

    let (ruangan_id, item_id, permintaan_id) = siapkan_pengadaan(&pool, "R").await;
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let supplier_id: i64 =
        sqlx::query_scalar("INSERT INTO supplier (nama) VALUES ('PT Uji Terima') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let po_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO po_header (no_po, tahun, supplier_id, tanggal_po, nilai_total, ppn, status, created_by)
           VALUES ('PO/UJI/2026/0001', 2026, $1, CURRENT_DATE, 5000000, 550000, 'SENT', (SELECT id FROM users ORDER BY id LIMIT 1)) RETURNING id"#,
    )
    .bind(supplier_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let detail_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO po_detail (po_header_id, permintaan_id, item_id, qty, harga_satuan, subtotal, satuan_id)
           VALUES ($1,$2,$3,10,500000,5000000,$4) RETURNING id"#,
    )
    .bind(po_id)
    .bind(permintaan_id)
    .bind(item_id)
    .bind(satuan_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Tanpa DO → 422 (dokumen wajib F07).
    let (status, body) = support::send(
        &app,
        "POST",
        "/penerimaan",
        Some(&admin),
        Some(json!({"po_detail_id": detail_id, "qty_terima": 4,
                    "no_invoice": "INV-1", "no_faktur_pajak": "FP-1"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "tanpa DO: {body}");

    // Parsial 4 → PARTIAL_RECEIVED
    let (status, body) = support::send(
        &app,
        "POST",
        "/penerimaan",
        Some(&admin),
        Some(json!({"po_detail_id": detail_id, "qty_terima": 4,
                    "no_do": "DO-1", "no_invoice": "INV-1", "no_faktur_pajak": "FP-1"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "parsial: {body}");
    let st: String = sqlx::query_scalar("SELECT status FROM po_header WHERE id = $1")
        .bind(po_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st, "PARTIAL_RECEIVED");

    // Sisa 6 → CLOSED
    let (status, _) = support::send(
        &app,
        "POST",
        "/penerimaan",
        Some(&admin),
        Some(json!({"po_detail_id": detail_id, "qty_terima": 6,
                    "no_do": "DO-2", "no_invoice": "INV-2", "no_faktur_pajak": "FP-2"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let st: String = sqlx::query_scalar("SELECT status FROM po_header WHERE id = $1")
        .bind(po_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st, "CLOSED");

    // Melebihi sisa → 422
    let (status, _) = support::send(
        &app,
        "POST",
        "/penerimaan",
        Some(&admin),
        Some(json!({"po_detail_id": detail_id, "qty_terima": 1,
                    "no_do": "X", "no_invoice": "X", "no_faktur_pajak": "X"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Serah terima → CONFIRMED → aset otomatis (nilai 5jt ≥ G03 1jt)
    let penerimaan_id: i64 = sqlx::query_scalar("SELECT id FROM penerimaan ORDER BY id LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let penerima_id: i64 = sqlx::query_scalar("SELECT id FROM users ORDER BY id LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let (status, body) = support::send(
        &app,
        "POST",
        "/serah-terima",
        Some(&admin),
        Some(json!({"penerimaan_id": penerimaan_id,
                    "ruangan_id": ruangan_id,
                    "penerima_ruangan_id": penerima_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "BAST: {body}");
    assert_eq!(body["data"]["status_konfirmasi"], "PENDING");
    let serah_id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/serah-terima/{serah_id}/konfirmasi"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "konfirmasi: {body}");
    assert_eq!(body["data"]["status_konfirmasi"], "CONFIRMED");

    let aset: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aset_tetap")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(aset, 1, "aset otomatis dari serah terima");
}

#[tokio::test]
async fn aset_registrasi_mutasi_dan_dashboard() {
    let ctx = support::setup().await;
    let app = support::build_app(ctx.pool.clone());
    let pool = ctx.pool.clone();
    let admin = support::bootstrap_admin(&app, "admin-ast@x.local").await;

    let ruangan_a: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('AS-A','Ruang A') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let ruangan_b: i64 = sqlx::query_scalar(
        "INSERT INTO ruangan (kode, nama) VALUES ('AS-B','Ruang B') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let satuan_id: i64 = sqlx::query_scalar("SELECT id FROM satuan WHERE kode = 'UNIT'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let kategori_id: i64 = sqlx::query_scalar("SELECT id FROM kategori WHERE kode = 'ALKES'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let item_id: i64 = sqlx::query_scalar(
        "INSERT INTO items (nama, satuan_id, kategori_id) VALUES ('Monitor Uji',$1,$2) RETURNING id",
    )
    .bind(satuan_id)
    .bind(kategori_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (status, body) = support::send(
        &app,
        "POST",
        "/aset/registrasi",
        Some(&admin),
        Some(json!({"item_id": item_id, "ruangan_id": ruangan_a,
                    "nilai": 7500000, "umur_ekonomis": 5, "no_seri": "SN-001"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "registrasi: {body}");
    let aset_id = body["data"]["id"].as_i64().unwrap();
    assert_eq!(body["data"]["status"], "REGISTERED");
    let kode = body["data"]["kode_aset"].as_str().unwrap().to_string();
    assert!(kode.contains("ALKES/2026"), "kode aset: {kode}");

    let (status, body) = support::send(
        &app,
        "GET",
        &format!("/aset/{aset_id}/qr"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["payload"].as_str().unwrap().contains(&kode));

    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/aset/{aset_id}/mutasi"),
        Some(&admin),
        Some(json!({"ke_ruangan_id": ruangan_b, "alasan": "relokasi"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "mutasi: {body}");

    let sekarang: i64 = sqlx::query_scalar("SELECT ruangan_id FROM aset_tetap WHERE id=$1")
        .bind(aset_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sekarang, ruangan_b);

    let (status, body) = support::send(
        &app,
        "POST",
        &format!("/aset/{aset_id}/status"),
        Some(&admin),
        Some(json!({"status": "IN_USE"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "IN_USE");

    let (status, body) = support::send(
        &app,
        "GET",
        "/dashboard/rekap?tahun=2026",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "rekap: {body}");

    let (status, body) = support::send(
        &app,
        "GET",
        "/dashboard/realisasi?tahun=2026",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "realisasi: {body}");
    assert_eq!(body["data"]["jumlah_aset"], 1);
}
