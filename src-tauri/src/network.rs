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
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Runs complete diagnostics on a device.
pub async fn run_diagnostics(request: DiagnosticsRequest) -> Result<DiagnosticsReport, MiioError> {
    let target_ip = request.ip.trim().to_string();
    if target_ip.is_empty() {
        return Err(MiioError::Protocol("missing ip".to_string()));
    }

    let miio_port = request.port;

    let tcp_yeelight_55443 = probe_tcp_connect(&target_ip, YEELIGHT_PORT, Duration::from_secs(2)).await;
    let tcp_miio_54321 = probe_tcp_connect(&target_ip, MIIO_PORT, Duration::from_secs(2)).await;

    let udp_unicast_hello = probe_udp_hello_unicast(&target_ip, miio_port).await?;
    let udp_broadcast_scan = scan_udp_broadcast_hello(miio_port, &target_ip, Duration::from_millis(BROADCAST_SCAN_DURATION_MS)).await?;

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
async fn probe_tcp_connect(ip: &str, port: u16, timeout_duration: Duration) -> TcpProbeResult {
    let addr: SocketAddr = match format!("{ip}:{port}").parse() {
        Ok(addr) => addr,
        Err(err) => {
            return TcpProbeResult {
                ok: false,
                error: Some(format!("invalid address: {err}")),
            }
        }
    };

    match timeout(timeout_duration, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => TcpProbeResult {
            ok: true,
            error: None,
        },
        Ok(Err(err)) => TcpProbeResult {
            ok: false,
            error: Some(err.to_string()),
        },
        Err(_) => TcpProbeResult {
            ok: false,
            error: Some("connection timed out".to_string()),
        },
    }
}

/// Sends a unicast UDP hello packet and waits for response.
async fn probe_udp_hello_unicast(ip: &str, port: u16) -> Result<UdpHelloProbeResult, MiioError> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;

    let addr = format!("{ip}:{port}");
    let hello = make_hello_packet();

    socket.send_to(&hello, &addr).await?;

    let mut recv_buf = [0u8; RECV_BUFFER_SIZE];
    match timeout(
        Duration::from_secs(DEFAULT_UDP_TIMEOUT_SECS),
        socket.recv_from(&mut recv_buf),
    )
    .await
    {
        Ok(Ok((len, _))) => match parse_hello_response(&recv_buf[..len]) {
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
        Ok(Err(err)) => Ok(UdpHelloProbeResult {
            ok: false,
            error: Some(err.to_string()),
            did: None,
            stamp: None,
        }),
        Err(_) => Ok(UdpHelloProbeResult {
            ok: false,
            error: Some(format!("no UDP reply on {addr} within timeout")),
            did: None,
            stamp: None,
        }),
    }
}

/// Scans for devices using UDP broadcast.
async fn scan_udp_broadcast_hello(
    port: u16,
    target_ip: &str,
    listen_duration: Duration,
) -> Result<BroadcastScanResult, MiioError> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    let hello = make_hello_packet();
    let broadcast_addr = format!("255.255.255.255:{port}");

    for _ in 0..HELLO_RETRY_COUNT {
        let _ = socket.send_to(&hello, &broadcast_addr).await;
    }

    let deadline = tokio::time::Instant::now() + listen_duration;
    let mut devices_seen = Vec::new();
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut target_seen = false;

    let mut recv_buf = [0u8; RECV_BUFFER_SIZE];

    while tokio::time::Instant::now() < deadline {
        let receive_timeout = tokio::time::timeout(Duration::from_millis(200), socket.recv_from(&mut recv_buf));
        
        match receive_timeout.await {
            Ok(Ok((len, src))) => {
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
            Ok(Err(_)) => {
                let _ = socket.send_to(&hello, &broadcast_addr).await;
            }
            Err(_) => {
                let _ = socket.send_to(&hello, &broadcast_addr).await;
            }
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