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

pub mod commands;
pub mod crypto;
pub mod network;
pub mod protocol;
pub mod types;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let toggle_item = MenuItem::with_id(app, "toggle", "Toggle Power", true, None::<&str>)?;
            let moonlight_item = MenuItem::with_id(app, "moonlight", "Toggle Moonlight", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&toggle_item, &moonlight_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        let _ = app.emit("tray-toggle", ());
                    }
                    "moonlight" => {
                        let _ = app.emit("tray-moonlight", ());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let visible = window.is_visible().unwrap_or(false);
                            if visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::CloseRequested { api: _, .. } => {
                            let _ = window_clone.close();
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_miio_command,
            commands::diagnose_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}