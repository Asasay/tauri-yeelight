//! Cryptographic operations for miIO protocol.
//!
//! This module handles token processing, key derivation, and payload
//! encryption/decryption using AES-128-CBC.

use crate::types::MiioError;
use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

/// Parses a token string (32 hex characters) into 16 bytes.
///
/// # Arguments
/// * `token` - String containing 32 hex characters
///
/// # Returns
/// * `Ok(Vec<u8>)` - 16 bytes token
/// * `Err(MiioError::InvalidToken)` - If token is not 32 chars
/// * `Err(MiioError::InvalidTokenHex)` - If token contains invalid hex
pub fn parse_token_hex(token: &str) -> Result<Vec<u8>, MiioError> {
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

/// Computes MD5 hash of input bytes (public for protocol module).
pub fn md5_bytes(input: &[u8]) -> Vec<u8> {
    md5::compute(input).0.to_vec()
}

/// Derives encryption key and IV from a token.
///
/// The miIO protocol derives keys as:
/// - Key = MD5(token)
/// - IV = MD5(key + token)
///
/// # Arguments
/// * `token` - 16-byte token
///
/// # Returns
/// * `Ok((key, iv))` - 16-byte key and IV
/// * `Err(MiioError::Protocol)` - If token is not 16 bytes
pub fn miio_key_iv(token: &[u8]) -> Result<(Vec<u8>, Vec<u8>), MiioError> {
    if token.len() != 16 {
        return Err(MiioError::Protocol("token must be 16 bytes".to_string()));
    }
    let key = md5_bytes(token);
    let iv = md5_bytes(&[key.as_slice(), token].concat());
    Ok((key, iv))
}

/// Encrypts a payload using AES-128-CBC with PKCS7 padding.
///
/// # Arguments
/// * `payload` - Raw bytes to encrypt
/// * `key` - 16-byte encryption key
/// * `iv` - 16-byte initialization vector
///
/// # Returns
/// * `Ok(Vec<u8>)` - Encrypted payload with padding
/// * `Err(MiioError::Crypto)` - If encryption fails
pub fn miio_encrypt_payload(payload: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, MiioError> {
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

/// Decrypts a miIO response packet.
///
/// # Arguments
/// * `data` - Complete packet bytes (header + encrypted payload)
/// * `token` - 16-byte device token
///
/// # Returns
/// * `Ok(Vec<u8>)` - Decrypted payload
/// * `Err(MiioError::Protocol)` - If packet is too short
/// * `Err(MiioError::Crypto)` - If decryption fails
pub fn decrypt_miio_packet(data: &[u8], token: &[u8]) -> Result<Vec<u8>, MiioError> {
    const MIIO_HEADER_SIZE: usize = 32;

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