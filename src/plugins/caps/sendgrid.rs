//! SendGrid Tool Plugin
//!
//! Native implementation of SendGrid email operations for carapace.
//! Supports sending emails and managing templates.
//!
//! Security: API key retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    ToolDefinition, ToolPluginInstance, ToolContext, ToolResult,
    BindingError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// SendGrid tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendGridConfig {
    /// API key
    #[serde(skip)]
    pub api_key: Option<String>,

    /// Default from email
    #[serde(default)]
    pub from_email: String,

    /// Default from name
    #[serde(default)]
    pub from_name: String,
}

impl Default for SendGridConfig {
    fn default() -> Self {
        Self { api_key: None, from_email: String::new(), from_name: String::new() }
    }
}

/// SendGrid API client
#[derive(Debug, Clone)]
pub struct SendGridClient {
    config: SendGridConfig,
    http_client: reqwest::blocking::Client,
}

impl SendGridClient {
    pub fn new(config: SendGridConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(60))
            .build().map_err(|e| BindingError::CallError(e.to_string()))?;
        Ok(Self { config, http_client })
    }

    pub fn with_key(mut self, key: String) -> Self { self.config.api_key = Some(key); self }

    fn auth_headers(&self) -> Result<Vec<(String, String)>, BindingError> {
        let key = self.config.api_key.as_ref()
            .ok_or_else(|| BindingError::CallError("SendGrid API key not configured".to_string()))?;
        Ok(vec![("Authorization".to_string(), format!("Bearer {}", key))])
    }

    fn request(&self, body: serde_json::Value) -> Result<serde_json::Value, BindingError> {
        let url = "https://api.sendgrid.com/v3/mail/send";
        let mut request = self.http_client.post(url);
        for (k, v) in self.auth_headers()? { request = request.header(k, v); }
        request = request.header("Content-Type", "application/json").json(&body);
        let resp = request.send().map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!("SendGrid API error: {}", text)));
        }
        Ok(json!({ "status": "sent", "status_code": resp.status().as_u16() }))
    }

    /// Send email
    pub fn send_email(&self, to: &str, subject: &str, body: &str, html: Option<&str>) -> Result<serde_json::Value, BindingError> {
        let email_body = json!({
            "personalizations": [{ "to": [{ "email": to }] }],
            "from": { "email": self.config.from_email, "name": self.config.from_name },
            "subject": subject,
            "content": [
                { "type": "text/plain", "value": body },
                if let Some(h) = html { vec![json!({ "type": "text/html", "value": h })] } else { vec![] }
            ]
        });
        self.request(email_body)
    }

    /// Send template email
    pub fn send_template_email(&self, to: &str, template_id: &str, dynamic_data: serde_json::Value) -> Result<serde_json::Value, BindingError> {
        let email_body = json!({
            "personalizations": [{ "to": [{ "email": to }], "dynamic_template_data": dynamic_data }],
            "from": { "email": self.config.from_email, "name": self.config.from_name },
            "template_id": template_id
        });
        self.request(email_body)
    }
}

/// SendGrid tool plugin
#[derive(Debug, Clone)]
pub struct SendGridTool { client: Option<SendGridClient> }

impl SendGridTool {
    pub fn new() -> Self { Self { client: None } }
    pub fn initialize(&mut self, config: SendGridConfig) -> Result<(), BindingError> {
        let client = SendGridClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for SendGridTool { fn default() -> Self { Self::new() } }

impl ToolPluginInstance for SendGridTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition { name: "sendgrid_send".to_string(), description: "Send an email.".to_string(), input_schema: SendInput::schema().to_string() },
            ToolDefinition { name: "sendgrid_template".to_string(), description: "Send a template email.".to_string(), input_schema: TemplateInput::schema().to_string() },
        ])
    }

    fn invoke(&self, name: &str, params: &str, _ctx: ToolContext) -> Result<ToolResult, BindingError> {
        let client = self.client.as_ref()
            .ok_or_else(|| BindingError::CallError("SendGrid tool not initialized".to_string()))?;
        match name {
            "sendgrid_send" => {
                let input: SendInput = serde_json::from_str(params)?;
                let result = client.send_email(&input.to, &input.subject, &input.body, input.html.as_deref())?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "sendgrid_template" => {
                let input: TemplateInput = serde_json::from_str(params)?;
                let result = client.send_template_email(&input.to, &input.template_id, input.dynamic_data.unwrap_or_else(|| json!({})))?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInput { pub to: String, pub subject: String, pub body: String, pub html: Option<String> }
impl SendInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"to": {"type": "string"}, "subject": {"type": "string"}, "body": {"type": "string"}, "html": {"type": "string"}}, "required": ["to", "subject", "body"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInput { pub to: String, pub template_id: String, pub dynamic_data: Option<serde_json::Value> }
impl TemplateInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"to": {"type": "string"}, "template_id": {"type": "string"}, "dynamic_data": {"type": "object"}}, "required": ["to", "template_id"]}) } }
