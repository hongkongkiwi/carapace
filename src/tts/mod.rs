//! Text-to-Speech (TTS) Module
//!
//! Provides TTS functionality with multiple providers including Edge TTS.

pub mod config;
pub mod providers;

pub use config::*;
pub use providers::*;

use thiserror::Error;

/// TTS errors
#[derive(Error, Debug)]
pub enum TtsError {
    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unsupported voice: {0}")]
    UnsupportedVoice(String),
}

/// Result type for TTS operations
pub type Result<T> = std::result::Result<T, TtsError>;
