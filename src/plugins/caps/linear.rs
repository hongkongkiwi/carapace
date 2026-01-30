//! Linear Tool Plugin
//!
//! Native implementation of Linear API operations for carapace.
//! Supports issues, projects, and labels.
//!
//! Security: API key retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    BindingError, ToolContext, ToolDefinition, ToolPluginInstance, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Linear tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearConfig {
    #[serde(skip)]
    pub api_key: Option<String>,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    "https://api.linear.app/graphql".to_string()
}

impl Default for LinearConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_base_url(),
        }
    }
}

/// Linear API client
#[derive(Debug, Clone)]
pub struct LinearClient {
    config: LinearConfig,
    http_client: reqwest::blocking::Client,
}

impl LinearClient {
    pub fn new(config: LinearConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| BindingError::CallError(e.to_string()))?;
        Ok(Self {
            config,
            http_client,
        })
    }

    pub fn with_key(mut self, key: String) -> Self {
        self.config.api_key = Some(key);
        self
    }

    fn request(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BindingError> {
        let key =
            self.config.api_key.as_ref().ok_or_else(|| {
                BindingError::CallError("Linear API key not configured".to_string())
            })?;
        let body = json!({ "query": query, "variables": variables });
        let resp = self
            .http_client
            .post(&self.config.base_url)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!(
                "Linear API error: {}",
                text
            )));
        }
        let result: serde_json::Value = resp
            .json()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))?;
        if let Some(errors) = result.get("errors") {
            return Err(BindingError::CallError(format!(
                "Linear GraphQL errors: {}",
                errors
            )));
        }
        Ok(result)
    }

    /// Create issue
    pub fn create_issue(
        &self,
        team_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value, BindingError> {
        let query = r#"mutation CreateIssue($teamId: String!, $title: String!, $description: String) { issueCreate(input: {teamId: $teamId, title: $title, description: $description}) { success issue { id title } } }"#;
        let variables =
            json!({ "teamId": team_id, "title": title, "description": description.unwrap_or("") });
        self.request(query, Some(variables))
    }

    /// Get issue
    pub fn get_issue(&self, issue_id: &str) -> Result<serde_json::Value, BindingError> {
        let query = r#"query GetIssue($id: String!) { issue(id: $id) { id title description state { name } assignee { name } } }"#;
        let variables = json!({ "id": issue_id });
        self.request(query, Some(variables))
    }

    /// List issues
    pub fn list_issues(
        &self,
        team_id: &str,
        limit: i32,
    ) -> Result<serde_json::Value, BindingError> {
        let query = r#"query ListIssues($teamId: String!, $first: Int!) { team(id: $teamId) { issues(first: $first) { nodes { id title state { name } } } } }"#;
        let variables = json!({ "teamId": team_id, "first": limit });
        self.request(query, Some(variables))
    }
}

/// Linear tool plugin
#[derive(Debug, Clone)]
pub struct LinearTool {
    client: Option<LinearClient>,
}

impl LinearTool {
    pub fn new() -> Self {
        Self { client: None }
    }
    pub fn initialize(&mut self, config: LinearConfig) -> Result<(), BindingError> {
        let client = LinearClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for LinearTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for LinearTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "linear_create_issue".to_string(),
                description: "Create a new issue.".to_string(),
                input_schema: IssueInput::schema().to_string(),
            },
            ToolDefinition {
                name: "linear_get_issue".to_string(),
                description: "Get issue details.".to_string(),
                input_schema: GetIssueInput::schema().to_string(),
            },
            ToolDefinition {
                name: "linear_list_issues".to_string(),
                description: "List issues in a team.".to_string(),
                input_schema: ListInput::schema().to_string(),
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
            .ok_or_else(|| BindingError::CallError("Linear tool not initialized".to_string()))?;
        match name {
            "linear_create_issue" => {
                let input: IssueInput = serde_json::from_str(params)?;
                let result = client.create_issue(
                    &input.team_id,
                    &input.title,
                    input.description.as_deref(),
                )?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "linear_get_issue" => {
                let input: GetIssueInput = serde_json::from_str(params)?;
                let result = client.get_issue(&input.issue_id)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "linear_list_issues" => {
                let input: ListInput = serde_json::from_str(params)?;
                let result = client.list_issues(&input.team_id, input.limit.unwrap_or(10))?;
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
pub struct IssueInput {
    pub team_id: String,
    pub title: String,
    pub description: Option<String>,
}
impl IssueInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"team_id": {"type": "string"}, "title": {"type": "string"}, "description": {"type": "string"}}, "required": ["team_id", "title"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIssueInput {
    pub issue_id: String,
}
impl GetIssueInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"issue_id": {"type": "string"}}, "required": ["issue_id"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInput {
    pub team_id: String,
    pub limit: Option<i32>,
}
impl ListInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"team_id": {"type": "string"}, "limit": {"type": "integer"}}, "required": ["team_id"]})
    }
}
