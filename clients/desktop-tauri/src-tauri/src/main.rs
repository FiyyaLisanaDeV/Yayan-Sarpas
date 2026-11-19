//! Desktop shell Tauri 2.
//!
//! Fase 1: window memuat SPA yang sama dengan web client; token disimpan
//! di localStorage webview. Draft offline (rusqlite) + sync queue menyusul Fase 4.
//!
//! Catatan build: crate Tauri ini standalone (bukan anggota Cargo workspace Rust)
//! agar target build desktop terpisah dari workspace server.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("gagal menjalankan aplikasi Tauri");
}
