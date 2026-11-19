//! Password — argon2 hash/verify + kebijakan password (K03/K04).

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use crate::error::{AppError, AppResult};

/// Hash password dengan argon2 (salt acak per hash).
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Config(format!("gagal hash password: {e}")))
}

/// Verifikasi password terhadap hash. False = tidak cocok.
pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Config(format!("hash password rusak: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Kebijakan password: panjang minimum (K03) + kompleksitas (K04).
/// Kompleksitas = wajib ada huruf besar, huruf kecil, dan angka.
pub fn validate_policy(password: &str, min_length: usize, require_complex: bool) -> AppResult<()> {
    if password.len() < min_length {
        return Err(AppError::WeakPassword);
    }
    if require_complex {
        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        if !(has_upper && has_lower && has_digit) {
            return Err(AppError::WeakPassword);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_dan_verify() {
        let hash = hash_password("Rahasia123").unwrap();
        assert!(verify_password("Rahasia123", &hash).unwrap());
        assert!(!verify_password("salah", &hash).unwrap());
    }

    #[test]
    fn hash_selalu_berbeda_karena_salt() {
        assert_ne!(
            hash_password("Rahasia123").unwrap(),
            hash_password("Rahasia123").unwrap()
        );
    }

    #[test]
    fn kebijakan_password() {
        assert!(validate_policy("Abcdef1", 8, true).is_err()); // pendek
        assert!(validate_policy("abcdefgh1", 8, true).is_err()); // tanpa kapital
        assert!(validate_policy("ABCDEFGH1", 8, true).is_err()); // tanpa kecil
        assert!(validate_policy("Abcdefgh", 8, true).is_err()); // tanpa angka
        assert!(validate_policy("Abcdefg1", 8, true).is_ok());
        assert!(validate_policy("abc", 3, false).is_ok()); // tanpa kompleksitas
    }
}
