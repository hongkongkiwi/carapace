//! Notion Tool Plugin
//!
//! Native implementation of Notion API operations for carapace.
//! Supports pages, databases, and blocks.
//!
//! Security: API key retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    BindingError, ToolContext, ToolDefinition, ToolPluginInstance, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Notion tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionConfig {
    #[serde(skip)]
    pub api_key: Option<String>,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    "https://api.notion.com/v1".to_string()
}

impl Default for NotionConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: default_base_url(),
        }
    }
}

/// Notion API client
#[derive(Debug, Clone)]
pub struct NotionClient {
    config: NotionConfig,
    http_client: reqwest::blocking::Client,
}

impl NotionClient {
    pub fn new(config: NotionConfig) -> Result<Self, BindingError> {
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

    fn auth_headers(&self) -> Result<Vec<(String, String)>, BindingError> {
        let key =
            self.config.api_key.as_ref().ok_or_else(|| {
                BindingError::CallError("Notion API key not configured".to_string())
            })?;
        Ok(vec![
            ("Authorization".to_string(), format!("Bearer {}", key)),
            ("Notion-Version".to_string(), "2022-06-28".to_string()),
        ])
    }

    fn request(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BindingError> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        let mut request = self.http_client.request(method, &url);
        for (k, v) in self.auth_headers()? {
            request = request.header(k, v);
        }
        request = request.header("Content-Type", "application/json");
        if let Some(b) = body {
            request = request.json(&b);
        }
        let resp = request
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!(
                "Notion API error: {}",
                text
            )));
        }
        resp.json()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    /// Create page
    pub fn create_page(
        &self,
        parent_id: &str,
        title: &str,
        content: &str,
    ) -> Result<serde_json::Value, BindingError> {
        let body = json!({
            "parent": { "page_id": parent_id },
            "properties": { "title": { "title": [{ "text": { "content": title } }] } },
            "children": [{ "object": "block", "type": "paragraph", "paragraph": { "rich_text": [{ "text": { "content": content } }] } }]
        });
        self.request(reqwest::Method::POST, "/pages", Some(body))
    }

    /// Get page
    pub fn get_page(&self, page_id: &str) -> Result<serde_json::Value, BindingError> {
        self.request(reqwest::Method::GET, &format!("/pages/{}", page_id), None)
    }

    /// Query database
    pub fn query_database(&self, database_id: &str) -> Result<serde_json::Value, BindingError> {
        self.request(
            reqwest::Method::POST,
            &format!("/databases/{}/query", database_id),
            None,
        )
    }

    /// Update page
    pub fn update_page(
        &self,
        page_id: &str,
        properties: serde_json::Value,
    ) -> Result<serde_json::Value, BindingError> {
        self.request(
            reqwest::Method::PATCH,
            &format!("/pages/{}", page_id),
            Some(properties),
        )
    }
}

/// Notion tool plugin
#[derive(Debug, Clone)]
pub struct NotionTool {
    client: Option<NotionClient>,
}

impl NotionTool {
    pub fn new() -> Self {
        Self { client: None }
    }
    pub fn initialize(&mut self, config: NotionConfig) -> Result<(), BindingError> {
        let client = NotionClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for NotionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for NotionTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "notion_create_page".to_string(),
                description: "Create a new page.".to_string(),
                input_schema: PageInput::schema().to_string(),
            },
            ToolDefinition {
                name: "notion_get_page".to_string(),
                description: "Get page details.".to_string(),
                input_schema: GetPageInput::schema().to_string(),
            },
            ToolDefinition {
                name: "notion_query_db".to_string(),
                description: "Query a database.".to_string(),
                input_schema: QueryInput::schema().to_string(),
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
            .ok_or_else(|| BindingError::CallError("Notion tool not initialized".to_string()))?;
        match name {
            "notion_create_page" => {
                let input: PageInput = serde_json::from_str(params)?;
                let result = client.create_page(&input.parent_id, &input.title, &input.content)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "notion_get_page" => {
                let input: GetPageInput = serde_json::from_str(params)?;
                let result = client.get_page(&input.page_id)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "notion_query_db" => {
                let input: QueryInput = serde_json::from_str(params)?;
                let result = client.query_database(&input.database_id)?;
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
pub struct PageInput {
    pub parent_id: String,
    pub title: String,
    pub content: String,
}
impl PageInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"parent_id": {"type": "string"}, "title": {"type": "string"}, "content": {"type": "string"}}, "required": ["parent_id", "title", "content"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPageInput {
    pub page_id: String,
}
impl GetPageInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"page_id": {"type": "string"}}, "required": ["page_id"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInput {
    pub database_id: String,
}
impl QueryInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"database_id": {"type": "string"}}, "required": ["database_id"]})
    }
}
