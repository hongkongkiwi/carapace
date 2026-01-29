//! Voice Calls Module
//!
//! Provides voice call functionality via Twilio integration.
//! Supports incoming/outgoing calls, streaming TwiML, and TTS playback.

pub mod call;
pub mod config;
pub mod twilio;

pub use call::*;
pub use config::*;
pub use twilio::*;

use thiserror::Error;

/// Voice module errors
#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("Twilio error: {0}")]
    TwilioError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Call not found: {0}")]
    CallNotFound(String),

    #[error("Invalid webhook signature")]
    InvalidSignature,

    #[error("TTS error: {0}")]
    TtsError(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for voice operations
pub type Result<T> = std::result::Result<T, VoiceError>;
