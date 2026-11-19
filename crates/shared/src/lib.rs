//! sarpras-shared — logika bersama: error, envelope API, auth (JWT+argon2),
//! RBAC, config loader (moka cache + resolusi scope + snapshot + simulasi),
//! dan AppConfig (figment).

pub mod api;
pub mod app_config;
pub mod auth;
pub mod config;
pub mod error;
pub mod excel;
pub mod rbac;
