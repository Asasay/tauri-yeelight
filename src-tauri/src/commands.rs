//! Tauri command handlers.
//!
//! This module exposes the functionality to the frontend via Tauri's
//! command system. Each function corresponds to a JavaScript/TypeScript
//! callable command.

use crate::crypto::{miio_encrypt_payload, miio_key_iv, parse_token_hex};
use crate::network::{current_unix_ts, run_diagnostics};
use crate::protocol::build_miio_packet;
use crate::types::{
    DiagnosticsRequest, DiagnosticsReport, MiioCommandRequest, MiioCommandResponse, MiioError,
};
use serde_json::json;
use std::net::UdpSocket;
use std::time::Duration;

/// Sends a miIO command to a device and returns the response.
///
/// This is the main function for interacting with Yeelight devices.
/// It performs the hello handshake, encrypts the command, sends it,
/// and decrypts the response.
///
/// # Arguments
/// * `request` - Contains ip, token, method, params, and optional port
///
/// # Returns
/// * `Ok(MiioCommandResponse)` - Raw and parsed response
/// * `Err(String)` - Error message (for Tauri)
#[tauri::command]
pub fn send_miio_command(request: MiioCommandRequest) -> Result<MiioCommandResponse, String> {
    execute_miio_command(request).map_err(|e| e.to_string())
}

/// Runs diagnostic checks on a device.
///
/// Tests TCP and UDP connectivity to help troubleshoot connection issues.
///
/// # Arguments
/// * `request` - Contains ip and optional port
///
/// # Returns
/// * `Ok(DiagnosticsReport)` - Results of all diagnostic tests
/// * `Err(String)` - Error message (for Tauri)
#[tauri::command]
pub fn diagnose_connection(request: DiagnosticsRequest) -> Result<DiagnosticsReport, String> {
    run_diagnostics(request).map_err(|e| e.to_string())
}

/// Executes a miIO command (internal implementation).
fn execute_miio_command(request: MiioCommandRequest) -> Result<MiioCommandResponse, MiioError> {
    let token = parse_token_hex(&request.token)?;
    let (key, iv) = miio_key_iv(&token)?;

    let addr = format!("{}:{}", request.ip, request.port);
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(4)))?;
    socket.set_write_timeout(Some(Duration::from_secs(4)))?;

    let mut recv_buf = [0u8; 2048];
    let hello_info = crate::protocol::hello_handshake(&socket, &addr, &mut recv_buf)?;

    // python-miio keeps ids in [1, 9999]
    let cmd_id = ((current_unix_ts() % 9998) + 1) as u32;
    let payload_json = json!({
        "id": cmd_id,
        "method": request.method,
        "params": request.params
    });
    let mut payload = serde_json::to_vec(&payload_json)?;
    payload.push(0);

    let encrypted_payload = miio_encrypt_payload(&payload, &key, &iv)?;
    let send_stamp = hello_info.stamp.wrapping_add(1);
    let packet = build_miio_packet(hello_info.device_id, send_stamp, &token, &encrypted_payload)?;
    socket.send_to(&packet, &addr)?;

    let (resp_len, _) = socket.recv_from(&mut recv_buf)?;
    let response_packet = &recv_buf[..resp_len];
    let decrypted = crate::crypto::decrypt_miio_packet(response_packet, &token)?;
    let raw = String::from_utf8_lossy(&decrypted).to_string();
    let parsed = serde_json::from_slice::<serde_json::Value>(&decrypted).ok();

    Ok(MiioCommandResponse { raw, json: parsed })
}