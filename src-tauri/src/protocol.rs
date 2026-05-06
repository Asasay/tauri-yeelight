//! MiIO protocol packet handling.
//!
//! This module handles building and parsing miIO protocol packets,
//! including the initial hello handshake and command/response packets.

use crate::types::{HelloResponse, MiioError};
use crate::crypto::md5_bytes;

/// Magic number that identifies miIO protocol packets.
const MIIO_MAGIC: u16 = 0x2131;

/// Size of the miIO packet header (not including checksum).
const MIIO_HEADER_SIZE: usize = 32;

/// Builds a complete miIO command packet.
///
/// The packet structure is:
/// - 16-byte header (magic, length, unknown, device_id, stamp)
/// - 16-byte MD5 checksum
/// - Encrypted payload
///
/// # Arguments
/// * `device_id` - 4-byte device identifier
/// * `stamp` - Timestamp (must be derived from hello response + 1)
/// * `token` - 16-byte device token
/// * `encrypted_payload` - Already-encrypted command data
///
/// # Returns
/// * `Ok(Vec<u8>)` - Complete packet ready to send
/// * `Err(MiioError)` - If packet construction fails
pub fn build_miio_packet(
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

/// Creates a hello packet for device discovery.
///
/// The hello packet is a simple UDP broadcast that devices respond to
/// with their device ID and timestamp (needed for encryption).
///
/// # Returns
/// * `[u8; 32]` - Hello packet with magic number
pub fn make_hello_packet() -> [u8; MIIO_HEADER_SIZE] {
    let mut packet = [0xffu8; MIIO_HEADER_SIZE];
    packet[0..2].copy_from_slice(&MIIO_MAGIC.to_be_bytes());
    packet[2..4].copy_from_slice(&(MIIO_HEADER_SIZE as u16).to_be_bytes());
    packet
}

/// Parses a hello response from a device.
///
/// # Arguments
/// * `data` - Raw bytes received from device
///
/// # Returns
/// * `Ok(HelloResponse)` - Parsed device ID and timestamp
/// * `Err(MiioError::Protocol)` - If response is invalid
pub fn parse_hello_response(data: &[u8]) -> Result<HelloResponse, MiioError> {
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

/// Performs the initial hello handshake with a device.
///
/// This sends a hello packet and waits for a response containing the
/// device ID and timestamp needed for encrypted communication.
///
/// # Arguments
/// * `socket` - UDP socket to use
/// * `addr` - Target address (ip:port)
/// * `recv_buf` - Buffer to store response
///
/// # Returns
/// * `Ok(HelloResponse)` - Device ID and timestamp
/// * `Err(MiioError)` - If handshake fails
pub fn hello_handshake(
    socket: &std::net::UdpSocket,
    addr: &str,
    recv_buf: &mut [u8; 2048],
) -> Result<HelloResponse, MiioError> {
    use std::io::ErrorKind;

    let (target_ip, target_port) = split_host_port(addr)?;
    let hello = make_hello_packet();
    let mut last_timeout = false;

    // Try unicast first
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

    // Fall back to broadcast (some devices only respond to broadcast)
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

/// Splits an address string into IP and port components.
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