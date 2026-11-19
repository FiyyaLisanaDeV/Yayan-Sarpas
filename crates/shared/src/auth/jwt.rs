//! JWT — access token (pendek) + refresh token (rotasi, disimpan hash-nya di DB).

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::rbac::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub nama: String,
    pub ruangan_id: Option<i64>,
    pub token_type: TokenType,
    pub jti: String,
    pub exp: usize,
    pub iat: usize,
}

const LEEWAY_SECONDS: i64 = 30;

/// Buat access token (umur pendek, default 15 menit dari K01).
pub fn issue_access(
    user_id: i64,
    nama: &str,
    role: Role,
    ruangan_id: Option<i64>,
    secret: &str,
    ttl_minutes: i64,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.as_str().to_string(),
        nama: nama.to_string(),
        ruangan_id,
        token_type: TokenType::Access,
        jti: Uuid::new_v4().to_string(),
        exp: (now + Duration::minutes(ttl_minutes)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Config(format!("gagal membuat token: {e}")))
}

/// Buat refresh token (umur panjang, default 8 jam dari K02).
pub fn issue_refresh(
    user_id: i64,
    nama: &str,
    role: Role,
    secret: &str,
    ttl_hours: i64,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.as_str().to_string(),
        nama: nama.to_string(),
        ruangan_id: None,
        token_type: TokenType::Refresh,
        jti: Uuid::new_v4().to_string(),
        exp: (now + Duration::hours(ttl_hours)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Config(format!("gagal membuat refresh token: {e}")))
}

/// Decode + verifikasi token (tanda tangan, kedaluwarsa).
pub fn decode_token(token: &str, secret: &str) -> AppResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = LEEWAY_SECONDS as u64;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::InvalidToken)?;
    Ok(data.claims)
}

/// SHA-256 hex — untuk menyimpan refresh token di DB (bukan plaintext).
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-0123456789-0123456789-abcdef";

    #[test]
    fn access_token_roundtrip() {
        let token = issue_access(42, "Budi", Role::KasiSarpras, Some(7), SECRET, 15).unwrap();
        let claims = decode_token(&token, SECRET).unwrap();
        assert_eq!(claims.sub, "42");
        assert_eq!(claims.role, "KASI_SARPRAS");
        assert_eq!(claims.token_type, TokenType::Access);
        assert_eq!(claims.ruangan_id, Some(7));
    }

    #[test]
    fn refresh_token_type_dibedakan() {
        let token = issue_refresh(1, "Admin", Role::Admin, SECRET, 8).unwrap();
        let claims = decode_token(&token, SECRET).unwrap();
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn token_salah_ditolak() {
        assert!(decode_token("bukan-token", SECRET).is_err());
        let token = issue_access(1, "A", Role::Admin, None, SECRET, 15).unwrap();
        assert!(decode_token(&token, "secret-lain-0123456789-0123456789").is_err());
    }

    #[test]
    fn hash_konsisten_dan_berbeda() {
        let h1 = hash_token("token-a");
        let h2 = hash_token("token-a");
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_token("token-b"));
        assert_eq!(h1.len(), 64);
    }
}
