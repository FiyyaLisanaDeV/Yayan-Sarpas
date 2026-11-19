//! Role & permission — 9 role sesuai §5.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    UserRuangan,
    KasiSarpras,
    KabidYanmed,
    WadirYanmed,
    PetugasGudang,
    KabagKeuangan,
    AdminAset,
    Admin,
    Auditor,
}

impl Role {
    pub const ALL: [Role; 9] = [
        Role::UserRuangan,
        Role::KasiSarpras,
        Role::KabidYanmed,
        Role::WadirYanmed,
        Role::PetugasGudang,
        Role::KabagKeuangan,
        Role::AdminAset,
        Role::Admin,
        Role::Auditor,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::UserRuangan => "USER_RUANGAN",
            Role::KasiSarpras => "KASI_SARPRAS",
            Role::KabidYanmed => "KABID_YANMED",
            Role::WadirYanmed => "WADIR_YANMED",
            Role::PetugasGudang => "PETUGAS_GUDANG",
            Role::KabagKeuangan => "KABAG_KEUANGAN",
            Role::AdminAset => "ADMIN_ASET",
            Role::Admin => "ADMIN",
            Role::Auditor => "AUDITOR",
        }
    }

    pub fn from_kode(s: &str) -> Option<Role> {
        match s {
            "USER_RUANGAN" => Some(Role::UserRuangan),
            "KASI_SARPRAS" => Some(Role::KasiSarpras),
            "KABID_YANMED" => Some(Role::KabidYanmed),
            "WADIR_YANMED" => Some(Role::WadirYanmed),
            "PETUGAS_GUDANG" => Some(Role::PetugasGudang),
            "KABAG_KEUANGAN" => Some(Role::KabagKeuangan),
            "ADMIN_ASET" => Some(Role::AdminAset),
            "ADMIN" => Some(Role::Admin),
            "AUDITOR" => Some(Role::Auditor),
            _ => None,
        }
    }
}

/// Implementasi FromStr agar `"...".parse::<Role>()` bekerja.
impl std::str::FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Role::from_kode(s).ok_or(())
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Cek role terhadap daftar yang diizinkan. ADMIN (superuser) selalu lolos.
pub fn ensure(user_role: Role, allowed: &[Role]) -> AppResult<()> {
    if user_role == Role::Admin || allowed.contains(&user_role) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Validasi string role (untuk input API).
pub fn parse_role(s: &str) -> AppResult<Role> {
    Role::from_kode(s).ok_or_else(|| {
        AppError::Validation(format!(
            "role '{s}' tidak dikenal. Pilihan: {}",
            Role::ALL
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_serde_roundtrip() {
        assert_eq!(Role::from_kode("KASI_SARPRAS"), Some(Role::KasiSarpras));
        assert_eq!(Role::KasiSarpras.as_str(), "KASI_SARPRAS");
        assert_eq!(Role::from_kode("TIDAK_ADA"), None);
        let json = serde_json::to_string(&Role::WadirYanmed).unwrap();
        assert_eq!(json, r#""WADIR_YANMED""#);
    }

    #[test]
    fn admin_selalu_lolos() {
        assert!(ensure(Role::Admin, &[Role::UserRuangan]).is_ok());
        assert!(ensure(Role::KasiSarpras, &[Role::KasiSarpras]).is_ok());
        assert!(ensure(Role::UserRuangan, &[Role::KasiSarpras]).is_err());
    }
}
