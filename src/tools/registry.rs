//! Tool Registry Operations
//!
//! Advanced registry features for tool management.

use super::{Tool, ToolError, ToolOutput};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Tool registry with persistence and management features
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    disabled: Vec<String>,
    categories: HashMap<String, Vec<String>>,
    metadata: HashMap<String, ToolMetadata>,
}

/// Metadata for a registered tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// When the tool was registered
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Source of the tool (built-in, plugin, etc.)
    pub source: ToolSource,
    /// Tool version
    pub version: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Usage statistics
    #[serde(skip)]
    pub stats: ToolStats,
}

/// Source of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    /// Built-in tool
    BuiltIn,
    /// Loaded from a plugin
    Plugin { name: String },
    /// External script/command
    External { path: String },
    /// Dynamically loaded
    Dynamic,
}

/// Tool usage statistics
#[derive(Debug, Clone, Default)]
pub struct ToolStats {
    /// Number of times executed
    pub execution_count: u64,
    /// Number of successful executions
    pub success_count: u64,
    /// Number of failed executions
    pub failure_count: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Last execution time
    pub last_executed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Tool listing with filters
#[derive(Debug, Clone, Default)]
pub struct ToolFilter {
    /// Filter by source
    pub source: Option<ToolSource>,
    /// Filter by category
    pub category: Option<String>,
    /// Include disabled tools
    pub include_disabled: bool,
    /// Filter by tags
    pub tags: Vec<String>,
}

/// Tool search result
#[derive(Debug, Clone)]
pub struct ToolSearchResult {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            disabled: Vec::new(),
            categories: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Register a tool with metadata
    pub fn register_with_metadata(
        &mut self,
        tool: Arc<dyn Tool>,
        metadata: ToolMetadata,
    ) {
        let name = tool.name().to_string();

        // Store category mapping
        if let Some(category) = tool.category() {
            self.categories
                .entry(category.to_string())
                .or_default()
                .push(name.clone());
        }

        self.metadata.insert(name.clone(), metadata);
        self.tools.insert(name.clone(), tool);

        info!(tool = %name, "Registered tool");
    }

    /// Register a tool (simple version)
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let metadata = ToolMetadata {
            registered_at: chrono::Utc::now(),
            source: ToolSource::BuiltIn,
            version: None,
            tags: Vec::new(),
            stats: ToolStats::default(),
        };
        self.register_with_metadata(tool, metadata);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if self.is_disabled(name) {
            return None;
        }
        self.tools.get(name).cloned()
    }

    /// Get tool metadata
    pub fn get_metadata(&self, name: &str) -> Option<&ToolMetadata> {
        self.metadata.get(name)
    }

    /// Get mutable metadata for updating stats
    pub fn get_metadata_mut(&mut self, name: &str) -> Option<&mut ToolMetadata> {
        self.metadata.get_mut(name)
    }

    /// Get tool information
    pub fn get_tool_info(&self, name: &str) -> Option<ToolInfo> {
        let tool = self.tools.get(name)?;
        Some(ToolInfo {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters_schema: tool.parameters_schema(),
            requires_approval: tool.requires_approval(),
            category: tool.category().map(|s| s.to_string()),
            enabled: !self.is_disabled(name),
            metadata: self.metadata.get(name).cloned(),
        })
    }

    /// Check if a tool exists and is enabled
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name) && !self.is_disabled(name)
    }

    /// Check if a tool is registered (including disabled)
    pub fn is_registered(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Check if a tool is disabled
    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.contains(&name.to_string())
    }

    /// Disable a tool
    pub fn disable(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !self.disabled.contains(&name) {
            self.disabled.push(name.clone());
            warn!(tool = %name, "Tool disabled");
        }
    }

    /// Enable a previously disabled tool
    pub fn enable(&mut self, name: &str) {
        self.disabled.retain(|n| n != name);
        info!(tool = %name, "Tool enabled");
    }

    /// Remove a tool from the registry
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        self.metadata.remove(name);
        self.tools.remove(name)
    }

    /// List all enabled tools
    pub fn list(&self) -> Vec<&str> {
        self.tools
            .keys()
            .filter(|k| !self.is_disabled(k))
            .map(|s| s.as_str())
            .collect()
    }

    /// List all tools with filter
    pub fn list_filtered(&self, filter: ToolFilter) -> Vec<(String, ToolInfo)> {
        self.tools
            .iter()
            .filter(|(name, _)| {
                // Check disabled filter
                if !filter.include_disabled && self.is_disabled(name) {
                    return false;
                }

                // Check source filter
                if let Some(ref source) = filter.source {
                    if let Some(meta) = self.metadata.get(name.as_str()) {
                        if !source_matches(&meta.source, source) {
                            return false;
                        }
                    }
                }

                // Check tag filter
                if !filter.tags.is_empty() {
                    if let Some(meta) = self.metadata.get(name.as_str()) {
                        let has_tag = filter
                            .tags
                            .iter()
                            .any(|t| meta.tags.contains(t));
                        if !has_tag {
                            return false;
                        }
                    }
                }

                true
            })
            .map(|(name, tool)| {
                let info = ToolInfo {
                    name: name.clone(),
                    description: tool.description().to_string(),
                    parameters_schema: tool.parameters_schema(),
                    requires_approval: tool.requires_approval(),
                    category: tool.category().map(|s| s.to_string()),
                    enabled: !self.is_disabled(name),
                    metadata: self.metadata.get(name).cloned(),
                };
                (name.clone(), info)
            })
            .collect()
    }

    /// List tools in a category
    pub fn list_by_category(&self, category: &str) -> Vec<&str> {
        self.categories
            .get(category)
            .map(|tools| {
                tools
                    .iter()
                    .filter(|t| !self.is_disabled(t))
                    .map(|s| s.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<&str> {
        self.categories.keys().map(|s| s.as_str()).collect()
    }

    /// List all tools with their schemas
    pub fn list_with_schemas(&self) -> Vec<(&str, &str, serde_json::Value)> {
        self.tools
            .values()
            .filter(|t| !self.is_disabled(t.name()))
            .map(|t| (t.name(), t.description(), t.parameters_schema()))
            .collect()
    }

    /// Get all tool definitions for AI function calling
    pub fn get_definitions(&self) -> Vec<super::super::ai::types::Tool> {
        self.tools
            .values()
            .filter(|t| !self.is_disabled(t.name()))
            .map(|t| super::super::ai::types::Tool {
                r#type: "function".to_string(),
                function: super::super::ai::types::Function {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    parameters: t.parameters_schema(),
                },
            })
            .collect()
    }

    /// Search tools by name or description
    pub fn search(&self, query: &str) -> Vec<ToolSearchResult> {
        let query_lower = query.to_lowercase();

        self.tools
            .values()
            .filter(|t| !self.is_disabled(t.name()))
            .filter_map(|t| {
                let name_lower = t.name().to_lowercase();
                let desc_lower = t.description().to_lowercase();

                let name_score = if name_lower == query_lower {
                    1.0
                } else if name_lower.contains(&query_lower) {
                    0.8
                } else {
                    0.0
                };

                let desc_score = if desc_lower.contains(&query_lower) {
                    0.5
                } else {
                    0.0
                };

                let relevance = name_score + desc_score;

                if relevance > 0.0 {
                    Some(ToolSearchResult {
                        name: t.name().to_string(),
                        description: t.description().to_string(),
                        relevance,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get registry statistics
    pub fn stats(&self) -> RegistryStats {
        RegistryStats {
            total_tools: self.tools.len(),
            enabled_tools: self.tools.len() - self.disabled.len(),
            disabled_tools: self.disabled.len(),
            categories: self.categories.len(),
        }
    }

    /// Record tool execution
    pub fn record_execution(&mut self, tool_name: &str, success: bool, duration_ms: u64) {
        if let Some(metadata) = self.metadata.get_mut(tool_name) {
            metadata.stats.execution_count += 1;
            metadata.stats.total_execution_time_ms += duration_ms;
            metadata.stats.last_executed = Some(chrono::Utc::now());

            if success {
                metadata.stats.success_count += 1;
            } else {
                metadata.stats.failure_count += 1;
            }
        }
    }

    /// Export registry to JSON
    pub fn export(&self) -> Result<String, serde_json::Error> {
        let export = RegistryExport {
            tools: self
                .tools
                .values()
                .map(|t| ToolExport {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    schema: t.parameters_schema(),
                    requires_approval: t.requires_approval(),
                    category: t.category().map(|s| s.to_string()),
                    metadata: self.metadata.get(t.name()).cloned(),
                })
                .collect(),
            disabled: self.disabled.clone(),
        };

        serde_json::to_string_pretty(&export)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_tools: usize,
    pub enabled_tools: usize,
    pub disabled_tools: usize,
    pub categories: usize,
}

/// Tool information for display
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub requires_approval: bool,
    pub category: Option<String>,
    pub enabled: bool,
    pub metadata: Option<ToolMetadata>,
}

/// Registry export format
#[derive(Debug, Serialize, Deserialize)]
struct RegistryExport {
    tools: Vec<ToolExport>,
    disabled: Vec<String>,
}

/// Tool export format
#[derive(Debug, Serialize, Deserialize)]
struct ToolExport {
    name: String,
    description: String,
    schema: serde_json::Value,
    requires_approval: bool,
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<ToolMetadata>,
}

/// Check if two sources match
fn source_matches(a: &ToolSource, b: &ToolSource) -> bool {
    matches!((a, b), (ToolSource::BuiltIn, ToolSource::BuiltIn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::builtins::EchoTool;

    #[test]
    fn test_registry_basic() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        assert!(registry.has("echo"));
        assert!(!registry.has("nonexistent"));
    }

    #[test]
    fn test_registry_disable_enable() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        assert!(registry.has("echo"));

        registry.disable("echo");
        assert!(!registry.has("echo"));
        assert!(registry.is_disabled("echo"));

        registry.enable("echo");
        assert!(registry.has("echo"));
        assert!(!registry.is_disabled("echo"));
    }

    #[test]
    fn test_registry_search() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let results = registry.search("echo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "echo");
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let stats = registry.stats();
        assert_eq!(stats.total_tools, 1);
        assert_eq!(stats.enabled_tools, 1);
    }
}
