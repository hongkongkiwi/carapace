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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> VoiceConfig {
        VoiceConfig {
            enabled: true,
            account_sid: "AC1234567890".to_string(),
            auth_token: "auth_token_123".to_string(),
            phone_number: "+1234567890".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            verify_signatures: true,
            tts_voice: "Polly.Joanna".to_string(),
            tts_language: "en-US".to_string(),
            barge_in_enabled: true,
            record_calls: false,
            max_call_duration: 3600,
        }
    }

    #[test]
    fn test_twilio_client_new() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config.clone(), call_manager);

        assert_eq!(client.config.account_sid, config.account_sid);
        assert_eq!(client.config.auth_token, config.auth_token);
        assert_eq!(client.config.phone_number, config.phone_number);
    }

    #[test]
    fn test_twilio_client_api_url() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let url = client.api_url();
        assert_eq!(url, "https://api.twilio.com/2010-04-01/Accounts/AC1234567890/Calls.json");
    }

    #[test]
    fn test_verify_signature_disabled() {
        let mut config = create_test_config();
        config.verify_signatures = false;
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let params = HashMap::new();
        // Should return true when signature verification is disabled
        assert!(client.verify_signature("https://example.com", &params, "any_signature"));
    }

    #[test]
    fn test_verify_signature_with_params() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let mut params = HashMap::new();
        params.insert("CallSid".to_string(), "CA123".to_string());
        params.insert("From".to_string(), "+1234567890".to_string());

        let url = "https://example.com/webhook";

        // Generate expected signature
        let mut data = url.to_string();
        let mut keys: Vec<_> = params.keys().collect();
        keys.sort();
        for key in &keys {
            if let Some(value) = params.get(*key) {
                data.push_str(key);
                data.push_str(value);
            }
        }

        let mut mac = Hmac::<Sha256>::new_from_slice("auth_token_123".as_bytes()).unwrap();
        mac.update(data.as_bytes());
        let result = mac.finalize();
        let expected_sig = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result.into_bytes());

        // Should verify correctly with proper signature
        assert!(client.verify_signature(url, &params, &expected_sig));

        // Should fail with wrong signature
        assert!(!client.verify_signature(url, &params, "invalid_signature"));
    }

    #[test]
    fn test_verify_signature_case_insensitive() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let mut params = HashMap::new();
        params.insert("CallSid".to_string(), "CA123".to_string());

        let url = "https://example.com/webhook";

        // Generate expected signature
        let mut data = url.to_string();
        let mut keys: Vec<_> = params.keys().collect();
        keys.sort();
        for key in &keys {
            if let Some(value) = params.get(*key) {
                data.push_str(key);
                data.push_str(value);
            }
        }

        let mut mac = Hmac::<Sha256>::new_from_slice("auth_token_123".as_bytes()).unwrap();
        mac.update(data.as_bytes());
        let result = mac.finalize();
        let expected_sig = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result.into_bytes());

        // Should verify with lowercase signature
        assert!(client.verify_signature(url, &params, &expected_sig.to_lowercase()));

        // Should verify with uppercase signature
        assert!(client.verify_signature(url, &params, &expected_sig.to_uppercase()));
    }

    #[test]
    fn test_generate_stream_twiml() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let stream_url = "wss://example.com/stream";
        let twiml = client.generate_stream_twiml(stream_url);

        assert!(twiml.contains(r#"<Stream url="wss://example.com/stream">"#));
        assert!(twiml.contains(r#"<Parameter name="voice" value="Polly.Joanna" />"#));
        assert!(twiml.contains(r#"<Parameter name="language" value="en-US" />"#));
        assert!(twiml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(twiml.contains("<Response>"));
        assert!(twiml.contains("</Response>"));
    }

    #[test]
    fn test_generate_gather_twiml() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let say_text = "Hello, please speak after the tone";
        let webhook_url = "https://example.com/gather";
        let twiml = client.generate_gather_twiml(say_text, webhook_url);

        assert!(twiml.contains(r#"<Say voice="Polly.Joanna" language="en-US">Hello, please speak after the tone</Say>"#));
        assert!(twiml.contains(r#"<Gather input="speech" action="https://example.com/gather" speechTimeout="auto">"#));
        assert!(twiml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(twiml.contains("<Response>"));
    }

    #[test]
    fn test_generate_say_twiml() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let text = "Thank you for calling. Goodbye.";
        let twiml = client.generate_say_twiml(text);

        assert!(twiml.contains(r#"<Say voice="Polly.Joanna" language="en-US">Thank you for calling. Goodbye.</Say>"#));
        assert!(twiml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(twiml.contains("<Response>"));
        assert!(twiml.contains("</Response>"));
    }

    #[test]
    fn test_generate_say_twiml_with_custom_voice() {
        let mut config = create_test_config();
        config.tts_voice = "Polly.Matthew".to_string();
        config.tts_language = "en-GB".to_string();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);

        let text = "Hello from Matthew";
        let twiml = client.generate_say_twiml(text);

        assert!(twiml.contains(r#"<Say voice="Polly.Matthew" language="en-GB">Hello from Matthew</Say>"#));
    }

    #[test]
    fn test_twilio_webhook_state_creation() {
        let config = create_test_config();
        let call_manager = Arc::new(CallManager::new());
        let client = TwilioClient::new(config, call_manager);
        let stream_url = "wss://example.com/stream".to_string();

        let state = TwilioWebhookState {
            client: client.clone(),
            stream_url: stream_url.clone(),
        };

        assert_eq!(state.stream_url, stream_url);
        assert_eq!(state.client.config.account_sid, client.config.account_sid);
    }
}
