//! Agent Context
//!
//! Context management for agent conversations including memory integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context provided to the agent for a turn
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// Relevant memories for this conversation
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub memories: Vec<Memory>,
    /// Additional context variables
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub variables: HashMap<String, String>,
    /// Previous conversation summary (if truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_summary: Option<String>,
    /// User preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_preferences: Option<UserPreferences>,
}

impl AgentContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a memory
    pub fn with_memory(mut self, memory: Memory) -> Self {
        self.memories.push(memory);
        self
    }

    /// Set memories
    pub fn with_memories(mut self, memories: Vec<Memory>) -> Self {
        self.memories = memories;
        self
    }

    /// Add a variable
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Set conversation summary
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.conversation_summary = Some(summary.into());
        self
    }

    /// Set user preferences
    pub fn with_preferences(mut self, prefs: UserPreferences) -> Self {
        self.user_preferences = Some(prefs);
        self
    }

    /// Check if context has memories
    pub fn has_memories(&self) -> bool {
        !self.memories.is_empty()
    }

    /// Format memories for inclusion in system prompt
    pub fn format_memories(&self) -> String {
        if self.memories.is_empty() {
            return String::new();
        }

        let mut output = String::from("\n\n## Relevant Context\n\n");
        for memory in &self.memories {
            output.push_str(&format!("- {}\n", memory.content));
        }
        output
    }

    /// Format variables for inclusion in system prompt
    pub fn format_variables(&self) -> String {
        if self.variables.is_empty() {
            return String::new();
        }

        let mut output = String::from("\n\n## Context\n\n");
        for (key, value) in &self.variables {
            output.push_str(&format!("- **{key}**: {value}\n"));
        }
        output
    }

    /// Build enhanced system prompt with context
    pub fn enhance_system_prompt(&self, base_prompt: &str) -> String {
        let mut enhanced = base_prompt.to_string();

        if let Some(ref summary) = self.conversation_summary {
            enhanced.push_str(&format!(
                "\n\n## Previous Conversation Summary\n\n{}",
                summary
            ));
        }

        enhanced.push_str(&self.format_memories());
        enhanced.push_str(&self.format_variables());

        enhanced
    }
}

/// A memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Memory ID
    pub id: String,
    /// Memory content
    pub content: String,
    /// Source of the memory (conversation, document, etc.)
    pub source: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
    /// When the memory was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Memory {
    /// Create a new memory
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            source: source.into(),
            relevance: 1.0,
            created_at: chrono::Utc::now(),
        }
    }

    /// Set relevance score
    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance = score.clamp(0.0, 1.0);
        self
    }
}

/// User preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Preferred response style
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_style: Option<String>,
    /// Preferred format (concise, detailed, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_preference: Option<String>,
    /// Technical level preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_level: Option<String>,
    /// Custom preferences
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub custom: HashMap<String, String>,
}

impl UserPreferences {
    /// Create new preferences
    pub fn new() -> Self {
        Self::default()
    }

    /// Set response style
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.response_style = Some(style.into());
        self
    }

    /// Set format preference
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format_preference = Some(format.into());
        self
    }

    /// Set technical level
    pub fn with_technical_level(mut self, level: impl Into<String>) -> Self {
        self.technical_level = Some(level.into());
        self
    }

    /// Add custom preference
    pub fn with_custom(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Format for inclusion in system prompt
    pub fn format(&self) -> String {
        let mut output = String::from("\n\n## User Preferences\n\n");

        if let Some(ref style) = self.response_style {
            output.push_str(&format!("- Response style: {}\n", style));
        }
        if let Some(ref format) = self.format_preference {
            output.push_str(&format!("- Format: {}\n", format));
        }
        if let Some(ref level) = self.technical_level {
            output.push_str(&format!("- Technical level: {}\n", level));
        }
        for (key, value) in &self.custom {
            output.push_str(&format!("- {}: {}\n", key, value));
        }

        output
    }
}

/// Context builder for constructing agent context
pub struct ContextBuilder {
    context: AgentContext,
}

impl ContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            context: AgentContext::new(),
        }
    }

    /// Add a memory
    pub fn memory(mut self, memory: Memory) -> Self {
        self.context.memories.push(memory);
        self
    }

    /// Add a variable
    pub fn variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.variables.insert(key.into(), value.into());
        self
    }

    /// Set summary
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.context.conversation_summary = Some(summary.into());
        self
    }

    /// Set preferences
    pub fn preferences(mut self, prefs: UserPreferences) -> Self {
        self.context.user_preferences = Some(prefs);
        self
    }

    /// Build the context
    pub fn build(self) -> AgentContext {
        self.context
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let context = ContextBuilder::new()
            .memory(Memory::new("1", "User likes Rust", "conversation"))
            .variable("project", "myapp")
            .summary("Previous discussion about programming languages")
            .build();

        assert_eq!(context.memories.len(), 1);
        assert_eq!(context.variables.get("project"), Some(&"myapp".to_string()));
        assert!(context.conversation_summary.is_some());
    }

    #[test]
    fn test_enhance_system_prompt() {
        let context = AgentContext::new()
            .with_memory(Memory::new("1", "Important fact", "source"))
            .with_variable("key", "value");

        let enhanced = context.enhance_system_prompt("You are helpful.");

        assert!(enhanced.contains("You are helpful."));
        assert!(enhanced.contains("Relevant Context"));
        assert!(enhanced.contains("Important fact"));
        assert!(enhanced.contains("Context"));
    }

    #[test]
    fn test_user_preferences() {
        let prefs = UserPreferences::new()
            .with_style("concise")
            .with_technical_level("advanced");

        let formatted = prefs.format();
        assert!(formatted.contains("concise"));
        assert!(formatted.contains("advanced"));
    }
}
