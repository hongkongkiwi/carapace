//! Twilio Tool Plugin
//!
//! Native implementation of Twilio SMS/Voice operations for carapace.
//! Supports sending SMS and making voice calls.
//!
//! Security: Credentials retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    BindingError, ToolContext, ToolDefinition, ToolPluginInstance, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Twilio tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    #[serde(skip)]
    pub account_sid: Option<String>,
    #[serde(skip)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub from_number: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    "https://api.twilio.com/2010-04-01".to_string()
}

impl Default for TwilioConfig {
    fn default() -> Self {
        Self {
            account_sid: None,
            auth_token: None,
            from_number: String::new(),
            base_url: default_base_url(),
        }
    }
}

/// Twilio API client
#[derive(Debug, Clone)]
pub struct TwilioClient {
    config: TwilioConfig,
    http_client: reqwest::blocking::Client,
}

impl TwilioClient {
    pub fn new(config: TwilioConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| BindingError::CallError(e.to_string()))?;
        Ok(Self {
            config,
            http_client,
        })
    }

    fn auth_header(&self) -> Result<String, BindingError> {
        let sid = self.config.account_sid.as_ref().ok_or_else(|| {
            BindingError::CallError("Twilio account_sid not configured".to_string())
        })?;
        let token = self.config.auth_token.as_ref().ok_or_else(|| {
            BindingError::CallError("Twilio auth_token not configured".to_string())
        })?;
        let creds = format!("{}:{}", sid, token);
        use base64::Engine;
        Ok(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(creds.as_bytes())
        ))
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BindingError> {
        let url = format!("{}{}", self.config.base_url, path);
        let mut request = self.http_client.request(method, &url);
        request = request.header("Authorization", self.auth_header()?);
        request = request.header("Content-Type", "application/x-www-form-urlencoded");
        if let Some(b) = body {
            request = request.body(Self::form_encode(&b));
        }
        let resp = request
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!(
                "Twilio API error: {}",
                text
            )));
        }
        resp.json()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    fn form_encode(data: &serde_json::Value) -> String {
        match data {
            serde_json::Value::Object(map) => map
                .iter()
                .filter_map(|(k, v)| Some(format!("{}={}", k, v.as_str().unwrap_or(""))))
                .collect::<Vec<_>>()
                .join("&"),
            _ => String::new(),
        }
    }

    /// Send SMS
    pub fn send_sms(&self, to: &str, body: &str) -> Result<serde_json::Value, BindingError> {
        let params = json!({ "To": to, "From": self.config.from_number, "Body": body });
        self.request(
            reqwest::Method::POST,
            &format!(
                "/Accounts/{}/Messages.json",
                self.config.account_sid.as_ref().unwrap_or(&String::new())
            ),
            Some(params),
        )
    }

    /// Make voice call
    pub fn make_call(&self, to: &str, url: &str) -> Result<serde_json::Value, BindingError> {
        let params = json!({ "To": to, "From": self.config.from_number, "Url": url });
        self.request(
            reqwest::Method::POST,
            &format!(
                "/Accounts/{}/Calls.json",
                self.config.account_sid.as_ref().unwrap_or(&String::new())
            ),
            Some(params),
        )
    }
}

/// Twilio tool plugin
#[derive(Debug, Clone)]
pub struct TwilioTool {
    client: Option<TwilioClient>,
}

impl TwilioTool {
    pub fn new() -> Self {
        Self { client: None }
    }
    pub fn initialize(&mut self, config: TwilioConfig) -> Result<(), BindingError> {
        let client = TwilioClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for TwilioTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for TwilioTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "twilio_sms".to_string(),
                description: "Send an SMS message.".to_string(),
                input_schema: SmsInput::schema().to_string(),
            },
            ToolDefinition {
                name: "twilio_call".to_string(),
                description: "Make a voice call with TwiML URL.".to_string(),
                input_schema: CallInput::schema().to_string(),
            },
        ])
    }

    fn invoke(
        &self,
        name: &str,
        params: &str,
        _ctx: ToolContext,
    ) -> Result<ToolResult, BindingError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| BindingError::CallError("Twilio tool not initialized".to_string()))?;
        match name {
            "twilio_sms" => {
                let input: SmsInput = serde_json::from_str(params)?;
                let result = client.send_sms(&input.to, &input.body)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "twilio_call" => {
                let input: CallInput = serde_json::from_str(params)?;
                let result = client.make_call(&input.to, &input.twiml_url)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsInput {
    pub to: String,
    pub body: String,
}
impl SmsInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"to": {"type": "string"}, "body": {"type": "string"}}, "required": ["to", "body"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallInput {
    pub to: String,
    pub twiml_url: String,
}
impl CallInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"to": {"type": "string"}, "twiml_url": {"type": "string"}}, "required": ["to", "twiml_url"]})
    }
}
