//! Jira Tool Plugin
//!
//! Native implementation of Jira API operations for carapace.
//! Supports issues, projects, and transitions.
//!
//! Security: API token retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    ToolDefinition, ToolPluginInstance, ToolContext, ToolResult,
    BindingError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Jira tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    #[serde(skip)]
    pub email: Option<String>,
    #[serde(skip)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub project_key: String,
}

impl Default for JiraConfig {
    fn default() -> Self { Self { email: None, api_token: None, base_url: String::new(), project_key: String::new() } }
}

/// Jira API client
#[derive(Debug, Clone)]
pub struct JiraClient {
    config: JiraConfig,
    http_client: reqwest::blocking::Client,
    auth_header: String,
}

impl JiraClient {
    pub fn new(config: JiraConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(60))
            .build().map_err(|e| BindingError::CallError(e.to_string()))?;
        let email = config.email.as_ref()
            .ok_or_else(|| BindingError::CallError("Jira email not configured".to_string()))?;
        let token = config.api_token.as_ref()
            .ok_or_else(|| BindingError::CallError("Jira API token not configured".to_string()))?;
        let creds = format!("{}:{}", email, token);
        use base64::Engine;
        let auth = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(creds.as_bytes()));
        Ok(Self { config, http_client, auth_header: auth })
    }

    fn request(&self, method: reqwest::Method, endpoint: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, BindingError> {
        let url = format!("{}/rest/api/2{}", self.config.base_url, endpoint);
        let mut request = self.http_client.request(method, &url);
        request = request.header("Authorization", &self.auth_header);
        request = request.header("Content-Type", "application/json");
        if let Some(b) = body { request = request.json(&b); }
        let resp = request.send().map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!("Jira API error: {}", text)));
        }
        resp.json().map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    /// Create issue
    pub fn create_issue(&self, summary: &str, description: &str, issue_type: &str) -> Result<serde_json::Value, BindingError> {
        let body = json!({
            "fields": {
                "project": { "key": self.config.project_key },
                "summary": summary,
                "description": description,
                "issuetype": { "name": issue_type }
            }
        });
        self.request(reqwest::Method::POST, "/issue", Some(body))
    }

    /// Get issue
    pub fn get_issue(&self, issue_key: &str) -> Result<serde_json::Value, BindingError> {
        self.request(reqwest::Method::GET, &format!("/issue/{}", issue_key), None)
    }

    /// Transition issue
    pub fn transition_issue(&self, issue_key: &str, transition_id: &str) -> Result<serde_json::Value, BindingError> {
        let body = json!({ "transition": { "id": transition_id } });
        self.request(reqwest::Method::POST, &format!("/issue/{}/transitions", issue_key), Some(body))
    }

    /// Add comment
    pub fn add_comment(&self, issue_key: &str, comment: &str) -> Result<serde_json::Value, BindingError> {
        let body = json!({ "body": comment });
        self.request(reqwest::Method::POST, &format!("/issue/{}/comment", issue_key), Some(body))
    }
}

/// Jira tool plugin
#[derive(Debug, Clone)]
pub struct JiraTool { client: Option<JiraClient> }

impl JiraTool {
    pub fn new() -> Self { Self { client: None } }
    pub fn initialize(&mut self, config: JiraConfig) -> Result<(), BindingError> {
        let client = JiraClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for JiraTool { fn default() -> Self { Self::new() } }

impl ToolPluginInstance for JiraTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition { name: "jira_create_issue".to_string(), description: "Create a new issue.".to_string(), input_schema: IssueInput::schema().to_string() },
            ToolDefinition { name: "jira_get_issue".to_string(), description: "Get issue details.".to_string(), input_schema: GetIssueInput::schema().to_string() },
            ToolDefinition { name: "jira_transition".to_string(), description: "Transition an issue to a new state.".to_string(), input_schema: TransitionInput::schema().to_string() },
            ToolDefinition { name: "jira_comment".to_string(), description: "Add a comment to an issue.".to_string(), input_schema: CommentInput::schema().to_string() },
        ])
    }

    fn invoke(&self, name: &str, params: &str, _ctx: ToolContext) -> Result<ToolResult, BindingError> {
        let client = self.client.as_ref()
            .ok_or_else(|| BindingError::CallError("Jira tool not initialized".to_string()))?;
        match name {
            "jira_create_issue" => {
                let input: IssueInput = serde_json::from_str(params)?;
                let result = client.create_issue(&input.summary, &input.description, &input.issue_type)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "jira_get_issue" => {
                let input: GetIssueInput = serde_json::from_str(params)?;
                let result = client.get_issue(&input.issue_key)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "jira_transition" => {
                let input: TransitionInput = serde_json::from_str(params)?;
                let result = client.transition_issue(&input.issue_key, &input.transition_id)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "jira_comment" => {
                let input: CommentInput = serde_json::from_str(params)?;
                let result = client.add_comment(&input.issue_key, &input.comment)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInput { pub summary: String, pub description: String, pub issue_type: String }
impl IssueInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"summary": {"type": "string"}, "description": {"type": "string"}, "issue_type": {"type": "string"}}, "required": ["summary", "description", "issue_type"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIssueInput { pub issue_key: String }
impl GetIssueInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"issue_key": {"type": "string"}}, "required": ["issue_key"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionInput { pub issue_key: String, pub transition_id: String }
impl TransitionInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"issue_key": {"type": "string"}, "transition_id": {"type": "string"}}, "required": ["issue_key", "transition_id"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentInput { pub issue_key: String, pub comment: String }
impl CommentInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"issue_key": {"type": "string"}, "comment": {"type": "string"}}, "required": ["issue_key", "comment"]}) } }
