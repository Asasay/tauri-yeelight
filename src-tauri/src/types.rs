//! Data types for miIO protocol communication.
//!
//! This module contains all the request/response structures used to communicate
//! with Yeelight devices via the miIO protocol.

use serde::{Deserialize, Serialize};

/// Default port for miIO protocol communication.
pub const DEFAULT_MIIO_PORT: u16 = 54321;

/// Alternative port for Yeelight (TCP).
pub const YEELIGHT_PORT: u16 = 55443;

/// Port for miIO protocol (UDP).
pub const MIIO_PORT: u16 = 54321;

/// Magic number that identifies miIO protocol packets.
pub const MIIO_MAGIC: u16 = 0x2131;

/// Size of the miIO packet header (not including checksum).
pub const MIIO_HEADER_SIZE: usize = 32;

/// Default UDP receive buffer size.
pub const RECV_BUFFER_SIZE: usize = 2048;

/// Default timeout for UDP operations (seconds).
pub const DEFAULT_UDP_TIMEOUT_SECS: u64 = 3;

/// Hello handshake retry attempts.
pub const HELLO_RETRY_COUNT: usize = 3;

/// Broadcast scan duration (milliseconds).
pub const BROADCAST_SCAN_DURATION_MS: u64 = 2500;

/// Request to send a miIO command to a device.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiioCommandRequest {
    /// Device IP address.
    pub ip: String,
    /// Device token (32 hex characters).
    pub token: String,
    /// Method name to invoke on device.
    pub method: String,
    /// Parameters for the method.
    pub params: serde_json::Value,
    /// Port number (defaults to 54321).
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_port() -> u16 {
    DEFAULT_MIIO_PORT
}

/// Request to run diagnostic checks on a device.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRequest {
    /// Device IP address.
    pub ip: String,
    /// Port number (defaults to 54321).
    #[serde(default = "default_port")]
    pub port: u16,
}

/// Result of a TCP connectivity probe.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpProbeResult {
    /// Whether the connection succeeded.
    pub ok: bool,
    /// Error message if connection failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a UDP hello probe (unicast).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UdpHelloProbeResult {
    /// Whether we received a valid hello response.
    pub ok: bool,
    /// Error message if probe failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Device ID from the hello response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<u32>,
    /// Timestamp from the hello response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stamp: Option<u32>,
}

/// A device seen during UDP broadcast scan.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastDeviceSeen {
    /// Device IP address.
    pub ip: String,
    /// Device ID.
    pub did: u32,
    /// Timestamp from hello packet.
    pub stamp: u32,
}

/// Result of UDP broadcast scan for device discovery.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastScanResult {
    /// Whether any devices were found.
    pub ok: bool,
    /// Error message if no devices found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// List of devices that responded to broadcast.
    #[serde(default)]
    pub devices_seen: Vec<BroadcastDeviceSeen>,
    /// Whether the target device was seen.
    pub target_seen: bool,
}

/// Complete diagnostic report for a device.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    /// Target IP that was probed.
    pub target_ip: String,
    /// miIO port used.
    pub miio_port: u16,
    /// TCP probe on Yeelight port (55443).
    pub tcp_yeelight_55443: TcpProbeResult,
    /// TCP probe on miIO port (54321).
    pub tcp_miio_54321: TcpProbeResult,
    /// UDP unicast hello probe result.
    pub udp_unicast_hello: UdpHelloProbeResult,
    /// UDP broadcast scan result.
    pub udp_broadcast_scan: BroadcastScanResult,
}

/// Response from a miIO command.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiioCommandResponse {
    /// Raw UTF-8 string of decrypted response.
    pub raw: String,
    /// Parsed JSON response (if valid).
    pub json: Option<serde_json::Value>,
}

/// Errors that can occur during miIO operations.
#[derive(Debug, thiserror::Error)]
pub enum MiioError {
    #[error("invalid token: expected 32 hex characters")]
    InvalidToken,

    #[error("failed to parse token hex")]
    InvalidTokenHex(#[from] std::num::ParseIntError),

    #[error("socket error: {0}")]
    Socket(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Internal: parsed hello response from a device.
#[derive(Debug)]
pub struct HelloResponse {
    /// Device ID as 4 bytes.
    pub device_id: [u8; 4],
    /// Timestamp from device.
    pub stamp: u32,
}