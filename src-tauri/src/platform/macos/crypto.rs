// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! At-rest encryption on macOS: a random 256-bit key kept in the user's
//! login Keychain, used with ChaCha20-Poly1305 (fresh random nonce per item).
//! Only this user account can read the key.

use std::sync::OnceLock;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use security_framework::passwords::{get_generic_password, set_generic_password};

const SERVICE: &str = "com.tanmayangarkar.clipdeck";
const ACCOUNT: &str = "history-encryption-key";
const MAGIC: &[u8; 4] = b"CDE1";

static KEY: OnceLock<[u8; 32]> = OnceLock::new();

fn key() -> Result<&'static [u8; 32], String> {
    if let Some(key) = KEY.get() {
        return Ok(key);
    }
    let key = match get_generic_password(SERVICE, ACCOUNT) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            key
        }
        _ => {
            let mut key = [0u8; 32];
            getrandom::fill(&mut key).map_err(|e| format!("random key: {e}"))?;
            set_generic_password(SERVICE, ACCOUNT, &key)
                .map_err(|e| format!("store key in Keychain: {e}"))?;
            key
        }
    };
    Ok(KEY.get_or_init(|| key))
}

pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    seal(key()?, data)
}

pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    open(key()?, data)
}

fn seal(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|e| format!("random nonce: {e}"))?;
    let sealed = cipher
        .encrypt(Nonce::from_slice(&nonce), data)
        .map_err(|_| "encryption failed".to_string())?;
    let mut out = Vec::with_capacity(MAGIC.len() + nonce.len() + sealed.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Ok(out)
}

fn open(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 16 || &data[..4] != MAGIC {
        return Err("not an encrypted ClipDeck item".into());
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&data[4..16]), &data[16..])
        .map_err(|_| "decryption failed (wrong key or damaged data)".to_string())
}

// The Keychain half is left to manual testing: a unit test must not create
// items in the developer's real login Keychain.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_tamper() {
        let key = [7u8; 32];
        let sealed = seal(&key, b"secret").unwrap();
        assert!(sealed.starts_with(MAGIC));
        assert!(!sealed.windows(6).any(|w| w == b"secret"));
        assert_eq!(open(&key, &sealed).unwrap(), b"secret");
        // Fresh nonce each time.
        assert_ne!(seal(&key, b"secret").unwrap(), sealed);

        let mut damaged = sealed.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(open(&key, &damaged).is_err());
        assert!(open(&[8u8; 32], &sealed).is_err());
        assert!(open(&key, b"CDK1 plain").is_err());
    }
}
