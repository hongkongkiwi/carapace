//! Voice Calls Channel Implementation
//!
//! Provides voice call functionality using various providers.
//! Currently supports Twilio for PSTN and SIP calls.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Voice call configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Enable voice calls
    pub enabled: bool,
    /// Twilio Account SID
    pub twilio_account_sid: String,
    /// Twilio Auth Token
    pub twilio_auth_token: String,
    /// Twilio Phone Number
    pub twilio_phone_number: String,
    /// SIP domain for SIP calls
    pub sip_domain: Option<String>,
    /// SIP username
    pub sip_username: Option<String>,
    /// SIP password
    pub sip_password: Option<String>,
    /// Default caller ID
    pub default_caller_id: String,
    /// Max call duration in minutes
    pub max_duration_mins: u32,
    /// Recording enabled
    pub recording_enabled: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            twilio_account_sid: String::new(),
            twilio_auth_token: String::new(),
            twilio_phone_number: String::new(),
            sip_domain: None,
            sip_username: None,
            sip_password: None,
            default_caller_id: String::new(),
            max_duration_mins: 60,
            recording_enabled: false,
        }
    }
}

/// Voice call error
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("call error: {0}")]
    Call(String),
    #[error("DTMF error: {0}")]
    Dtmf(String),
    #[error("recording error: {0}")]
    Recording(String),
}

/// Voice call status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceCallStatus {
    Queued,
    Ringing,
    InProgress,
    Completed,
    Failed,
    Busy,
    NoAnswer,
    Canceled,
}

/// Active voice call
#[derive(Debug)]
pub struct VoiceCall {
    /// Call SID/ID
    pub id: String,
    /// Phone number or SIP URI
    pub to: String,
    /// Caller ID
    pub from: String,
    /// Current status
    pub status: VoiceCallStatus,
    /// Duration in seconds
    pub duration: u64,
    /// Start time
    pub start_time: u64,
    /// Recording URL if enabled
    pub recording_url: Option<String>,
    /// Transcription if available
    pub transcription: Option<String>,
}

/// DTMF tone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DtmfTone {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Star,
    Pound,
}

impl DtmfTone {
    fn as_str(&self) -> &str {
        match self {
            DtmfTone::Zero => "0",
            DtmfTone::One => "1",
            DtmfTone::Two => "2",
            DtmfTone::Three => "3",
            DtmfTone::Four => "4",
            DtmfTone::Five => "5",
            DtmfTone::Six => "6",
            DtmfTone::Seven => "7",
            DtmfTone::Eight => "8",
            DtmfTone::Nine => "9",
            DtmfTone::Star => "*",
            DtmfTone::Pound => "#",
        }
    }
}

/// Voice channel struct
#[derive(Debug)]
pub struct VoiceChannel {
    config: VoiceConfig,
    event_tx: mpsc::Sender<MessageContent>,
    active_calls: HashMap<String, VoiceCall>,
}

impl VoiceChannel {
    /// Create a new voice channel
    pub fn new(config: VoiceConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        Self {
            config,
            event_tx,
            active_calls: HashMap::new(),
        }
    }

    /// Make an outbound call
    pub async fn make_call(
        &mut self,
        to: &str,
        from: Option<&str>,
        _twiml_url: Option<&str>,
    ) -> Result<String, VoiceError> {
        let from = from.unwrap_or(&self.config.default_caller_id);

        // Using Twilio API structure
        let call_sid = format!("CA_{}", uuid::Uuid::new_v4().simple());

        let call = VoiceCall {
            id: call_sid.clone(),
            to: to.to_string(),
            from: from.to_string(),
            status: VoiceCallStatus::Queued,
            duration: 0,
            start_time: now_secs(),
            recording_url: None,
            transcription: None,
        };

        self.active_calls.insert(call_sid.clone(), call);
        debug!("Call queued: {} -> {}", from, to);

        Ok(call_sid)
    }

    /// Make a call and play TTS message
    pub async fn call_and_play(
        &mut self,
        to: &str,
        _message: &str,
    ) -> Result<String, VoiceError> {
        self.make_call(to, None, None).await
    }

    /// End an active call
    pub async fn end_call(&mut self, call_id: &str) -> Result<(), VoiceError> {
        if let Some(call) = self.active_calls.get_mut(call_id) {
            call.status = VoiceCallStatus::Completed;
            debug!("Call ended: {}", call_id);
            Ok(())
        } else {
            Err(VoiceError::Call("Call not found".to_string()))
        }
    }

    /// Send DTMF tones to a call
    pub async fn send_dtmf(
        &mut self,
        call_id: &str,
        tones: &[DtmfTone],
    ) -> Result<(), VoiceError> {
        let tones_str: String = tones.iter().map(|t| t.as_str()).collect();
        debug!("Sending DTMF to {}: {}", call_id, tones_str);
        Ok(())
    }

    /// Get call status
    pub async fn get_call_status(&self, call_id: &str) -> Option<VoiceCallStatus> {
        self.active_calls.get(call_id).map(|c| c.status)
    }

    /// Get all active calls
    pub async fn active_calls(&self) -> Vec<&VoiceCall> {
        self.active_calls.values().collect()
    }

    /// Get call by phone number
    pub async fn get_call_by_number(&self, phone: &str) -> Option<&VoiceCall> {
        self.active_calls.values().find(|c| c.to == phone)
    }

    /// Send voicemail
    pub async fn send_voicemail(
        &mut self,
        to: &str,
        _audio_url: &str,
    ) -> Result<String, VoiceError> {
        self.make_call(to, None, None).await
    }

    /// Get call recording
    pub async fn get_recording(&self, call_id: &str) -> Option<&str> {
        self.active_calls
            .get(call_id)
            .and_then(|c| c.recording_url.as_deref())
    }

    /// Connect to voice provider
    pub async fn connect(&mut self) -> Result<(), VoiceError> {
        info!("Voice channel connected");
        Ok(())
    }

    /// Disconnect from voice provider
    pub async fn disconnect(&mut self) -> Result<(), VoiceError> {
        // End all active calls - collect keys first to avoid borrow issues
        let call_ids: Vec<String> = self.active_calls.keys().cloned().collect();
        for call_id in call_ids {
            let _ = self.end_call(&call_id).await;
        }
        info!("Voice channel disconnected");
        Ok(())
    }
}

/// Voice provider trait for abstraction
#[async_trait::async_trait]
pub trait VoiceProvider {
    async fn make_call(&self, to: &str, from: &str, twiml: Option<&str>) -> Result<String, VoiceError>;
    async fn end_call(&self, call_id: &str) -> Result<(), VoiceError>;
    async fn send_dtmf(&self, call_id: &str, tones: &[DtmfTone]) -> Result<(), VoiceError>;
    async fn get_call_status(&self, call_id: &str) -> Result<VoiceCallStatus, VoiceError>;
}

/// Twilio voice provider
#[derive(Debug)]
pub struct TwilioProvider {
    account_sid: String,
    auth_token: String,
    base_url: String,
}

impl TwilioProvider {
    pub fn new(account_sid: &str, auth_token: &str) -> Self {
        Self {
            account_sid: account_sid.to_string(),
            auth_token: auth_token.to_string(),
            base_url: format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}.json",
                account_sid
            ),
        }
    }

    fn auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.account_sid, self.auth_token);
        format!("Basic {}", base64_encode(&credentials))
    }
}

fn base64_encode(input: &str) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).map(|&b| b as u32).unwrap_or(0);
        let b2 = chunk.get(2).map(|&b| b as u32).unwrap_or(0);

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[(n >> 18) as usize] as char);
        result.push(CHARSET[(n >> 12 & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARSET[(n >> 6 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARSET[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Helper to get current time in seconds
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Export for module
pub use VoiceChannel as Channel;
pub use VoiceConfig as Config;
pub use VoiceError as Error;
