//! Voice Configuration
//!
//! Configuration for voice calls via Twilio

use serde::{Deserialize, Serialize};

/// Voice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Enable voice calls
    pub enabled: bool,
    /// Twilio account SID
    pub account_sid: String,
    /// Twilio auth token
    pub auth_token: String,
    /// Twilio phone number
    pub phone_number: String,
    /// Webhook URL for Twilio (public URL)
    pub webhook_url: String,
    /// Verify Twilio webhook signatures
    pub verify_signatures: bool,
    /// Default TTS voice
    pub tts_voice: String,
    /// TTS language
    pub tts_language: String,
    /// Enable barge-in (user can interrupt)
    pub barge_in_enabled: bool,
    /// Record calls
    pub record_calls: bool,
    /// Maximum call duration in seconds
    pub max_call_duration: u32,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            account_sid: String::new(),
            auth_token: String::new(),
            phone_number: String::new(),
            webhook_url: String::new(),
            verify_signatures: true,
            tts_voice: "Polly.Joanna".to_string(),
            tts_language: "en-US".to_string(),
            barge_in_enabled: true,
            record_calls: false,
            max_call_duration: 3600,
        }
    }
}

impl VoiceConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.account_sid.is_empty() {
            return Err("Twilio account SID is required".to_string());
        }

        if self.auth_token.is_empty() {
            return Err("Twilio auth token is required".to_string());
        }

        if self.phone_number.is_empty() {
            return Err("Twilio phone number is required".to_string());
        }

        if self.webhook_url.is_empty() {
            return Err("Webhook URL is required".to_string());
        }

        Ok(())
    }

    /// Check if voice is properly configured
    pub fn is_configured(&self) -> bool {
        self.enabled
            && !self.account_sid.is_empty()
            && !self.auth_token.is_empty()
            && !self.phone_number.is_empty()
            && !self.webhook_url.is_empty()
    }
}
