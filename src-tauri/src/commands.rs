//! Tauri command handlers.
//!
//! This module exposes the functionality to the frontend via Tauri's
//! command system. Each function corresponds to a JavaScript/TypeScript
//! callable command.

use crate::crypto::{miio_encrypt_payload, miio_key_iv, parse_token_hex, decrypt_miio_packet};
use crate::network::{current_unix_ts, run_diagnostics};
use crate::protocol::{build_miio_packet, hello_handshake};
use crate::types::{
    DiagnosticsRequest, DiagnosticsReport, MiioCommandRequest, MiioCommandResponse, MiioError,
    RECV_BUFFER_SIZE,
};
use serde_json::json;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// Sends a miIO command to a device and returns the response.
#[tauri::command]
pub async fn send_miio_command(request: MiioCommandRequest) -> Result<MiioCommandResponse, String> {
    execute_miio_command(request).await.map_err(|e| e.to_string())
}

/// Runs diagnostic checks on a device.
#[tauri::command]
pub async fn diagnose_connection(request: DiagnosticsRequest) -> Result<DiagnosticsReport, String> {
    run_diagnostics(request).await.map_err(|e| e.to_string())
}

/// Executes a miIO command (internal implementation).
async fn execute_miio_command(request: MiioCommandRequest) -> Result<MiioCommandResponse, MiioError> {
    let token = parse_token_hex(&request.token)?;
    let (key, iv) = miio_key_iv(&token)?;

    let addr = format!("{}:{}", request.ip, request.port);
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    let mut recv_buf = [0u8; RECV_BUFFER_SIZE];
    let hello_info = hello_handshake(&addr).await?;

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
    socket.send_to(&packet, &addr).await?;

    let (resp_len, _) = match timeout(Duration::from_secs(4), socket.recv_from(&mut recv_buf)).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(MiioError::Socket(e)),
        Err(_) => return Err(MiioError::Protocol("recv timed out".to_string())),
    };
    let response_packet = &recv_buf[..resp_len];
    let decrypted = decrypt_miio_packet(response_packet, &token)?;
    let raw = String::from_utf8_lossy(&decrypted).to_string();
    let parsed = serde_json::from_slice::<serde_json::Value>(&decrypted).ok();

    Ok(MiioCommandResponse { raw, json: parsed })
}