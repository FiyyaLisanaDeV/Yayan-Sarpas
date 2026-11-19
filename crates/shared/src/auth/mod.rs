//! Auth — JWT access/refresh, hashing password argon2, kebijakan password.

pub mod jwt;
pub mod password;

use serde::Serialize;

use crate::rbac::Role;

/// User yang sudah terautentikasi (hasil extractor middleware).
#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: i64,
    pub nama: String,
    pub nip: String,
    pub email: String,
    pub role: Role,
    pub ruangan_id: Option<i64>,
}
