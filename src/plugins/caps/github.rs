//! GitHub Tool Plugin
//!
//! Native implementation of GitHub API operations for carapace.
//! Supports issues, PRs, repos, and actions.
//!
//! Security: API token retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    ToolDefinition, ToolPluginInstance, ToolContext, ToolResult,
    BindingError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// GitHub tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// Personal access token
    #[serde(skip)]
    pub token: Option<String>,

    /// API base URL
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// Default owner/repo for operations
    #[serde(default)]
    pub default_repo: String,
}

fn default_base_url() -> String {
    "https://api.github.com".to_string()
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            token: None,
            base_url: default_base_url(),
            default_repo: String::new(),
        }
    }
}

/// GitHub API client
#[derive(Debug, Clone)]
pub struct GitHubClient {
    config: GitHubConfig,
    http_client: reqwest::blocking::Client,
}

impl GitHubClient {
    pub fn new(config: GitHubConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| BindingError::CallError(e.to_string()))?;
        Ok(Self { config, http_client })
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.config.token = Some(token);
        self
    }

    fn auth_headers(&self) -> Result<Vec<(String, String)>, BindingError> {
        let token = self.config.token.as_ref()
            .ok_or_else(|| BindingError::CallError("GitHub token not configured".to_string()))?;
        Ok(vec![("Authorization".to_string(), format!("Bearer {}", token))])
    }

    fn request(&self, method: reqwest::Method, endpoint: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, BindingError> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        let mut request = self.http_client.request(method, &url);
        for (k, v) in self.auth_headers()? {
            request = request.header(k, v);
        }
        request = request.header("Accept", "application/vnd.github.v3+json");
        if let Some(b) = body {
            request = request.json(&b);
        }
        let resp = request.send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!("GitHub API error: {}", text)));
        }
        resp.json().map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    /// Get repository info
    pub fn get_repo(&self, owner: &str, repo: &str) -> Result<serde_json::Value, BindingError> {
        self.request(reqwest::Method::GET, &format!("/repos/{}/{}", owner, repo), None)
    }

    /// Create issue
    pub fn create_issue(&self, owner: &str, repo: &str, title: &str, body: Option<&str>) -> Result<serde_json::Value, BindingError> {
        let body_json = json!({ "title": title, "body": body.unwrap_or("") });
        self.request(reqwest::Method::POST, &format!("/repos/{}/{}/issues", owner, repo), Some(body_json))
    }

    /// List issues
    pub fn list_issues(&self, owner: &str, repo: &str) -> Result<serde_json::Value, BindingError> {
        self.request(reqwest::Method::GET, &format!("/repos/{}/{}/issues", owner, repo), None)
    }

    /// Get user info
    pub fn get_user(&self, username: &str) -> Result<serde_json::Value, BindingError> {
        self.request(reqwest::Method::GET, &format!("/users/{}", username), None)
    }
}

/// GitHub tool plugin
#[derive(Debug, Clone)]
pub struct GitHubTool {
    client: Option<GitHubClient>,
}

impl GitHubTool {
    pub fn new() -> Self { Self { client: None } }
    pub fn initialize(&mut self, config: GitHubConfig) -> Result<(), BindingError> {
        let client = GitHubClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for GitHubTool { fn default() -> Self { Self::new() } }

impl ToolPluginInstance for GitHubTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition { name: "github_get_repo".to_string(), description: "Get repository information.".to_string(), input_schema: RepoInput::schema().to_string() },
            ToolDefinition { name: "github_create_issue".to_string(), description: "Create a new issue.".to_string(), input_schema: IssueInput::schema().to_string() },
            ToolDefinition { name: "github_list_issues".to_string(), description: "List issues in a repository.".to_string(), input_schema: RepoInput::schema().to_string() },
            ToolDefinition { name: "github_get_user".to_string(), description: "Get user information.".to_string(), input_schema: UserInput::schema().to_string() },
        ])
    }

    fn invoke(&self, name: &str, params: &str, _ctx: ToolContext) -> Result<ToolResult, BindingError> {
        let client = self.client.as_ref()
            .ok_or_else(|| BindingError::CallError("GitHub tool not initialized".to_string()))?;
        match name {
            "github_get_repo" => {
                let input: RepoInput = serde_json::from_str(params)?;
                let result = client.get_repo(&input.owner, &input.repo)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "github_create_issue" => {
                let input: IssueInput = serde_json::from_str(params)?;
                let result = client.create_issue(&input.owner, &input.repo, &input.title, input.body.as_deref())?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "github_list_issues" => {
                let input: RepoInput = serde_json::from_str(params)?;
                let result = client.list_issues(&input.owner, &input.repo)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            "github_get_user" => {
                let input: UserInput = serde_json::from_str(params)?;
                let result = client.get_user(&input.username)?;
                Ok(ToolResult { success: true, result: Some(serde_json::to_string(&result)?), error: None })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInput { pub owner: String, pub repo: String }

impl RepoInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}}, "required": ["owner", "repo"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInput { pub owner: String, pub repo: String, pub title: String, pub body: Option<String> }

impl IssueInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}, "title": {"type": "string"}, "body": {"type": "string"}}, "required": ["owner", "repo", "title"]}) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInput { pub username: String }

impl UserInput { fn schema() -> serde_json::Value { json!({"type": "object", "properties": {"username": {"type": "string"}}, "required": ["username"]}) } }
