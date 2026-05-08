//! Tauri backend for Yeelight miIO protocol communication.
//!
//! This crate provides commands to send miIO commands to Yeelight devices
//! and run connection diagnostics.
//!
//! ## Module Overview
//! - `types` - Data structures for requests/responses
//! - `crypto` - Token parsing and AES encryption
//! - `protocol` - Packet building and parsing
//! - `network` - Device discovery and diagnostics
//! - `commands` - Tauri command handlers
//! - `tray` - System tray and window event handling

pub mod commands;
pub mod crypto;
pub mod network;
pub mod protocol;
pub mod tray;
pub mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tray::setup_tray(app)?;
            tray::setup_window_close_handler(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_miio_command,
            commands::diagnose_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}