//! Credential storage module
//!
//! Platform-specific secure credential storage:
//! - macOS: Keychain
//! - Linux: Secret Service
//! - Windows: Credential Manager

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;
