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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_config_default() {
        let config = VoiceConfig::default();
        assert!(!config.enabled);
        assert!(config.account_sid.is_empty());
        assert!(config.auth_token.is_empty());
        assert!(config.phone_number.is_empty());
        assert!(config.webhook_url.is_empty());
        assert!(config.verify_signatures);
        assert_eq!(config.tts_voice, "Polly.Joanna");
        assert_eq!(config.tts_language, "en-US");
        assert!(config.barge_in_enabled);
        assert!(!config.record_calls);
        assert_eq!(config.max_call_duration, 3600);
    }

    #[test]
    fn test_voice_config_validate_disabled() {
        let config = VoiceConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_voice_config_validate_missing_account_sid() {
        let config = VoiceConfig {
            enabled: true,
            auth_token: "token".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            ..VoiceConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("account SID"));
    }

    #[test]
    fn test_voice_config_validate_missing_auth_token() {
        let config = VoiceConfig {
            enabled: true,
            account_sid: "AC123".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            ..VoiceConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("auth token"));
    }

    #[test]
    fn test_voice_config_validate_missing_phone_number() {
        let config = VoiceConfig {
            enabled: true,
            account_sid: "AC123".to_string(),
            auth_token: "token".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            ..VoiceConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("phone number"));
    }

    #[test]
    fn test_voice_config_validate_missing_webhook_url() {
        let config = VoiceConfig {
            enabled: true,
            account_sid: "AC123".to_string(),
            auth_token: "token".to_string(),
            phone_number: "+1234567890".to_string(),
            ..VoiceConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Webhook URL"));
    }

    #[test]
    fn test_voice_config_validate_valid() {
        let config = VoiceConfig {
            enabled: true,
            account_sid: "AC123".to_string(),
            auth_token: "token".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            ..VoiceConfig::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_voice_config_is_configured() {
        let config = VoiceConfig::default();
        assert!(!config.is_configured());

        let config = VoiceConfig {
            enabled: true,
            account_sid: "AC123".to_string(),
            auth_token: "token".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            ..VoiceConfig::default()
        };
        assert!(config.is_configured());

        let config = VoiceConfig {
            enabled: true,
            account_sid: "AC123".to_string(),
            auth_token: "token".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: "".to_string(),
            ..VoiceConfig::default()
        };
        assert!(!config.is_configured());
    }
}
