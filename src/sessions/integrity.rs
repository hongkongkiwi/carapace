//! Session integrity verification using HMAC-SHA256.
//!
//! Provides tamper detection for session files by computing and verifying
//! HMAC-SHA256 signatures stored in sidecar `.hmac` files.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Domain separation tag for HMAC key derivation.
const KEY_DERIVATION_TAG: &[u8] = b"session-integrity-hmac-v1";

/// HMAC sidecar file extension.
const HMAC_EXTENSION: &str = "hmac";

/// Action to take when integrity verification fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityAction {
    /// Log a warning and continue loading the session (auto-migrates).
    #[default]
    Warn,
    /// Reject the session and refuse to load it.
    Reject,
}

/// Session integrity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    /// Master switch — when `false`, HMAC operations are skipped.
    #[serde(default)]
    pub enabled: bool,
    /// Action on integrity failure.
    #[serde(default)]
    pub action: IntegrityAction,
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            action: IntegrityAction::Warn,
        }
    }
}

/// Integrity verification errors.
#[derive(Debug, thiserror::Error)]
pub enum IntegrityError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("HMAC verification failed for {file}: {reason}")]
    VerificationFailed { file: String, reason: String },
    #[error("Session rejected due to integrity violation: {file}")]
    Rejected { file: String },
}

/// Derive an HMAC key from a server secret using SHA-256.
///
/// Uses domain separation: `SHA-256(server_secret || "session-integrity-hmac-v1")`
pub fn derive_hmac_key(server_secret: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(server_secret);
    hasher.update(KEY_DERIVATION_TAG);
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Compute HMAC-SHA256 over the given data.
pub fn compute_hmac(key: &[u8; 32], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Verify HMAC-SHA256 over the given data.
pub fn verify_hmac(key: &[u8; 32], data: &[u8], expected: &[u8]) -> bool {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
    mac.update(data);
    mac.verify_slice(expected).is_ok()
}

/// Get the HMAC sidecar file path for a given data file.
fn hmac_path(file_path: &Path) -> PathBuf {
    let mut path = file_path.as_os_str().to_owned();
    path.push(".");
    path.push(HMAC_EXTENSION);
    PathBuf::from(path)
}

/// Write an HMAC sidecar file for the given data file.
///
/// Reads the file contents, computes the HMAC, and writes it to `{file_path}.hmac`.
pub fn write_hmac_file(key: &[u8; 32], file_path: &Path) -> Result<(), io::Error> {
    let data = fs::read(file_path)?;
    let hmac = compute_hmac(key, &data);
    let sidecar = hmac_path(file_path);
    fs::write(&sidecar, hex::encode(hmac))?;
    Ok(())
}

/// Verify the HMAC sidecar file for the given data file.
///
/// # Behavior
///
/// - Missing `.hmac` file with `action: Warn` → logs warning, writes HMAC (auto-migration).
/// - Missing `.hmac` file with `action: Reject` → returns error.
/// - HMAC mismatch with `action: Warn` → logs warning.
/// - HMAC mismatch with `action: Reject` → returns error.
pub fn verify_hmac_file(
    key: &[u8; 32],
    file_path: &Path,
    config: &IntegrityConfig,
) -> Result<(), IntegrityError> {
    if !config.enabled {
        return Ok(());
    }

    // Read the data file
    let data = fs::read(file_path)?;

    let sidecar = hmac_path(file_path);
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unknown>");

    match fs::read_to_string(&sidecar) {
        Ok(stored_hex) => {
            let stored_hmac =
                hex::decode(stored_hex.trim()).map_err(|e| IntegrityError::VerificationFailed {
                    file: file_name.to_string(),
                    reason: format!("invalid hex in HMAC sidecar: {e}"),
                })?;

            if !verify_hmac(key, &data, &stored_hmac) {
                let msg = format!("HMAC verification failed for {file_name} — possible tampering");

                match config.action {
                    IntegrityAction::Warn => {
                        tracing::warn!("{}", msg);
                        Ok(())
                    }
                    IntegrityAction::Reject => Err(IntegrityError::Rejected {
                        file: file_name.to_string(),
                    }),
                }
            } else {
                tracing::debug!(file = %file_name, "session integrity verification passed");
                Ok(())
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            match config.action {
                IntegrityAction::Warn => {
                    tracing::warn!(
                        file = %file_name,
                        "no HMAC sidecar found — auto-migrating (writing HMAC)"
                    );
                    // Auto-migrate: write the HMAC sidecar
                    let hmac = compute_hmac(key, &data);
                    let _ = fs::write(&sidecar, hex::encode(hmac));
                    Ok(())
                }
                IntegrityAction::Reject => Err(IntegrityError::Rejected {
                    file: file_name.to_string(),
                }),
            }
        }
        Err(e) => Err(IntegrityError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // ==================== Key Derivation ====================

    #[test]
    fn test_derive_hmac_key_deterministic() {
        let key1 = derive_hmac_key(b"server-secret-1");
        let key2 = derive_hmac_key(b"server-secret-1");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_hmac_key_different_secrets() {
        let key1 = derive_hmac_key(b"secret-a");
        let key2 = derive_hmac_key(b"secret-b");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_hmac_key_length() {
        let key = derive_hmac_key(b"test");
        assert_eq!(key.len(), 32);
    }

    // ==================== HMAC Roundtrip ====================

    #[test]
    fn test_compute_verify_hmac_roundtrip() {
        let key = derive_hmac_key(b"test-secret");
        let data = b"session data here";
        let hmac = compute_hmac(&key, data);
        assert!(verify_hmac(&key, data, &hmac));
    }

    #[test]
    fn test_verify_hmac_wrong_data() {
        let key = derive_hmac_key(b"test-secret");
        let data = b"original data";
        let hmac = compute_hmac(&key, data);
        assert!(!verify_hmac(&key, b"tampered data", &hmac));
    }

    #[test]
    fn test_verify_hmac_wrong_key() {
        let key1 = derive_hmac_key(b"secret-1");
        let key2 = derive_hmac_key(b"secret-2");
        let data = b"some data";
        let hmac = compute_hmac(&key1, data);
        assert!(!verify_hmac(&key2, data, &hmac));
    }

    #[test]
    fn test_verify_hmac_wrong_mac() {
        let key = derive_hmac_key(b"secret");
        let data = b"data";
        let wrong_mac = vec![0u8; 32];
        assert!(!verify_hmac(&key, data, &wrong_mac));
    }

    // ==================== Sidecar Files ====================

    #[test]
    fn test_write_hmac_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("meta.json");
        fs::write(&file_path, r#"{"id":"test"}"#).unwrap();

        let key = derive_hmac_key(b"server-secret");
        write_hmac_file(&key, &file_path).unwrap();

        let sidecar = dir.path().join("meta.json.hmac");
        assert!(sidecar.exists(), "HMAC sidecar should exist");

        let hmac_hex = fs::read_to_string(&sidecar).unwrap();
        assert!(!hmac_hex.is_empty());
    }

    #[test]
    fn test_verify_hmac_file_success() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("history.jsonl");
        fs::write(&file_path, "line1\nline2\n").unwrap();

        let key = derive_hmac_key(b"test-secret");
        write_hmac_file(&key, &file_path).unwrap();

        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Reject,
        };

        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_hmac_file_tampered() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("meta.json");
        fs::write(&file_path, r#"{"id":"original"}"#).unwrap();

        let key = derive_hmac_key(b"test-secret");
        write_hmac_file(&key, &file_path).unwrap();

        // Tamper with the file
        fs::write(&file_path, r#"{"id":"tampered"}"#).unwrap();

        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Reject,
        };

        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_hmac_file_tampered_warn_mode() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("meta.json");
        fs::write(&file_path, r#"{"id":"original"}"#).unwrap();

        let key = derive_hmac_key(b"test-secret");
        write_hmac_file(&key, &file_path).unwrap();

        // Tamper
        fs::write(&file_path, r#"{"id":"tampered"}"#).unwrap();

        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Warn,
        };

        // Warn mode should still return Ok
        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_ok());
    }

    // ==================== Missing HMAC Sidecar ====================

    #[test]
    fn test_missing_hmac_warn_auto_migrates() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("meta.json");
        fs::write(&file_path, r#"{"id":"test"}"#).unwrap();

        let key = derive_hmac_key(b"test-secret");

        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Warn,
        };

        // No HMAC file exists yet
        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_ok());

        // Auto-migration should have created the HMAC file
        let sidecar = dir.path().join("meta.json.hmac");
        assert!(
            sidecar.exists(),
            "auto-migration should create HMAC sidecar"
        );

        // Now verification should pass
        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_hmac_reject_fails() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("meta.json");
        fs::write(&file_path, r#"{"id":"test"}"#).unwrap();

        let key = derive_hmac_key(b"test-secret");

        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Reject,
        };

        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_err());
    }

    // ==================== Disabled Config ====================

    #[test]
    fn test_disabled_config_skips_verification() {
        let key = derive_hmac_key(b"secret");
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("meta.json");
        fs::write(&file_path, "data").unwrap();

        let config = IntegrityConfig {
            enabled: false,
            action: IntegrityAction::Reject,
        };

        // No HMAC sidecar, but disabled — should pass
        let result = verify_hmac_file(&key, &file_path, &config);
        assert!(result.is_ok());
    }

    // ==================== Config Serialization ====================

    #[test]
    fn test_integrity_config_default() {
        let config = IntegrityConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.action, IntegrityAction::Warn);
    }

    #[test]
    fn test_integrity_config_serde_roundtrip() {
        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Reject,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: IntegrityConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.action, IntegrityAction::Reject);
    }

    // ==================== HMAC Path ====================

    #[test]
    fn test_hmac_path() {
        let path = Path::new("/sessions/abc/meta.json");
        let sidecar = hmac_path(path);
        assert_eq!(sidecar, PathBuf::from("/sessions/abc/meta.json.hmac"));
    }

    #[test]
    fn test_hmac_path_jsonl() {
        let path = Path::new("/sessions/abc/history.jsonl");
        let sidecar = hmac_path(path);
        assert_eq!(sidecar, PathBuf::from("/sessions/abc/history.jsonl.hmac"));
    }

    // ==================== Empty Data ====================

    #[test]
    fn test_hmac_empty_data() {
        let key = derive_hmac_key(b"secret");
        let hmac = compute_hmac(&key, b"");
        assert!(verify_hmac(&key, b"", &hmac));
        assert!(!verify_hmac(&key, b"non-empty", &hmac));
    }

    // ==================== Large Data ====================

    #[test]
    fn test_hmac_large_data() {
        let key = derive_hmac_key(b"secret");
        let data = vec![0xABu8; 1024 * 1024]; // 1 MB
        let hmac = compute_hmac(&key, &data);
        assert!(verify_hmac(&key, &data, &hmac));
    }

    // ==================== Multiple Files ====================

    #[test]
    fn test_multiple_files_independent_hmacs() {
        let dir = TempDir::new().unwrap();
        let key = derive_hmac_key(b"secret");

        let file1 = dir.path().join("meta.json");
        let file2 = dir.path().join("history.jsonl");
        fs::write(&file1, "meta content").unwrap();
        fs::write(&file2, "history content").unwrap();

        write_hmac_file(&key, &file1).unwrap();
        write_hmac_file(&key, &file2).unwrap();

        let config = IntegrityConfig {
            enabled: true,
            action: IntegrityAction::Reject,
        };

        assert!(verify_hmac_file(&key, &file1, &config).is_ok());
        assert!(verify_hmac_file(&key, &file2, &config).is_ok());

        // Tamper with file1 only — file2 should still pass
        fs::write(&file1, "tampered").unwrap();
        assert!(verify_hmac_file(&key, &file1, &config).is_err());
        assert!(verify_hmac_file(&key, &file2, &config).is_ok());
    }
}
