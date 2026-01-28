//! Typed configuration structures
//!
//! Provides strongly-typed access to configuration values with validation
//! and default values.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Metadata about the configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaConfig>,

    /// Environment variables to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    /// Wizard configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wizard: Option<WizardConfig>,

    /// Diagnostics configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticsConfig>,

    /// Logging configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingConfig>,

    /// AI model providers configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<ModelsConfig>,

    /// Node host configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_host: Option<NodeHostConfig>,

    /// Agent definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<HashMap<String, AgentConfig>>,

    /// Tool configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsConfig>,

    /// Key bindings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindings: Option<HashMap<String, String>>,

    /// Broadcast configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcast: Option<BroadcastConfig>,

    /// Audio configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioConfig>,

    /// Media configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaConfig>,

    /// Message handling configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<MessagesConfig>,

    /// Custom commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<HashMap<String, CustomCommand>>,

    /// Approval configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approvals: Option<ApprovalsConfig>,

    /// Session configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionConfig>,

    /// Cron job configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<CronConfig>,

    /// Webhooks configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HooksConfig>,

    /// Web server configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<WebConfig>,

    /// Channel configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<HashMap<String, ChannelConfig>>,

    /// Discovery configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery: Option<DiscoveryConfig>,

    /// Canvas host configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canvas_host: Option<CanvasHostConfig>,

    /// Talk/voice configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub talk: Option<TalkConfig>,

    /// Gateway server configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<GatewayConfig>,

    /// Skills configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<SkillsConfig>,

    /// Plugin configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<PluginsConfig>,
}

impl Config {
    /// Get configuration version
    pub fn version(&self) -> Option<&str> {
        self.meta.as_ref().and_then(|m| m.version.as_deref())
    }

    /// Get default agent configuration
    pub fn default_agent(&self) -> Option<&AgentConfig> {
        self.agents.as_ref().and_then(|a| a.get("default"))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate agents
        if let Some(agents) = &self.agents {
            for (name, agent) in agents {
                if let Err(e) = agent.validate() {
                    errors.push(ValidationError {
                        path: format!("agents.{}", name),
                        message: e,
                    });
                }
            }
        }

        // Validate models
        if let Some(models) = &self.models {
            if let Err(e) = models.validate() {
                errors.push(ValidationError {
                    path: "models".to_string(),
                    message: e,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

/// Metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MetaConfig {
    /// Configuration format version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Configuration description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Wizard configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WizardConfig {
    /// Whether wizard has been completed
    #[serde(default)]
    pub completed: bool,

    /// Wizard steps that have been completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_steps: Option<Vec<String>>,
}

/// Diagnostics configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsConfig {
    /// Enable diagnostic logging
    #[serde(default)]
    pub enabled: bool,

    /// Diagnostic log directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_dir: Option<PathBuf>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,

    /// Log format (json, text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Log destination (stdout, stderr, file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,

    /// Log file path (when destination is file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
}

/// Models configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelsConfig {
    /// Default model provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,

    /// Provider configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<HashMap<String, ModelProviderConfig>>,
}

impl ModelsConfig {
    /// Validate models configuration
    pub fn validate(&self) -> Result<(), String> {
        if let Some(providers) = &self.providers {
            for (name, config) in providers {
                if config.api_key.is_none() && config.api_key_env.is_none() {
                    return Err(format!(
                        "Provider '{}' must have either apiKey or apiKeyEnv",
                        name
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Model provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderConfig {
    /// Provider base URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// API key (direct, not recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Environment variable containing API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,

    /// Default model for this provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    /// Available models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Maximum retries
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_timeout() -> u64 {
    60
}

fn default_retries() -> u32 {
    3
}

/// Node host configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeHostConfig {
    /// Enable node host mode
    #[serde(default)]
    pub enabled: bool,

    /// Node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,

    /// Allowed node commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_commands: Option<Vec<String>>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Agent name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Model to use (provider/model format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Temperature (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens per response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Tools available to this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Whether to enable vision capabilities
    #[serde(default)]
    pub vision: bool,

    /// Memory configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<AgentMemoryConfig>,
}

impl AgentConfig {
    /// Validate agent configuration
    pub fn validate(&self) -> Result<(), String> {
        if let Some(temp) = self.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err("temperature must be between 0.0 and 2.0".to_string());
            }
        }
        Ok(())
    }
}

/// Agent memory configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemoryConfig {
    /// Enable memory for this agent
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Number of recent messages to include in context
    #[serde(default = "default_context_window")]
    pub context_window: usize,

    /// Memory search results to include
    #[serde(default = "default_memory_results")]
    pub search_results: usize,
}

fn default_true() -> bool {
    true
}

fn default_context_window() -> usize {
    20
}

fn default_memory_results() -> usize {
    5
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolsConfig {
    /// Built-in tools configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub built_in: Option<HashMap<String, ToolConfig>>,

    /// External tool directories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<PathBuf>>,

    /// Require approval for destructive tools
    #[serde(default = "default_true")]
    pub require_approval: bool,
}

/// Individual tool configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Enable this tool
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Tool-specific configuration
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Broadcast configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastConfig {
    /// Enable broadcasting
    #[serde(default)]
    pub enabled: bool,

    /// Broadcast channels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
}

/// Audio configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    /// Default TTS provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_provider: Option<String>,

    /// Default TTS voice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_voice: Option<String>,

    /// Audio output device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_device: Option<String>,

    /// Audio input device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_device: Option<String>,
}

/// Media configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaConfig {
    /// Media storage directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_dir: Option<PathBuf>,

    /// Maximum file size in bytes
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Allowed MIME types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_types: Option<Vec<String>>,
}

fn default_max_file_size() -> u64 {
    50 * 1024 * 1024 // 50MB
}

/// Messages configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessagesConfig {
    /// Maximum message length
    #[serde(default = "default_max_message_length")]
    pub max_length: usize,

    /// Enable message threading
    #[serde(default = "default_true")]
    pub threading: bool,
}

fn default_max_message_length() -> usize {
    4000
}

/// Custom command definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomCommand {
    /// Command description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Command to execute
    pub command: String,

    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,

    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    /// Timeout in seconds
    #[serde(default = "default_command_timeout")]
    pub timeout_seconds: u64,
}

fn default_command_timeout() -> u64 {
    30
}

/// Approvals configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalsConfig {
    /// Require approval for bash commands
    #[serde(default = "default_true")]
    pub bash: bool,

    /// Require approval for file writes
    #[serde(default = "default_true")]
    pub file_write: bool,

    /// Require approval for external tool execution
    #[serde(default)]
    pub external_tools: bool,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    /// Session storage directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_dir: Option<PathBuf>,

    /// Maximum sessions to keep
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,

    /// Session retention days
    #[serde(default = "default_session_retention")]
    pub retention_days: u32,
}

fn default_max_sessions() -> usize {
    100
}

fn default_session_retention() -> u32 {
    30
}

/// Cron configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CronConfig {
    /// Enable cron jobs
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Cron job definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jobs: Option<HashMap<String, CronJob>>,
}

/// Cron job definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
    /// Job schedule (cron expression)
    pub schedule: String,

    /// Job command or agent message
    pub command: String,

    /// Job timezone
    #[serde(default = "default_timezone")]
    pub timezone: String,

    /// Enable this job
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Hooks configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HooksConfig {
    /// Enable webhooks
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Hook definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definitions: Option<HashMap<String, HookDefinition>>,
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HookDefinition {
    /// Hook path
    pub path: String,

    /// Handler command or agent
    pub handler: String,

    /// HTTP method
    #[serde(default = "default_hook_method")]
    pub method: String,

    /// Authentication token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

fn default_hook_method() -> String {
    "POST".to_string()
}

/// Web server configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebConfig {
    /// Bind address
    #[serde(default = "default_bind_address")]
    pub bind: String,

    /// Enable TLS
    #[serde(default)]
    pub tls: bool,

    /// TLS certificate path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_path: Option<PathBuf>,

    /// TLS key path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<PathBuf>,

    /// CORS origins
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors_origins: Option<Vec<String>>,
}

fn default_bind_address() -> String {
    "127.0.0.1:8080".to_string()
}

/// Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConfig {
    /// Channel type (telegram, discord, slack, etc.)
    #[serde(rename = "type")]
    pub channel_type: String,

    /// Enable this channel
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Channel-specific configuration
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryConfig {
    /// Enable service discovery
    #[serde(default)]
    pub enabled: bool,

    /// Discovery methods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methods: Option<Vec<String>>,
}

/// Canvas host configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CanvasHostConfig {
    /// Enable canvas host
    #[serde(default)]
    pub enabled: bool,

    /// Canvas port
    #[serde(default = "default_canvas_port")]
    pub port: u16,
}

fn default_canvas_port() -> u16 {
    8081
}

/// Talk configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TalkConfig {
    /// Enable talk mode
    #[serde(default)]
    pub enabled: bool,

    /// Voice wake keyword
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_keyword: Option<String>,

    /// Push-to-talk key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ptt_key: Option<String>,
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    /// Gateway bind address
    #[serde(default = "default_gateway_bind")]
    pub bind: String,

    /// Enable device pairing
    #[serde(default = "default_true")]
    pub pairing_enabled: bool,

    /// Authentication token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

fn default_gateway_bind() -> String {
    "127.0.0.1:8080".to_string()
}

/// Skills configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SkillsConfig {
    /// Skills directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<PathBuf>,

    /// Auto-discover skills
    #[serde(default = "default_true")]
    pub auto_discover: bool,

    /// Enabled skills
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Vec<String>>,
}

/// Plugins configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginsConfig {
    /// Plugins directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<PathBuf>,

    /// Enabled plugins
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Vec<String>>,

    /// Plugin configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
}
