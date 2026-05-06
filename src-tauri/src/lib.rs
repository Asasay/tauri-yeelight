use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

const MIIO_MAGIC: u16 = 0x2131;
const MIIO_HEADER_SIZE: usize = 32;
const DEFAULT_MIIO_PORT: u16 = 54321;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MiioCommandRequest {
    ip: String,
    token: String,
    method: String,
    params: serde_json::Value,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 {
    DEFAULT_MIIO_PORT
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsRequest {
    ip: String,
    #[serde(default = "default_port")]
    port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TcpProbeResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UdpHelloProbeResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    did: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stamp: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BroadcastDeviceSeen {
    ip: String,
    did: u32,
    stamp: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BroadcastScanResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(default)]
    devices_seen: Vec<BroadcastDeviceSeen>,
    target_seen: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsReport {
    target_ip: String,
    miio_port: u16,
    tcp_yeelight_55443: TcpProbeResult,
    tcp_miio_54321: TcpProbeResult,
    udp_unicast_hello: UdpHelloProbeResult,
    udp_broadcast_scan: BroadcastScanResult,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MiioCommandResponse {
    raw: String,
    json: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
enum MiioError {
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

#[derive(Debug)]
struct HelloResponse {
    device_id: [u8; 4],
    stamp: u32,
}

#[tauri::command]
fn send_miio_command(request: MiioCommandRequest) -> Result<MiioCommandResponse, String> {
    execute_miio_command(request).map_err(|e| e.to_string())
}

#[tauri::command]
fn diagnose_connection(request: DiagnosticsRequest) -> Result<DiagnosticsReport, String> {
    run_diagnostics(request).map_err(|e| e.to_string())
}

fn execute_miio_command(request: MiioCommandRequest) -> Result<MiioCommandResponse, MiioError> {
    let token = parse_token_hex(&request.token)?;
    let (key, iv) = miio_key_iv(&token)?;

    let addr = format!("{}:{}", request.ip, request.port);
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(4)))?;
    socket.set_write_timeout(Some(Duration::from_secs(4)))?;

    let mut recv_buf = [0u8; 2048];
    let hello_info = hello_handshake(&socket, &addr, &mut recv_buf)?;

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
    let decrypted = decrypt_miio_packet(response_packet, &token)?;
    let raw = String::from_utf8_lossy(&decrypted).to_string();
    let parsed = serde_json::from_slice::<serde_json::Value>(&decrypted).ok();

    Ok(MiioCommandResponse { raw, json: parsed })
}

fn run_diagnostics(request: DiagnosticsRequest) -> Result<DiagnosticsReport, MiioError> {
    let target_ip = request.ip.trim().to_string();
    if target_ip.is_empty() {
        return Err(MiioError::Protocol("missing ip".to_string()));
    }

    let miio_port = request.port;
    let tcp_yeelight_55443 = probe_tcp_connect(&target_ip, 55443, Duration::from_secs(2));
    let tcp_miio_54321 = probe_tcp_connect(&target_ip, 54321, Duration::from_secs(2));

    let udp_unicast_hello = probe_udp_hello_unicast(&target_ip, miio_port)?;

    let udp_broadcast_scan = scan_udp_broadcast_hello(miio_port, &target_ip, Duration::from_millis(2500))?;

    Ok(DiagnosticsReport {
        target_ip,
        miio_port,
        tcp_yeelight_55443,
        tcp_miio_54321,
        udp_unicast_hello,
        udp_broadcast_scan,
    })
}

fn probe_tcp_connect(ip: &str, port: u16, timeout: Duration) -> TcpProbeResult {
    let addr: SocketAddr = match format!("{ip}:{port}").parse() {
        Ok(addr) => addr,
        Err(err) => {
            return TcpProbeResult {
                ok: false,
                error: Some(format!("invalid address: {err}")),
            }
        }
    };

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => TcpProbeResult {
            ok: true,
            error: None,
        },
        Err(err) => TcpProbeResult {
            ok: false,
            error: Some(err.to_string()),
        },
    }
}

fn probe_udp_hello_unicast(ip: &str, port: u16) -> Result<UdpHelloProbeResult, MiioError> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.set_write_timeout(Some(Duration::from_secs(3)))?;

    let addr = format!("{ip}:{port}");
    let hello = make_hello_packet();

    socket.send_to(&hello, &addr)?;

    let mut recv_buf = [0u8; 2048];
    match socket.recv_from(&mut recv_buf) {
        Ok((len, _)) => match parse_hello_response(&recv_buf[..len]) {
            Ok(parsed) => Ok(UdpHelloProbeResult {
                ok: true,
                error: None,
                did: Some(u32::from_be_bytes(parsed.device_id)),
                stamp: Some(parsed.stamp),
            }),
            Err(err) => Ok(UdpHelloProbeResult {
                ok: false,
                error: Some(err.to_string()),
                did: None,
                stamp: None,
            }),
        },
        Err(err) if err.kind() == ErrorKind::TimedOut || err.kind() == ErrorKind::WouldBlock => {
            Ok(UdpHelloProbeResult {
                ok: false,
                error: Some(format!("no UDP reply on {addr} within timeout ({err})")),
                did: None,
                stamp: None,
            })
        }
        Err(err) => Err(MiioError::Socket(err)),
    }
}

fn scan_udp_broadcast_hello(
    port: u16,
    target_ip: &str,
    listen_duration: Duration,
) -> Result<BroadcastScanResult, MiioError> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(200)))?;
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    socket.set_broadcast(true)?;

    let hello = make_hello_packet();
    let broadcast_addr = format!("255.255.255.255:{port}");

    // Prime a few broadcasts; some devices answer only after repeated discovery.
    for _ in 0..3 {
        let _ = socket.send_to(&hello, &broadcast_addr);
    }

    let deadline = Instant::now() + listen_duration;
    let mut devices_seen = Vec::new();
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut target_seen = false;

    let mut recv_buf = [0u8; 2048];

    while Instant::now() < deadline {
        match socket.recv_from(&mut recv_buf) {
            Ok((len, src)) => {
                let src_ip = src.ip().to_string();
                let Ok(parsed) = parse_hello_response(&recv_buf[..len]) else {
                    continue;
                };

                let did = u32::from_be_bytes(parsed.device_id);
                let key = format!("{src_ip}:{did}");
                if seen_keys.insert(key) {
                    devices_seen.push(BroadcastDeviceSeen {
                        ip: src_ip.clone(),
                        did,
                        stamp: parsed.stamp,
                    });
                }

                if src_ip == target_ip {
                    target_seen = true;
                }
            }
            Err(err) if err.kind() == ErrorKind::TimedOut || err.kind() == ErrorKind::WouldBlock => {
                let _ = socket.send_to(&hello, &broadcast_addr);
            }
            Err(err) => return Err(MiioError::Socket(err)),
        }
    }

    Ok(BroadcastScanResult {
        ok: !devices_seen.is_empty(),
        error: if devices_seen.is_empty() {
            Some("no miIO hello replies observed on LAN broadcast during scan window".to_string())
        } else {
            None
        },
        devices_seen,
        target_seen,
    })
}

fn split_host_port(addr: &str) -> Result<(String, u16), MiioError> {
    let mut parts = addr.rsplitn(2, ':');
    let port_str = parts
        .next()
        .ok_or_else(|| MiioError::Protocol("invalid target address".to_string()))?;
    let host_part = parts
        .next()
        .ok_or_else(|| MiioError::Protocol("invalid target address".to_string()))?;

    let port: u16 = port_str
        .parse()
        .map_err(|err| MiioError::Protocol(format!("invalid miIO port: {err}")))?;

    Ok((host_part.to_string(), port))
}

fn hello_handshake(
    socket: &UdpSocket,
    addr: &str,
    recv_buf: &mut [u8; 2048],
) -> Result<HelloResponse, MiioError> {
    let (target_ip, target_port) = split_host_port(addr)?;
    let hello = make_hello_packet();
    let mut last_timeout = false;

    for _ in 0..3 {
        socket.send_to(&hello, addr)?;
        match socket.recv_from(recv_buf) {
            Ok((hello_len, _)) => return parse_hello_response(&recv_buf[..hello_len]),
            Err(err) if err.kind() == ErrorKind::TimedOut || err.kind() == ErrorKind::WouldBlock => {
                last_timeout = true;
            }
            Err(err) => return Err(MiioError::Socket(err)),
        }
    }

    // Some devices only answer miIO hello on LAN broadcast.
    socket.set_broadcast(true)?;
    let broadcast_addr = format!("255.255.255.255:{target_port}");
    for _ in 0..3 {
        socket.send_to(&hello, &broadcast_addr)?;
        match socket.recv_from(recv_buf) {
            Ok((hello_len, src)) => {
                if src.ip().to_string() == target_ip {
                    return parse_hello_response(&recv_buf[..hello_len]);
                }
            }
            Err(err) if err.kind() == ErrorKind::TimedOut || err.kind() == ErrorKind::WouldBlock => {
                last_timeout = true;
            }
            Err(err) => return Err(MiioError::Socket(err)),
        }
    }

    if last_timeout {
        return Err(MiioError::Protocol(
            "no miIO hello response from target device (unicast+broadcast failed). Device may block local miIO on current firmware/network mode"
                .to_string(),
        ));
    }
    Err(MiioError::Protocol("miIO hello handshake failed".to_string()))
}

fn parse_token_hex(token: &str) -> Result<Vec<u8>, MiioError> {
    let normalized = token.trim();
    if normalized.len() != 32 {
        return Err(MiioError::InvalidToken);
    }

    let mut out = Vec::with_capacity(16);
    for idx in (0..normalized.len()).step_by(2) {
        let b = u8::from_str_radix(&normalized[idx..idx + 2], 16)?;
        out.push(b);
    }
    Ok(out)
}

fn md5_bytes(input: &[u8]) -> Vec<u8> {
    md5::compute(input).0.to_vec()
}

fn make_hello_packet() -> [u8; MIIO_HEADER_SIZE] {
    let mut packet = [0xffu8; MIIO_HEADER_SIZE];
    packet[0..2].copy_from_slice(&MIIO_MAGIC.to_be_bytes());
    packet[2..4].copy_from_slice(&(MIIO_HEADER_SIZE as u16).to_be_bytes());
    packet
}

fn parse_hello_response(data: &[u8]) -> Result<HelloResponse, MiioError> {
    if data.len() < MIIO_HEADER_SIZE {
        return Err(MiioError::Protocol("hello response too short".to_string()));
    }
    let magic = u16::from_be_bytes([data[0], data[1]]);
    if magic != MIIO_MAGIC {
        return Err(MiioError::Protocol("invalid magic in hello response".to_string()));
    }

    let device_id: [u8; 4] = data[8..12]
        .try_into()
        .map_err(|_| MiioError::Protocol("hello response missing device id".to_string()))?;
    let stamp = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    Ok(HelloResponse { device_id, stamp })
}

fn miio_encrypt_payload(payload: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, MiioError> {
    let cipher = Aes128CbcEnc::new_from_slices(key, iv)
        .map_err(|e| MiioError::Crypto(format!("invalid key/iv: {e}")))?;
    let mut buf = payload.to_vec();
    let msg_len = buf.len();
    buf.resize(msg_len + 16, 0u8);
    let encrypted = cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buf, msg_len)
        .map_err(|e| MiioError::Crypto(format!("failed to encrypt payload: {e}")))?;
    Ok(encrypted.to_vec())
}

fn build_miio_packet(
    device_id: [u8; 4],
    stamp: u32,
    token: &[u8],
    encrypted_payload: &[u8],
) -> Result<Vec<u8>, MiioError> {
    let data_len = encrypted_payload.len();
    let total_len = MIIO_HEADER_SIZE + data_len;

    let mut header = [0u8; 16];
    header[0..2].copy_from_slice(&MIIO_MAGIC.to_be_bytes());
    header[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    header[4..8].copy_from_slice(&0u32.to_be_bytes()); // unknown
    header[8..12].copy_from_slice(&device_id);
    header[12..16].copy_from_slice(&stamp.to_be_bytes());

    let digest_input = [header.as_slice(), token, encrypted_payload].concat();
    let checksum = md5_bytes(&digest_input);
    if checksum.len() != 16 {
        return Err(MiioError::Crypto("invalid md5 checksum length".to_string()));
    }

    let mut packet = Vec::with_capacity(total_len);
    packet.extend_from_slice(&header);
    packet.extend_from_slice(&checksum);
    packet.extend_from_slice(encrypted_payload);

    Ok(packet)
}

fn miio_key_iv(token: &[u8]) -> Result<(Vec<u8>, Vec<u8>), MiioError> {
    if token.len() != 16 {
        return Err(MiioError::Protocol("token must be 16 bytes".to_string()));
    }
    let key = md5_bytes(token);
    let iv = md5_bytes(&[key.as_slice(), token].concat());
    Ok((key, iv))
}

fn decrypt_miio_packet(data: &[u8], token: &[u8]) -> Result<Vec<u8>, MiioError> {
    if data.len() < MIIO_HEADER_SIZE {
        return Err(MiioError::Protocol("response too short".to_string()));
    }

    let ciphertext = &data[MIIO_HEADER_SIZE..];
    if ciphertext.is_empty() {
        return Ok(Vec::new());
    }

    let (key, iv) = miio_key_iv(token)?;
    let cipher = Aes128CbcDec::new_from_slices(&key, &iv)
        .map_err(|e| MiioError::Crypto(format!("invalid key/iv: {e}")))?;
    let mut buf = ciphertext.to_vec();
    let decrypted = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| MiioError::Crypto(format!("failed to decrypt response: {e}")))?;

    let mut out = decrypted.to_vec();
    while out.last() == Some(&0) {
        out.pop();
    }
    Ok(out)
}

fn current_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![send_miio_command, diagnose_connection])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
