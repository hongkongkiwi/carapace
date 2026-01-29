//! Microsoft Teams Tool Plugin
//!
//! Native implementation of Microsoft Teams operations for carapace.
//! Supports sending messages to channels and users.
//!
//! Security: Webhook URL retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    ToolDefinition, ToolPluginInstance, ToolContext, ToolResult,
    BindingError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Teams tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Incoming webhook URL
    #[serde(skip)]
    pub webhook_url: Option<String>,
    /// Default channel/team name
    #[serde(default)]
    pub default_channel: String,
}

impl Default for TeamsConfig {
    fn default() -> Self { Self { webhook_url: None, default_channel: String::new() } }
}

/// Teams API client
#[derive(Debug, Clone)]
pub struct TeamsClient {
    config: TeamsConfig,
    http_client: reqwest::blocking::Client,
}

impl TeamsClient {
    pub fn new(config: TeamsConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(60))
            .build().map_err(|e| BindingError::CallError(e.to_string()))?;
        Ok(Self { config, http_client })
    }

    pub fn with_webhook(mut self, url: String) -> Self { self.config.webhook_url = Some(url); self }

    fn webhook_url(&self) -> Result<&str, BindingError> {
        self.config.webhook_url.as_deref()
            .ok_or_else(|| BindingError::CallError("Teams webhook URL not configured".to_string()))
    }

    /// Send message to channel
    pub fn send_message(&self, title: &str, text: &str) -> Result<serde_json::Value, BindingError> {
        let payload = json!({
            "@type": "MessageCard",
            "themeColor": "0076D7",
            "summary": title,
            "sections": [{
                "activityTitle": title,
                "text": text,
            }]
        });
        let resp = self.http_client.post(self.webhook_url()?)
            .json(&payload)
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!("Teams API error: {}", text)));
        }
        Ok(json!({ "status": "sent" }))
    }

    /// Send adaptive card
    pub fn send_adaptive_card(&self, card: serde_json::Value) -> Result<serde_json::Value, BindingError> {
        let payload = json!({ "@type": "MessageCard", "attachments": [{ "contentType": "application/vnd.microsoft.card.adaptive", "content": card }] });
        let resp = self.http_client.post(self.webhook_url()?)
            .json(&payload)
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!("Teams API error: {}", text)));
        }
        Ok(json!({ "status": "sent" }))
    }
}

/// Teams tool plugin
#[derive(Debug, Clone)]
pub struct TeamsTool { client: Option<TeamsClient> }

impl TeamsTool {
    pub fn new() -> Self { Self { client: None } }
    pub fn initialize(&mut self, config: TeamsConfig) -> Result<(), BindingError> {
        let client = TeamsClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for TeamsTool { fn default() -> Self { Self::new() } }

impl ToolPluginInstance for TeamsTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition { name: "teams_send".to_string(), description: "Send a message to Teams channel.".to_string(), input_schema: MessageInput::schema().to_string() },
            ToolDefinition { name: "teams_card".to_string(), description: "Send an Adaptive Card to Teams.".to_string(), input_schema: CardInput::schema().to_string() },
        ])
    }

    fn invoke(&self, name: &str, params: &str, _ctx: ToolContext) -> Result<ToolResult, BindingError> {
        let client = self.client.as_ref()
            .ok_or_else(|| BindingError::CallError("Teams tool not initialized".to_string()))?;
        match name {
            "teams_send" => {
                let input: MessageInput = serde_json::from_str(params)?;
                let result = client.send_message(&input.title, &input.text)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "teams_card" => {
                let input: CardInput = serde_json::from_str(params)?;
                let result = client.send_adaptive_card(input.card)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInput { pub title: String, pub text: String }
impl MessageInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"title": {"type": "string"}, "text": {"type": "string"}}, "required": ["title", "text"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInput { pub card: serde_json::Value }
impl CardInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"card": {"type": "object"}}, "required": ["card"]}) } }
