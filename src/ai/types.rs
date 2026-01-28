//! AI Provider Types
//!
//! Common types for AI provider interactions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of a message in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    /// Get the role as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "content")]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Array of content parts (for multimodal)
    Parts(Vec<ContentPart>),
    /// Tool call result content
    ToolResult { tool_call_id: String, content: String },
}

impl MessageContent {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        MessageContent::Text(text.into())
    }

    /// Get as text if it's a text variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(t) => Some(t),
            _ => None,
        }
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

/// Individual content part (for multimodal messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ContentPart {
    /// Text content part
    Text { text: String },
    /// Image content part
    Image { source: ImageSource },
}

/// Image source for vision models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ImageSource {
    /// Base64 encoded image
    Base64 { media_type: String, data: String },
    /// Image URL
    Url { url: String },
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    #[serde(flatten)]
    pub content: MessageContent,
    /// Optional name for the message sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }
}

/// Tool/function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool type (usually "function")
    pub r#type: String,
    /// Function definition
    pub function: Function,
}

impl Tool {
    /// Create a new function tool
    pub fn function(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            r#type: "function".to_string(),
            function: Function {
                name: name.into(),
                description: description.into(),
                parameters: serde_json::Value::Object(serde_json::Map::new()),
            },
        }
    }

    /// Add parameters schema
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.function.parameters = params;
        self
    }
}

/// Function definition for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
}

/// Tool call in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Tool type (usually "function")
    pub r#type: String,
    /// Function call details
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
    /// Arguments as JSON string
    pub arguments: String,
}

impl FunctionCall {
    /// Parse the arguments as JSON
    pub fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> crate::ai::Result<T> {
        Ok(serde_json::from_str(&self.arguments)?)
    }
}

/// Tool choice setting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Let the model decide
    Auto,
    /// Force the model to use a tool
    Required,
    /// Don't use tools
    None,
    /// Force specific tool
    #[serde(rename = "function")]
    Function { name: String },
}

/// Response format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Default text response
    Text,
    /// JSON object response
    JsonObject,
    /// JSON with schema
    JsonSchema { schema: serde_json::Value },
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Sampling temperature (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Nucleus sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Tool choice setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Response format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl ChatRequest {
    /// Create a new chat request
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            tools: None,
            tool_choice: None,
            stream: false,
            response_format: None,
        }
    }

    /// Add a message
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }
}

/// Chat completion response
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// Response ID
    pub id: String,
    /// Object type
    pub object: String,
    /// Creation timestamp
    pub created: u64,
    /// Model used
    pub model: String,
    /// Response choices
    pub choices: Vec<Choice>,
    /// Usage statistics
    pub usage: Usage,
}

/// A choice in a chat response
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    /// Choice index
    pub index: u32,
    /// The message
    pub message: AssistantMessage,
    /// Finish reason
    pub finish_reason: Option<FinishReason>,
}

/// Assistant message in a response
#[derive(Debug, Clone, Deserialize)]
pub struct AssistantMessage {
    /// Role (always "assistant")
    pub role: String,
    /// Content (may be null if tool_calls present)
    #[serde(default)]
    pub content: Option<String>,
    /// Tool calls if any
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Finish reason for a generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Completed normally
    Stop,
    /// Hit length limit
    Length,
    /// Tool calls were made
    ToolCalls,
    /// Content filter triggered
    ContentFilter,
    /// Other reason
    Other,
}

/// Token usage statistics
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Usage {
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

/// Streaming chat completion chunk
#[derive(Debug, Clone, Deserialize)]
pub struct ChatChunk {
    /// Chunk ID
    pub id: String,
    /// Object type
    pub object: String,
    /// Creation timestamp
    pub created: u64,
    /// Model used
    pub model: String,
    /// Choice deltas
    pub choices: Vec<ChoiceDelta>,
}

/// Delta in a streaming choice
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChoiceDelta {
    /// Choice index
    pub index: u32,
    /// Delta content
    pub delta: DeltaContent,
    /// Finish reason (if complete)
    pub finish_reason: Option<FinishReason>,
}

/// Content delta in streaming response
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeltaContent {
    /// Role (only in first chunk)
    pub role: Option<String>,
    /// Content text (may be null)
    pub content: Option<String>,
    /// Tool calls delta
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Tool call delta for streaming
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolCallDelta {
    /// Tool call index
    pub index: u32,
    /// Tool call ID
    pub id: Option<String>,
    /// Tool type
    pub r#type: Option<String>,
    /// Function delta
    pub function: Option<FunctionCallDelta>,
}

/// Function call delta
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FunctionCallDelta {
    /// Function name (may be partial)
    pub name: Option<String>,
    /// Arguments (may be partial)
    pub arguments: Option<String>,
}

/// Model information
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    /// Model ID
    pub id: String,
    /// Model name/display name
    pub name: String,
    /// Provider name
    pub provider: String,
    /// Context window size
    pub context_window: Option<u32>,
    /// Maximum output tokens
    pub max_output_tokens: Option<u32>,
    /// Supports vision
    pub supports_vision: bool,
    /// Supports tool calling
    pub supports_tools: bool,
    /// Pricing per 1K tokens (input, output)
    pub pricing: Option<(f64, f64)>,
}

/// Provider configuration for different AI providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderSettings {
    /// Provider name
    pub name: String,
    /// API key
    pub api_key: Option<String>,
    /// Base URL
    pub base_url: Option<String>,
    /// Default model
    pub default_model: Option<String>,
    /// Available models
    pub models: Vec<ModelInfo>,
}

/// Conversation thread for multi-turn chats
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Model used
    pub model: Option<String>,
    /// System prompt
    pub system_prompt: Option<String>,
}

impl Conversation {
    /// Create a new conversation
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the system prompt
    pub fn with_system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a user message
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Add an assistant message
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// Build messages including system prompt
    pub fn build_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();
        if let Some(ref system) = self.system_prompt {
            messages.push(Message::system(system.clone()));
        }
        messages.extend(self.messages.clone());
        messages
    }

    /// Get the message count (excluding system)
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content.as_text(), Some("Hello"));
    }

    #[test]
    fn test_conversation() {
        let conv = Conversation::new()
            .with_system("You are helpful")
            .with_model("gpt-4")
            .user("Hi")
            .assistant("Hello!");

        assert_eq!(conv.message_count(), 2);
        let messages = conv.build_messages();
        assert_eq!(messages.len(), 3); // Including system
    }

    #[test]
    fn test_tool_creation() {
        let tool = Tool::function(
            "get_weather",
            "Get the weather for a location"
        ).with_parameters(serde_json::json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            }
        }));

        assert_eq!(tool.function.name, "get_weather");
    }

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new("gpt-4")
            .add_message(Message::system("System"))
            .add_message(Message::user("Hello"));

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
    }
}
