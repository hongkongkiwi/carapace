//! TLS configuration and management
//!
//! Provides HTTPS support with certificate loading and configuration.
//! By default, HTTPS is required. Use `--insecure-http` flag for HTTP-only mode.

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::fmt;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur during TLS configuration
#[derive(Error, Debug)]
pub enum TlsError {
    #[error("Certificate file not found: {0}")]
    CertificateNotFound(PathBuf),

    #[error("Private key file not found: {0}")]
    KeyNotFound(PathBuf),

    #[error("Failed to read certificate file: {0}")]
    CertificateReadError(#[source] std::io::Error),

    #[error("Failed to read private key file: {0}")]
    KeyReadError(#[source] std::io::Error),

    #[error("No valid certificates found in file: {0}")]
    NoCertificatesFound(PathBuf),

    #[error("No valid private key found in file: {0}")]
    NoPrivateKeyFound(PathBuf),

    #[error("Invalid certificate format: {0}")]
    InvalidCertificate(String),

    #[error("Invalid private key format: {0}")]
    InvalidPrivateKey(String),

    #[error("Failed to create TLS config: {0}")]
    ConfigCreationError(String),
}

/// TLS configuration for the server
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to the certificate file (PEM format)
    pub cert_path: PathBuf,
    /// Path to the private key file (PEM format)
    pub key_path: PathBuf,
    /// Whether TLS is enabled
    pub enabled: bool,
    /// Whether to allow insecure HTTP (development mode)
    pub allow_insecure_http: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            enabled: true,
            allow_insecure_http: false,
        }
    }
}

impl TlsConfig {
    /// Create a new TLS configuration with custom paths
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
            enabled: true,
            allow_insecure_http: false,
        }
    }

    /// Create an insecure HTTP configuration (no TLS)
    pub fn insecure() -> Self {
        Self {
            cert_path: PathBuf::new(),
            key_path: PathBuf::new(),
            enabled: false,
            allow_insecure_http: true,
        }
    }

    /// Check if the certificate and key files exist
    pub fn certificates_exist(&self) -> bool {
        self.cert_path.exists() && self.key_path.exists()
    }

    /// Get a human-readable description of the TLS mode
    pub fn mode_description(&self) -> &'static str {
        if self.allow_insecure_http {
            "INSECURE HTTP (development mode)"
        } else if self.enabled {
            "HTTPS (TLS encrypted)"
        } else {
            "HTTP (no encryption)"
        }
    }
}

/// Default certificate path: ~/.moltbot/tls/cert.pem
fn default_cert_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".moltbot/tls/cert.pem")
}

/// Default key path: ~/.moltbot/tls/key.pem
fn default_key_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".moltbot/tls/key.pem")
}

/// Load certificates from a PEM file
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let file = fs::File::open(path).map_err(|e| TlsError::CertificateReadError(e))?;
    let mut reader = BufReader::new(file);

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .filter_map(|cert| cert.ok())
        .collect();

    if certs.is_empty() {
        return Err(TlsError::NoCertificatesFound(path.to_path_buf()));
    }

    Ok(certs)
}

/// Load a private key from a PEM file
fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let file = fs::File::open(path).map_err(|e| TlsError::KeyReadError(e))?;
    let mut reader = BufReader::new(file);

    // Try PKCS8 first
    if let Some(key) = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .filter_map(|key| key.ok())
        .next()
    {
        return Ok(
            PrivateKeyDer::try_from(key).map_err(|e| TlsError::InvalidPrivateKey(e.to_string()))?
        );
    }

    // Reset reader and try RSA
    let file = fs::File::open(path).map_err(|e| TlsError::KeyReadError(e))?;
    let mut reader = BufReader::new(file);
    if let Some(key) = rustls_pemfile::rsa_private_keys(&mut reader)
        .filter_map(|key| key.ok())
        .next()
    {
        return Ok(
            PrivateKeyDer::try_from(key).map_err(|e| TlsError::InvalidPrivateKey(e.to_string()))?
        );
    }

    Err(TlsError::NoPrivateKeyFound(path.to_path_buf()))
}

/// Create a TLS server configuration
pub fn create_tls_config(config: &TlsConfig) -> Result<ServerConfig, TlsError> {
    if !config.enabled {
        return Err(TlsError::ConfigCreationError(
            "TLS is not enabled".to_string(),
        ));
    }

    if !config.certificates_exist() {
        return Err(TlsError::CertificateNotFound(config.cert_path.clone()));
    }

    let certs = load_certs(&config.cert_path)?;
    let key = load_key(&config.key_path)?;

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TlsError::ConfigCreationError(e.to_string()))?;

    info!(
        "TLS configuration loaded successfully (cert: {}, key: {})",
        config.cert_path.display(),
        config.key_path.display()
    );

    Ok(tls_config)
}

/// Check TLS configuration and provide helpful warnings/errors
pub fn validate_tls_config(config: &TlsConfig) -> Result<(), TlsError> {
    if config.allow_insecure_http {
        warn!("⚠️  INSECURE HTTP MODE ENABLED - DO NOT USE IN PRODUCTION");
        warn!("   Traffic will be unencrypted. Use only for local development.");
        return Ok(());
    }

    if !config.enabled {
        warn!("⚠️  TLS is disabled - connections will be unencrypted");
        return Ok(());
    }

    if !config.cert_path.exists() {
        return Err(TlsError::CertificateNotFound(config.cert_path.clone()));
    }

    if !config.key_path.exists() {
        return Err(TlsError::KeyNotFound(config.key_path.clone()));
    }

    // Try loading to validate format
    let _ = load_certs(&config.cert_path)?;
    let _ = load_key(&config.key_path)?;

    info!(
        "✓ TLS certificates validated (cert: {}, key: {})",
        config.cert_path.display(),
        config.key_path.display()
    );

    Ok(())
}

/// Generate a self-signed certificate for development using openssl
///
/// WARNING: This should only be used for development/testing!
pub fn generate_self_signed_cert(cert_path: &Path, key_path: &Path) -> Result<(), TlsError> {
    use std::fs;
    use std::process::Command;

    info!("Generating self-signed certificate for development...");

    // Create directory if needed
    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent).map_err(|e| TlsError::CertificateReadError(e))?;
    }

    // Use openssl command to generate self-signed certificate
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-nodes",
            "-days",
            "365",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key_path.to_str().unwrap(),
            "-out",
            cert_path.to_str().unwrap(),
            "-subj",
            "/CN=localhost/O=Carapace Development",
            "-addext",
            "subjectAltName=DNS:localhost,IP:127.0.0.1",
        ])
        .output()
        .map_err(|e| TlsError::ConfigCreationError(format!("Failed to run openssl: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TlsError::ConfigCreationError(format!(
            "openssl failed: {}",
            stderr
        )));
    }

    // Set restrictive permissions on key file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(key_path).map_err(|e| TlsError::KeyReadError(e))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600); // Owner read/write only
        fs::set_permissions(key_path, permissions).map_err(|e| TlsError::KeyReadError(e))?;
    }

    info!("✓ Self-signed certificate generated:");
    info!("  Certificate: {}", cert_path.display());
    info!("  Key: {}", key_path.display());
    warn!("⚠️  This is a self-signed certificate - browsers will show warnings");

    Ok(())
}

/// Display TLS setup instructions
pub fn print_tls_setup_instructions() {
    eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
    eprintln!("║           TLS Certificate Setup Required                       ║");
    eprintln!("╠════════════════════════════════════════════════════════════════╣");
    eprintln!("║  HTTPS is enabled by default for security.                     ║");
    eprintln!("║  You have three options:                                       ║");
    eprintln!("║                                                                ║");
    eprintln!("║  1. Use existing certificates:                                 ║");
    eprintln!("║     Place your cert.pem and key.pem in:                        ║");
    eprintln!("║     ~/.moltbot/tls/                                            ║");
    eprintln!("║                                                                ║");
    eprintln!("║  2. Generate self-signed certificate:                          ║");
    eprintln!("║     openssl req -x509 -nodes -days 365 -newkey rsa:2048       ║");
    eprintln!("║       -keyout ~/.moltbot/tls/key.pem                          ║");
    eprintln!("║       -out ~/.moltbot/tls/cert.pem                            ║");
    eprintln!("║       -subj \"/CN=localhost\"                                  ║");
    eprintln!("║                                                                ║");
    eprintln!("║  3. Run in insecure HTTP mode (development only):              ║");
    eprintln!("║     carapace start --insecure-http                             ║");
    eprintln!("║     ⚠️  WARNING: Only use this for local development!          ║");
    eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert!(config.enabled);
        assert!(!config.allow_insecure_http);
        assert!(!config.cert_path.as_os_str().is_empty());
        assert!(!config.key_path.as_os_str().is_empty());
    }

    #[test]
    fn test_tls_config_insecure() {
        let config = TlsConfig::insecure();
        assert!(!config.enabled);
        assert!(config.allow_insecure_http);
        assert!(config.cert_path.as_os_str().is_empty());
        assert!(config.key_path.as_os_str().is_empty());
    }

    #[test]
    fn test_tls_config_mode_description() {
        assert_eq!(
            TlsConfig::default().mode_description(),
            "HTTPS (TLS encrypted)"
        );
        assert_eq!(
            TlsConfig::insecure().mode_description(),
            "INSECURE HTTP (development mode)"
        );
    }

    #[test]
    fn test_validate_tls_config_insecure() {
        let config = TlsConfig::insecure();
        // Should not error, just warn
        assert!(validate_tls_config(&config).is_ok());
    }

    #[test]
    fn test_validate_tls_config_missing_certs() {
        let config = TlsConfig::default();
        // Default paths won't exist in test
        assert!(validate_tls_config(&config).is_err());
    }
}
