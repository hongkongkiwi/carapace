//! Configuration encryption at rest
//!
//! Provides transparent encryption/decryption of sensitive configuration fields
//! using a master key stored in the OS credential store.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Encryption errors
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Failed to access credential store: {0}")]
    CredentialStore(String),

    #[error("Encryption failed: {0}")]
    Encryption(String),

    #[error("Decryption failed: {0}")]
    Decryption(String),

    #[error("Invalid ciphertext format: {0}")]
    InvalidFormat(String),

    #[error("Master key not found in credential store")]
    MasterKeyNotFound,

    #[error("Failed to generate master key: {0}")]
    KeyGeneration(String),
}

/// Marker for encrypted values in JSON
pub const ENCRYPTED_PREFIX: &str = "ENC:";

/// Service name for credential store
const CREDENTIAL_SERVICE: &str = "carapace-config-encryption";

/// Account name for master key
const MASTER_KEY_ACCOUNT: &str = "master-key";

/// Fields that should be encrypted at rest
const SENSITIVE_FIELDS: &[&str] = &[
    "apiKey",
    "api_key",
    "token",
    "auth_token",
    "access_token",
    "refresh_token",
    "password",
    "secret",
    "client_secret",
    "private_key",
    "credential",
    "bottoken",
    "bot_token",
    "webhook_secret",
    "signing_secret",
];

/// Check if a field name indicates sensitive content
fn is_sensitive_field(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    SENSITIVE_FIELDS.iter().any(|&s| name_lower.contains(s))
}

/// Generate a new 256-bit master key
fn generate_master_key() -> Vec<u8> {
    use aes_gcm::aead::rand_core::RngCore;
    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

/// Store the master key in the OS credential store
#[cfg(target_os = "macos")]
fn store_master_key(key: &[u8]) -> Result<(), EncryptionError> {
    use keyring::Entry;

    let entry = Entry::new(CREDENTIAL_SERVICE, MASTER_KEY_ACCOUNT)
        .map_err(|e| EncryptionError::CredentialStore(e.to_string()))?;

    let key_b64 = BASE64.encode(key);
    entry
        .set_password(&key_b64)
        .map_err(|e| EncryptionError::CredentialStore(e.to_string()))?;

    debug!("Master key stored in macOS Keychain");
    Ok(())
}

/// Store the master key in the OS credential store
#[cfg(target_os = "linux")]
fn store_master_key(key: &[u8]) -> Result<(), EncryptionError> {
    use keyring::Entry;

    let entry = Entry::new(CREDENTIAL_SERVICE, MASTER_KEY_ACCOUNT)
        .map_err(|e| EncryptionError::CredentialStore(e.to_string()))?;

    let key_b64 = BASE64.encode(key);
    entry
        .set_password(&key_b64)
        .map_err(|e| EncryptionError::CredentialStore(e.to_string()))?;

    debug!("Master key stored in Linux Secret Service");
    Ok(())
}

/// Store the master key in the OS credential store
#[cfg(target_os = "windows")]
fn store_master_key(key: &[u8]) -> Result<(), EncryptionError> {
    use keyring::Entry;

    let entry = Entry::new(CREDENTIAL_SERVICE, MASTER_KEY_ACCOUNT)
        .map_err(|e| EncryptionError::CredentialStore(e.to_string()))?;

    let key_b64 = BASE64.encode(key);
    entry
        .set_password(&key_b64)
        .map_err(|e| EncryptionError::CredentialStore(e.to_string()))?;

    debug!("Master key stored in Windows Credential Manager");
    Ok(())
}

/// Retrieve the master key from the OS credential store
#[cfg(target_os = "macos")]
fn retrieve_master_key() -> Result<Option<Vec<u8>>, EncryptionError> {
    use keyring::Entry;

    let entry = match Entry::new(CREDENTIAL_SERVICE, MASTER_KEY_ACCOUNT) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    match entry.get_password() {
        Ok(key_b64) => {
            let key = BASE64
                .decode(key_b64)
                .map_err(|e| EncryptionError::CredentialStore(format!("Invalid key format: {}", e)))?;
            Ok(Some(key))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(EncryptionError::CredentialStore(e.to_string())),
    }
}

/// Retrieve the master key from the OS credential store
#[cfg(target_os = "linux")]
fn retrieve_master_key() -> Result<Option<Vec<u8>>, EncryptionError> {
    use keyring::Entry;

    let entry = match Entry::new(CREDENTIAL_SERVICE, MASTER_KEY_ACCOUNT) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    match entry.get_password() {
        Ok(key_b64) => {
            let key = BASE64
                .decode(key_b64)
                .map_err(|e| EncryptionError::CredentialStore(format!("Invalid key format: {}", e)))?;
            Ok(Some(key))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(EncryptionError::CredentialStore(e.to_string())),
    }
}

/// Retrieve the master key from the OS credential store
#[cfg(target_os = "windows")]
fn retrieve_master_key() -> Result<Option<Vec<u8>>, EncryptionError> {
    use keyring::Entry;

    let entry = match Entry::new(CREDENTIAL_SERVICE, MASTER_KEY_ACCOUNT) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    match entry.get_password() {
        Ok(key_b64) => {
            let key = BASE64
                .decode(key_b64)
                .map_err(|e| EncryptionError::CredentialStore(format!("Invalid key format: {}", e)))?;
            Ok(Some(key))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(EncryptionError::CredentialStore(e.to_string())),
    }
}

/// Get or create the master encryption key
pub fn get_or_create_master_key() -> Result<Vec<u8>, EncryptionError> {
    if let Some(key) = retrieve_master_key()? {
        debug!("Retrieved existing master key from credential store");
        return Ok(key);
    }

    info!("Generating new master encryption key...");
    let key = generate_master_key();
    store_master_key(&key)?;
    info!("Master encryption key stored securely in OS credential store");

    Ok(key)
}

/// Encrypt a single value
fn encrypt_value(plaintext: &str, cipher: &Aes256Gcm) -> Result<String, EncryptionError> {
    use aes_gcm::aead::rand_core::RngCore;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

    // Combine nonce + ciphertext and encode
    let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(format!("{}{}", ENCRYPTED_PREFIX, BASE64.encode(combined)))
}

/// Decrypt a single value
fn decrypt_value(ciphertext: &str, cipher: &Aes256Gcm) -> Result<String, EncryptionError> {
    if !ciphertext.starts_with(ENCRYPTED_PREFIX) {
        return Err(EncryptionError::InvalidFormat(
            "Value does not have encrypted prefix".to_string(),
        ));
    }

    let encoded = &ciphertext[ENCRYPTED_PREFIX.len()..];
    let combined = BASE64
        .decode(encoded)
        .map_err(|e| EncryptionError::InvalidFormat(format!("Invalid base64: {}", e)))?;

    if combined.len() < 12 {
        return Err(EncryptionError::InvalidFormat(
            "Ciphertext too short".to_string(),
        ));
    }

    let (nonce_bytes, encrypted) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, encrypted)
        .map_err(|e| EncryptionError::Decryption(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| EncryptionError::Decryption(format!("Invalid UTF-8: {}", e)))
}

/// Recursively encrypt sensitive fields in a JSON value
pub fn encrypt_config(value: &Value, master_key: &[u8]) -> Result<Value, EncryptionError> {
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);

    encrypt_value_recursive(value, &cipher)
}

fn encrypt_value_recursive(value: &Value, cipher: &Aes256Gcm) -> Result<Value, EncryptionError> {
    match value {
        Value::Object(map) => {
            let mut encrypted = serde_json::Map::new();
            for (key, val) in map.iter() {
                if is_sensitive_field(key) && val.is_string() {
                    // Encrypt sensitive string fields
                    if let Some(s) = val.as_str() {
                        // Check if already encrypted
                        if !s.starts_with(ENCRYPTED_PREFIX) {
                            let encrypted_val = encrypt_value(s, cipher)?;
                            encrypted.insert(key.clone(), Value::String(encrypted_val));
                        } else {
                            encrypted.insert(key.clone(), val.clone());
                        }
                    }
                } else {
                    // Recursively process non-sensitive fields
                    encrypted.insert(key.clone(), encrypt_value_recursive(val, cipher)?);
                }
            }
            Ok(Value::Object(encrypted))
        }
        Value::Array(arr) => {
            let encrypted: Result<Vec<Value>, _> = arr
                .iter()
                .map(|v| encrypt_value_recursive(v, cipher))
                .collect();
            Ok(Value::Array(encrypted?))
        }
        // Primitive values are returned as-is
        _ => Ok(value.clone()),
    }
}

/// Recursively decrypt sensitive fields in a JSON value
pub fn decrypt_config(value: &Value, master_key: &[u8]) -> Result<Value, EncryptionError> {
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);

    decrypt_value_recursive(value, &cipher)
}

fn decrypt_value_recursive(value: &Value, cipher: &Aes256Gcm) -> Result<Value, EncryptionError> {
    match value {
        Value::Object(map) => {
            let mut decrypted = serde_json::Map::new();
            for (key, val) in map.iter() {
                if is_sensitive_field(key) && val.is_string() {
                    // Decrypt sensitive string fields
                    if let Some(s) = val.as_str() {
                        if s.starts_with(ENCRYPTED_PREFIX) {
                            let decrypted_val = decrypt_value(s, cipher)?;
                            decrypted.insert(key.clone(), Value::String(decrypted_val));
                        } else {
                            // Not encrypted, keep as-is (for migration)
                            decrypted.insert(key.clone(), val.clone());
                        }
                    }
                } else {
                    // Recursively process non-sensitive fields
                    decrypted.insert(key.clone(), decrypt_value_recursive(val, cipher)?);
                }
            }
            Ok(Value::Object(decrypted))
        }
        Value::Array(arr) => {
            let decrypted: Result<Vec<Value>, _> = arr
                .iter()
                .map(|v| decrypt_value_recursive(v, cipher))
                .collect();
            Ok(Value::Array(decrypted?))
        }
        // Primitive values are returned as-is
        _ => Ok(value.clone()),
    }
}

/// Check if a config contains any encrypted values
pub fn has_encrypted_values(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            for (_, val) in map.iter() {
                if let Some(s) = val.as_str() {
                    if s.starts_with(ENCRYPTED_PREFIX) {
                        return true;
                    }
                }
                if has_encrypted_values(val) {
                    return true;
                }
            }
            false
        }
        Value::Array(arr) => arr.iter().any(has_encrypted_values),
        _ => false,
    }
}

/// Check if a config contains any unencrypted sensitive values
pub fn has_unencrypted_secrets(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter() {
                if is_sensitive_field(key) && val.is_string() {
                    if let Some(s) = val.as_str() {
                        if !s.starts_with(ENCRYPTED_PREFIX) && !s.is_empty() {
                            return true;
                        }
                    }
                }
                if has_unencrypted_secrets(val) {
                    return true;
                }
            }
            false
        }
        Value::Array(arr) => arr.iter().any(has_unencrypted_secrets),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_value() {
        let key = generate_master_key();
        let key_ref = Key::<Aes256Gcm>::from_slice(&key);
        let cipher = Aes256Gcm::new(key_ref);

        let plaintext = "secret-api-key-12345";
        let encrypted = encrypt_value(plaintext, &cipher).unwrap();

        // Verify it has the prefix
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));

        // Decrypt and verify
        let decrypted = decrypt_value(&encrypted, &cipher).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_config() {
        let key = generate_master_key();

        let config = serde_json::json!({
            "gateway": {
                "port": 8080,
                "auth": {
                    "token": "secret-token-123",
                    "enabled": true
                }
            },
            "channels": {
                "telegram": {
                    "token": "bot123:secret",
                    "enabled": true
                }
            }
        });

        // Encrypt
        let encrypted = encrypt_config(&config, &key).unwrap();

        // Verify sensitive fields are encrypted
        let token = encrypted["gateway"]["auth"]["token"].as_str().unwrap();
        assert!(token.starts_with(ENCRYPTED_PREFIX));

        let tg_token = encrypted["channels"]["telegram"]["token"].as_str().unwrap();
        assert!(tg_token.starts_with(ENCRYPTED_PREFIX));

        // Non-sensitive fields should not be encrypted
        let port = encrypted["gateway"]["port"].as_u64().unwrap();
        assert_eq!(port, 8080);

        // Decrypt and verify
        let decrypted = decrypt_config(&encrypted, &key).unwrap();
        assert_eq!(decrypted["gateway"]["auth"]["token"], "secret-token-123");
        assert_eq!(decrypted["channels"]["telegram"]["token"], "bot123:secret");
    }

    #[test]
    fn test_has_encrypted_values() {
        let key = generate_master_key();

        let config = serde_json::json!({
            "gateway": {
                "auth": {
                    "token": "secret-token"
                }
            }
        });

        assert!(!has_encrypted_values(&config));

        let encrypted = encrypt_config(&config, &key).unwrap();
        assert!(has_encrypted_values(&encrypted));
    }

    #[test]
    fn test_has_unencrypted_secrets() {
        let config = serde_json::json!({
            "gateway": {
                "auth": {
                    "token": "secret-token"
                }
            }
        });

        assert!(has_unencrypted_secrets(&config));

        let empty_config = serde_json::json!({
            "gateway": {
                "port": 8080
            }
        });

        assert!(!has_unencrypted_secrets(&empty_config));
    }
}
