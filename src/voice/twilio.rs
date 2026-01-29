//! Twilio Integration
//!
//! Twilio webhook handlers and API client for voice calls

use super::{call::*, config::VoiceConfig, VoiceError, Result};
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, Response},
    body::Body,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
use std::collections::HashMap;

/// Twilio webhook payload for voice calls
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TwilioWebhook {
    /// Call SID
    pub call_sid: String,
    /// Call status
    pub call_status: String,
    /// From number
    pub from: String,
    /// To number
    pub to: String,
    /// Direction (inbound/outbound)
    pub direction: String,
    /// Call duration (on completed)
    #[serde(default)]
    pub call_duration: Option<String>,
    /// Recording URL (if recorded)
    #[serde(default)]
    pub recording_url: Option<String>,
    /// Speech result (from Gather)
    #[serde(default)]
    pub speech_result: Option<String>,
    /// Digits pressed (from Gather)
    #[serde(default)]
    pub digits: Option<String>,
}

/// Twilio client for API calls
#[derive(Debug, Clone)]
pub struct TwilioClient {
    config: VoiceConfig,
    http_client: reqwest::Client,
    call_manager: Arc<CallManager>,
}

impl TwilioClient {
    /// Create a new Twilio client
    pub fn new(config: VoiceConfig, call_manager: Arc<CallManager>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            call_manager,
        }
    }

    /// Get the Twilio API URL
    fn api_url(&self) -> String {
        format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Calls.json",
            self.config.account_sid
        )
    }

    /// Make an outbound call
    pub async fn make_call(&self, to: &str, webhook_url: &str) -> Result<String> {
        let call = self
            .call_manager
            .create_call(
                CallDirection::Outbound,
                self.config.phone_number.clone(),
                to.to_string(),
                self.config.barge_in_enabled,
            )
            .await;

        let params = [
            ("To", to),
            ("From", &self.config.phone_number),
            ("Url", webhook_url),
            ("StatusCallback", webhook_url),
            ("StatusCallbackEvent", "initiated ringing answered completed"),
            ("Record", if self.config.record_calls { "true" } else { "false" }),
            ("TimeLimit", &self.config.max_call_duration.to_string()),
        ];

        let response = self
            .http_client
            .post(&self.api_url())
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VoiceError::TwilioError(format!(
                "Failed to make call: {}",
                error_text
            )));
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(sid) = result.get("sid").and_then(|s| s.as_str()) {
            self.call_manager.set_twilio_sid(&call.id, sid.to_string()).await;
        }

        Ok(call.id)
    }

    /// End a call
    pub async fn end_call(&self, call_id: &str) -> Result<()> {
        let call = self
            .call_manager
            .get_call(call_id)
            .await
            .ok_or_else(|| VoiceError::CallNotFound(call_id.to_string()))?;

        if let Some(sid) = call.twilio_sid {
            let url = format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}/Calls/{}.json",
                self.config.account_sid, sid
            );

            let params = [("Status", "completed")];

            let response = self
                .http_client
                .post(&url)
                .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
                .form(&params)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(VoiceError::TwilioError(format!(
                    "Failed to end call: {}",
                    error_text
                )));
            }
        }

        self.call_manager.end_call(call_id).await;
        Ok(())
    }

    /// Verify webhook signature
    pub fn verify_signature(&self, url: &str, params: &HashMap<String, String>, signature: &str) -> bool {
        if !self.config.verify_signatures {
            return true;
        }

        // Build the string to sign: URL + sorted params
        let mut data = url.to_string();
        let mut keys: Vec<_> = params.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(value) = params.get(key) {
                data.push_str(key);
                data.push_str(value);
            }
        }

        // Compute HMAC-SHA256
        let mut mac = match Hmac::<Sha256>::new_from_slice(self.config.auth_token.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(data.as_bytes());
        let result = mac.finalize();
        let expected = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result.into_bytes());

        // Compare signatures (case-insensitive base64)
        expected.eq_ignore_ascii_case(signature)
    }

    /// Generate TwiML for streaming conversation
    pub fn generate_stream_twiml(&self, stream_url: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
    <Connect>
        <Stream url="{}">
            <Parameter name="voice" value="{}" />
            <Parameter name="language" value="{}" />
        </Stream>
    </Connect>
</Response>"#,
            stream_url, self.config.tts_voice, self.config.tts_language
        )
    }

    /// Generate TwiML for gather (speech recognition)
    pub fn generate_gather_twiml(&self, say_text: &str, webhook_url: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
    <Say voice="{}" language="{}">{}</Say>
    <Gather input="speech" action="{}" speechTimeout="auto">
        <Say>Please speak after the tone.</Say>
    </Gather>
</Response>"#,
            self.config.tts_voice, self.config.tts_language, say_text, webhook_url
        )
    }

    /// Generate TwiML for simple say
    pub fn generate_say_twiml(&self, text: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
    <Say voice="{}" language="{}">{}</Say>
</Response>"#,
            self.config.tts_voice, self.config.tts_language, text
        )
    }
}

/// Twilio webhook state for axum
#[derive(Debug, Clone)]
pub struct TwilioWebhookState {
    pub client: TwilioClient,
    pub stream_url: String,
}

/// Handle incoming voice webhook
pub async fn handle_voice_webhook(
    State(state): State<Arc<TwilioWebhookState>>,
    Form(payload): Form<TwilioWebhook>,
) -> std::result::Result<Html<String>, StatusCode> {
    // Map Twilio status to our status
    let status = match payload.call_status.as_str() {
        "queued" => CallStatus::Queued,
        "ringing" => CallStatus::Ringing,
        "in-progress" => CallStatus::InProgress,
        "completed" => CallStatus::Completed,
        "busy" => CallStatus::Busy,
        "no-answer" => CallStatus::NoAnswer,
        "failed" => CallStatus::Failed,
        "canceled" => CallStatus::Cancelled,
        _ => CallStatus::InProgress,
    };

    // Find or create call
    let call_id = if let Some(call) = state.client.call_manager.get_call_by_twilio_sid(&payload.call_sid).await {
        // Update existing call
        let _ = state.client.call_manager.update_status(&call.id, status).await;
        call.id
    } else {
        // Create new inbound call
        let direction = if payload.direction.contains("inbound") {
            CallDirection::Inbound
        } else {
            CallDirection::Outbound
        };

        let call = state.client.call_manager.create_call(
            direction,
            payload.from.clone(),
            payload.to.clone(),
            state.client.config.barge_in_enabled,
        ).await;

        state.client.call_manager.set_twilio_sid(&call.id, payload.call_sid.clone()).await;
        call.id
    };

    // Handle speech result if present
    if let Some(speech) = payload.speech_result {
        state.client.call_manager.add_transcript_entry(&call_id, "caller".to_string(), speech).await;
    }

    // Handle digits if present
    if let Some(digits) = payload.digits {
        state.client.call_manager.add_transcript_entry(&call_id, "caller".to_string(), format!("Pressed: {}", digits)).await;
    }

    // Handle recording
    if let Some(recording_url) = payload.recording_url {
        state.client.call_manager.set_recording(&call_id, recording_url).await;
    }

    // Return TwiML based on call status
    let twiml = match status {
        CallStatus::InProgress => {
            // Start streaming for conversation
            state.client.generate_stream_twiml(&state.stream_url)
        }
        _ => {
            // Simple response for other statuses
            state.client.generate_say_twiml("Thank you for calling. Goodbye.")
        }
    };

    Ok(Html(twiml))
}

/// Handle status callback webhook
pub async fn handle_status_callback(
    State(state): State<Arc<TwilioWebhookState>>,
    Form(payload): Form<TwilioWebhook>,
) -> StatusCode {
    let status = match payload.call_status.as_str() {
        "queued" => CallStatus::Queued,
        "ringing" => CallStatus::Ringing,
        "in-progress" => CallStatus::InProgress,
        "completed" => CallStatus::Completed,
        "busy" => CallStatus::Busy,
        "no-answer" => CallStatus::NoAnswer,
        "failed" => CallStatus::Failed,
        "canceled" => CallStatus::Cancelled,
        _ => return StatusCode::OK,
    };

    if let Some(call) = state.client.call_manager.get_call_by_twilio_sid(&payload.call_sid).await {
        let _ = state.client.call_manager.update_status(&call.id, status).await;

        // Handle recording URL
        if let Some(recording_url) = payload.recording_url {
            let _ = state.client.call_manager.set_recording(&call.id, recording_url).await;
        }
    }

    StatusCode::OK
}

/// Handle recording callback
pub async fn handle_recording_callback(
    State(state): State<Arc<TwilioWebhookState>>,
    Form(payload): Form<TwilioWebhook>,
) -> StatusCode {
    if let Some(recording_url) = payload.recording_url {
        if let Some(call) = state.client.call_manager.get_call_by_twilio_sid(&payload.call_sid).await {
            let _ = state.client.call_manager.set_recording(&call.id, recording_url).await;
        }
    }
    StatusCode::OK
}
