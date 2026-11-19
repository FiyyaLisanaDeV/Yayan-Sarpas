//! Upload file — multipart → filesystem lokal ./data/uploads/{ref_type}/{ref_id}/.
//! Unduh via GET /uploads/{attachment_id} (auth).

use axum::extract::{Multipart, Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use sarpras_shared::api::ApiResponse;
use sarpras_shared::error::{AppError, AppResult};

use crate::middleware::auth::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::post(upload))
        .route("/qr/:file", get(download_qr))
        .route("/:id", get(download))
}

const EXT_ALLOWED: &[&str] = &[
    "pdf", "jpg", "jpeg", "png", "docx", "xlsx", "xls", "zip", "csv",
];

#[derive(serde::Serialize, sqlx::FromRow)]
struct AttachmentRow {
    id: i64,
    ref_type: String,
    ref_id: i64,
    path: String,
    jenis: Option<String>,
    uploaded_by: i64,
    uploaded_at: chrono::DateTime<chrono::Utc>,
}

pub async fn upload(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    mut multipart: Multipart,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let mut ref_type: Option<String> = None;
    let mut ref_id: Option<i64> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_data: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("multipart tidak valid".into()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "ref_type" => ref_type = field.text().await.ok().filter(|s| !s.is_empty()),
            "ref_id" => ref_id = field.text().await.ok().and_then(|s| s.parse().ok()),
            "file" => {
                file_name = field.file_name().map(String::from);
                content_type = field.content_type().map(String::from);
                file_data = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::Validation("gagal membaca file".into()))?
                    .to_vec();
            }
            _ => {}
        }
    }

    let ref_type =
        ref_type.ok_or_else(|| AppError::Validation("field ref_type wajib diisi".into()))?;
    let ref_id = ref_id.ok_or_else(|| AppError::Validation("field ref_id wajib diisi".into()))?;
    let file_name =
        file_name.ok_or_else(|| AppError::Validation("field file wajib diisi".into()))?;

    // Batas ukuran (MAX_UPLOAD_MB).
    let max_bytes = state.cfg.max_upload_mb.saturating_mul(1024 * 1024);
    if file_data.len() as u64 > max_bytes {
        return Err(AppError::Validation(format!(
            "ukuran file melebihi batas {} MB",
            state.cfg.max_upload_mb
        )));
    }
    if file_data.is_empty() {
        return Err(AppError::Validation("file kosong".into()));
    }

    // Validasi ekstensi.
    let ext = file_name
        .rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .ok_or_else(|| AppError::Validation("file tanpa ekstensi".into()))?;
    if !EXT_ALLOWED.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "ekstensi .{ext} tidak diizinkan (diizinkan: {})",
            EXT_ALLOWED.join(", ")
        )));
    }

    // Path relatif: {ref_type}/{ref_id}/{uuid}.{ext}
    let rel_dir = format!("{ref_type}/{ref_id}");
    let rel_path = format!("{rel_dir}/{}.{}", Uuid::new_v4(), ext);
    let abs_dir = std::path::Path::new(&state.cfg.upload_dir).join(&rel_dir);
    tokio::fs::create_dir_all(&abs_dir)
        .await
        .map_err(|e| AppError::Config(format!("gagal membuat direktori upload: {e}")))?;
    let abs_path = std::path::Path::new(&state.cfg.upload_dir).join(&rel_path);
    tokio::fs::write(&abs_path, &file_data)
        .await
        .map_err(|e| AppError::Config(format!("gagal menyimpan file: {e}")))?;

    let rec = sqlx::query_as!(
        AttachmentRow,
        r#"INSERT INTO attachments (ref_type, ref_id, path, jenis, uploaded_by)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, ref_type, ref_id, path, jenis, uploaded_by, uploaded_at"#,
        ref_type,
        ref_id,
        rel_path,
        content_type,
        actor.id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(ApiResponse::ok(serde_json::json!({
        "id": rec.id, "ref_type": rec.ref_type, "ref_id": rec.ref_id,
        "path": rec.path, "jenis": rec.jenis, "uploaded_at": rec.uploaded_at,
    })))
}

/// Unduh attachment by id (hanya path dari DB — bebas traversal).
pub async fn download(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    let row = sqlx::query_as!(
        AttachmentRow,
        "SELECT id, ref_type, ref_id, path, jenis, uploaded_by, uploaded_at FROM attachments WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Lampiran".into()))?;

    let abs_path = std::path::Path::new(&state.cfg.upload_dir).join(&row.path);
    let bytes = tokio::fs::read(&abs_path)
        .await
        .map_err(|_| AppError::NotFound("File lampiran tidak ditemukan di storage".into()))?;

    let content_type = row
        .jenis
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let file_name = row.path.rsplit('/').next().unwrap_or("file").to_string();
    let mut resp = ([(CONTENT_TYPE, content_type)], bytes).into_response();
    resp.headers_mut().insert(
        CONTENT_DISPOSITION,
        format!("inline; filename=\"{file_name}\"")
            .parse()
            .map_err(|_| AppError::Internal)?,
    );
    Ok(resp)
}

/// Unduh gambar QR aset (PNG) dari `upload_dir/qr/{file}` (auth).
/// Nama file divalidasi ketat untuk mencegah traversal.
pub async fn download_qr(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(file): Path<String>,
) -> AppResult<Response> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(AppError::Validation("nama file QR tidak valid".into()));
    }
    if !file.ends_with(".png") {
        return Err(AppError::Validation("format QR tidak valid".into()));
    }
    let abs_path = std::path::Path::new(&state.cfg.upload_dir)
        .join("qr")
        .join(&file);
    let bytes = tokio::fs::read(&abs_path)
        .await
        .map_err(|_| AppError::NotFound("File QR tidak ditemukan".into()))?;
    let mut resp = ([(CONTENT_TYPE, "image/png")], bytes).into_response();
    resp.headers_mut().insert(
        CONTENT_DISPOSITION,
        format!("inline; filename=\"{file}\"")
            .parse()
            .map_err(|_| AppError::Internal)?,
    );
    Ok(resp)
}

// Json import hanya untuk menghindari unused warning bila rute berubah.
#[allow(dead_code)]
fn _json_reexport(_: Json<serde_json::Value>) {}
