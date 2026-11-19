//! Envelope respons API standar: { success, data, error, meta }

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize = ()> {
    pub success: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
            error: None,
            meta: None,
        }
    }

    pub fn ok_paged(data: T, page: i64, per_page: i64, total: i64) -> Self {
        Self {
            success: true,
            data,
            error: None,
            meta: Some(Meta {
                page,
                per_page,
                total,
            }),
        }
    }
}

impl<T: Serialize> axum::response::IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::OK, axum::Json(self)).into_response()
    }
}
