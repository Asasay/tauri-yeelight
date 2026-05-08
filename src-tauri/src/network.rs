//! Network operations for device discovery and diagnostics.
//!
//! This module handles TCP connectivity tests, UDP hello probes,
//! and broadcast scanning for device discovery.

use crate::protocol::{parse_hello_response, make_hello_packet};
use crate::types::{
    BroadcastDeviceSeen, BroadcastScanResult, DiagnosticsRequest, DiagnosticsReport, MiioError, TcpProbeResult,
    UdpHelloProbeResult, MIIO_PORT, YEELIGHT_PORT, RECV_BUFFER_SIZE, DEFAULT_UDP_TIMEOUT_SECS,
    HELLO_RETRY_COUNT, BROADCAST_SCAN_DURATION_MS,
};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Runs complete diagnostics on a device.
///
/// Tests:
/// 1. TCP connectivity on Yeelight port (55443)
/// 2. TCP connectivity on miIO port (54321)
/// 3. UDP unicast hello probe
/// 4. UDP broadcast scan
pub fn run_diagnostics(request: DiagnosticsRequest) -> Result<DiagnosticsReport, MiioError> {
    let target_ip = request.ip.trim().to_string();
    if target_ip.is_empty() {
        return Err(MiioError::Protocol("missing ip".to_string()));
    }

    let miio_port = request.port;
    let tcp_yeelight_55443 = probe_tcp_connect(&target_ip, YEELIGHT_PORT, Duration::from_secs(2));
    let tcp_miio_54321 = probe_tcp_connect(&target_ip, MIIO_PORT, Duration::from_secs(2));

    let udp_unicast_hello = probe_udp_hello_unicast(&target_ip, miio_port)?;

    let udp_broadcast_scan = scan_udp_broadcast_hello(miio_port, &target_ip, Duration::from_millis(BROADCAST_SCAN_DURATION_MS))?;

    Ok(DiagnosticsReport {
        target_ip,
        miio_port,
        tcp_yeelight_55443,
        tcp_miio_54321,
        udp_unicast_hello,
        udp_broadcast_scan,
    })
}

/// Probes TCP connectivity to a specific port.
///
/// Used to check if device ports are open (e.g., Yeelight port 55443).
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

/// Sends a unicast UDP hello packet and waits for response.
///
/// This is the first step in communicating with a miIO device.
fn probe_udp_hello_unicast(ip: &str, port: u16) -> Result<UdpHelloProbeResult, MiioError> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(DEFAULT_UDP_TIMEOUT_SECS)))?;
    socket.set_write_timeout(Some(Duration::from_secs(DEFAULT_UDP_TIMEOUT_SECS)))?;

    let addr = format!("{ip}:{port}");
    let hello = make_hello_packet();

    socket.send_to(&hello, &addr)?;

    let mut recv_buf = [0u8; RECV_BUFFER_SIZE];
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

/// Scans for devices using UDP broadcast.
///
/// Sends hello packets to broadcast address and collects responses
/// from all devices on the local network.
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
    for _ in 0..HELLO_RETRY_COUNT {
        let _ = socket.send_to(&hello, &broadcast_addr);
    }

    let deadline = Instant::now() + listen_duration;
    let mut devices_seen = Vec::new();
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut target_seen = false;

    let mut recv_buf = [0u8; RECV_BUFFER_SIZE];

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

/// Gets current Unix timestamp.
pub fn current_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}