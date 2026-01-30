//! Channel Configuration Schema System
//!
//! Provides unified configuration management for all channels with:
//! - Typed configuration enum for all channel types
//! - JSON schema generation for validation
//! - Configuration validation and defaults
//! - Migration helpers for config versions

use crate::channels::discord::DiscordConfig;
use crate::channels::slack::SlackConfig;
use crate::channels::telegram::TelegramConfig;
use crate::channels::whatsapp::WhatsAppConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Channel type identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Console,
    Discord,
    GoogleChat,
    IMessage,
    Line,
    Matrix,
    Signal,
    Skype,
    Slack,
    Teams,
    #[default]
    Telegram,
    Voice,
    WhatsApp,
    Webhook,
    WebChat,
    Zalo,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Console => write!(f, "console"),
            ChannelType::Discord => write!(f, "discord"),
            ChannelType::GoogleChat => write!(f, "google_chat"),
            ChannelType::IMessage => write!(f, "imessage"),
            ChannelType::Line => write!(f, "line"),
            ChannelType::Matrix => write!(f, "matrix"),
            ChannelType::Signal => write!(f, "signal"),
            ChannelType::Skype => write!(f, "skype"),
            ChannelType::Slack => write!(f, "slack"),
            ChannelType::Teams => write!(f, "teams"),
            ChannelType::Telegram => write!(f, "telegram"),
            ChannelType::Voice => write!(f, "voice"),
            ChannelType::WhatsApp => write!(f, "whatsapp"),
            ChannelType::Webhook => write!(f, "webhook"),
            ChannelType::WebChat => write!(f, "webchat"),
            ChannelType::Zalo => write!(f, "zalo"),
        }
    }
}

/// Unified channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    Console(ConsoleChannelConfig),
    Discord(DiscordConfig),
    GoogleChat(GoogleChatConfig),
    IMessage(IMessageConfig),
    Line(LineConfig),
    Matrix(MatrixConfig),
    Signal(SignalConfig),
    Skype(SkypeConfig),
    Slack(SlackConfig),
    Teams(TeamsConfig),
    Telegram(TelegramConfig),
    Voice(VoiceConfig),
    WhatsApp(WhatsAppConfig),
    Webhook(WebhookConfig),
    WebChat(WebChatConfig),
    Zalo(ZaloConfig),
}

/// Console channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsoleChannelConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_prompt")]
    pub prompt: String,
}

fn default_enabled() -> bool {
    true
}

fn default_prompt() -> String {
    "> ".to_string()
}

/// Google Chat channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleChatConfig {
    pub bot_token: String,
    #[serde(default)]
    pub space: String,
    #[serde(default = "default_thread_key")]
    pub thread_key: Option<String>,
}

fn default_thread_key() -> Option<String> {
    None
}

/// iMessage channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IMessageConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_reply: bool,
    #[serde(default)]
    pub handle_prefix: String,
}

/// Line channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineConfig {
    pub channel_access_token: String,
    pub channel_secret: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Matrix channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatrixConfig {
    pub homeserver: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_room_id")]
    pub room_id: Option<String>,
}

fn default_room_id() -> Option<String> {
    None
}

/// Signal channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    pub phone_number: String,
    #[serde(default)]
    pub captcha: Option<String>,
    #[serde(default)]
    pub device_name: String,
}

/// Skype channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkypeConfig {
    pub app_id: String,
    pub app_secret: String,
    #[serde(default)]
    pub redirect_uri: String,
}

/// Teams channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamsConfig {
    pub app_id: String,
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub webhook_url: Option<String>,
}

/// Voice channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub transcription: bool,
    #[serde(default)]
    pub vad_filter: bool,
}

/// Webhook channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookConfig {
    pub inbound_url: String,
    pub outbound_url: Option<String>,
    #[serde(default = "default_secret")]
    pub secret: Option<String>,
}

fn default_secret() -> Option<String> {
    None
}

/// WebChat channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebChatConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub direct_line_token: String,
    #[serde(default)]
    pub web_socket: bool,
}

/// Zalo channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZaloConfig {
    pub app_id: String,
    pub app_secret: String,
    #[serde(default)]
    pub access_token: Option<String>,
}

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid field value: {0}")]
    InvalidValue(String),

    #[error("Unknown channel type: {0}")]
    UnknownChannelType(String),

    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration validation result
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Channel configuration schema generator
pub struct ChannelSchema;

impl ChannelSchema {
    /// Generate JSON schema for a specific channel type
    pub fn for_type(channel_type: &ChannelType) -> serde_json::Value {
        match channel_type {
            ChannelType::Telegram => Self::telegram_schema(),
            ChannelType::Discord => Self::discord_schema(),
            ChannelType::Slack => Self::slack_schema(),
            ChannelType::WhatsApp => Self::whatsapp_schema(),
            _ => Self::generic_schema(channel_type),
        }
    }

    /// Generate combined schema for all channel types
    pub fn all_channels() -> serde_json::Value {
        let mut channels = HashMap::new();
        for ct in [
            ChannelType::Telegram,
            ChannelType::Discord,
            ChannelType::Slack,
            ChannelType::WhatsApp,
        ] {
            channels.insert(ct.to_string(), Self::for_type(&ct));
        }
        json!({
            "type": "object",
            "properties": {
                "type": {
                    "type": "string",
                    "enum": channels.keys().map(|k| json!(k)).collect::<Vec<_>>()
                },
                "config": {
                    "type": "object",
                    "oneOf": channels.values().collect::<Vec<_>>()
                }
            }
        })
    }

    fn telegram_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["bot_token"],
            "properties": {
                "bot_token": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Bot token from @BotFather"
                },
                "webhook_url": {
                    "type": "string",
                    "format": "uri",
                    "description": "Webhook URL (optional, uses polling if not set)"
                },
                "secret_token": {
                    "type": "string",
                    "description": "Secret token for webhook verification"
                },
                "max_message_length": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 4096,
                    "default": 4096
                },
                "enable_media": {
                    "type": "boolean",
                    "default": true
                },
                "enable_callbacks": {
                    "type": "boolean",
                    "default": true
                }
            }
        })
    }

    fn discord_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["bot_token", "application_id"],
            "properties": {
                "bot_token": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Bot token from Discord Developer Portal"
                },
                "application_id": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Application ID"
                },
                "guild_id": {
                    "type": "string",
                    "description": "Guild ID for commands"
                },
                "enable_commands": {
                    "type": "boolean",
                    "default": true
                },
                "enable_components": {
                    "type": "boolean",
                    "default": true
                },
                "max_message_length": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 2000,
                    "default": 2000
                }
            }
        })
    }

    fn slack_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["bot_token", "signing_secret"],
            "properties": {
                "bot_token": {
                    "type": "string",
                    "pattern": "^xoxb-",
                    "description": "Bot token from Slack App"
                },
                "app_token": {
                    "type": "string",
                    "pattern": "^xapp-",
                    "description": "App level token for socket mode"
                },
                "signing_secret": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Signing secret for verification"
                },
                "socket_mode": {
                    "type": "boolean",
                    "default": false
                },
                "default_channel": {
                    "type": "string",
                    "description": "Default channel to post to"
                },
                "max_message_length": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 4000,
                    "default": 4000
                }
            }
        })
    }

    fn whatsapp_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["account_sid", "auth_token", "from_number"],
            "properties": {
                "account_sid": {
                    "type": "string",
                    "pattern": "^AC[a-f0-9]{32}$",
                    "description": "Twilio Account SID"
                },
                "auth_token": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Twilio Auth Token"
                },
                "from_number": {
                    "type": "string",
                    "pattern": "^whatsapp:\\+[0-9]+$",
                    "description": "Twilio Phone Number (format: whatsapp:+1234567890)"
                },
                "webhook_url": {
                    "type": "string",
                    "format": "uri",
                    "description": "Webhook URL for incoming messages"
                },
                "media_base_url": {
                    "type": "string",
                    "format": "uri",
                    "description": "Media URL base for media messages"
                }
            }
        })
    }

    fn generic_schema(_channel_type: &ChannelType) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "default": true
                }
            },
            "additionalProperties": true
        })
    }
}

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate channel configuration
    pub fn validate(config: &ChannelConfig) -> ConfigResult<()> {
        match config {
            ChannelConfig::Telegram(cfg) => Self::validate_telegram(cfg),
            ChannelConfig::Discord(cfg) => Self::validate_discord(cfg),
            ChannelConfig::Slack(cfg) => Self::validate_slack(cfg),
            ChannelConfig::WhatsApp(cfg) => Self::validate_whatsapp(cfg),
            _ => Ok(()), // Skip validation for other channels for now
        }
    }

    fn validate_telegram(cfg: &TelegramConfig) -> ConfigResult<()> {
        if cfg.bot_token.is_empty() {
            return Err(ConfigError::MissingField("bot_token".to_string()));
        }
        if cfg.bot_token.len() < 10 {
            return Err(ConfigError::InvalidValue(
                "bot_token appears too short".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_discord(cfg: &DiscordConfig) -> ConfigResult<()> {
        if cfg.bot_token.is_empty() {
            return Err(ConfigError::MissingField("bot_token".to_string()));
        }
        if cfg.application_id.is_empty() {
            return Err(ConfigError::MissingField("application_id".to_string()));
        }
        Ok(())
    }

    fn validate_slack(cfg: &SlackConfig) -> ConfigResult<()> {
        if cfg.bot_token.is_empty() {
            return Err(ConfigError::MissingField("bot_token".to_string()));
        }
        if cfg.signing_secret.is_empty() {
            return Err(ConfigError::MissingField("signing_secret".to_string()));
        }
        Ok(())
    }

    fn validate_whatsapp(cfg: &WhatsAppConfig) -> ConfigResult<()> {
        if cfg.account_sid.is_empty() {
            return Err(ConfigError::MissingField("account_sid".to_string()));
        }
        if cfg.auth_token.is_empty() {
            return Err(ConfigError::MissingField("auth_token".to_string()));
        }
        if cfg.from_number.is_empty() {
            return Err(ConfigError::MissingField("from_number".to_string()));
        }
        Ok(())
    }
}

/// Helper to convert legacy config format to unified format
pub fn migrate_config(
    channel_type: &str,
    legacy: &serde_json::Value,
) -> Result<ChannelConfig, String> {
    match channel_type {
        "telegram" => {
            let bot_token = legacy
                .get("bot_token")
                .and_then(|v| v.as_str())
                .ok_or("Missing bot_token")?
                .to_string();
            Ok(ChannelConfig::Telegram(TelegramConfig {
                bot_token,
                webhook_url: legacy
                    .get("webhook_url")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                secret_token: legacy
                    .get("secret_token")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                max_message_length: legacy
                    .get("max_message_length")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096) as usize,
                enable_media: legacy
                    .get("enable_media")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                enable_callbacks: legacy
                    .get("enable_callbacks")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            }))
        }
        "discord" => Ok(ChannelConfig::Discord(DiscordConfig {
            bot_token: legacy
                .get("bot_token")
                .and_then(|v| v.as_str())
                .ok_or("Missing bot_token")?
                .to_string(),
            application_id: legacy
                .get("application_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing application_id")?
                .to_string(),
            guild_id: legacy
                .get("guild_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            enable_commands: legacy
                .get("enable_commands")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            enable_components: legacy
                .get("enable_components")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            max_message_length: legacy
                .get("max_message_length")
                .and_then(|v| v.as_u64())
                .unwrap_or(2000) as usize,
            intents: DiscordConfig::default().intents,
        })),
        _ => Err(format!("Unknown channel type: {}", channel_type)),
    }
}

// Note: DiscordConfig, SlackConfig, TelegramConfig, WhatsAppConfig are available
// from their respective modules: crate::channels::discord, crate::channels::slack, etc.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::discord::DiscordConfig;
    use crate::channels::slack::SlackConfig;
    use crate::channels::telegram::TelegramConfig;
    use crate::channels::whatsapp::WhatsAppConfig;

    #[test]
    fn test_channel_type_display() {
        assert_eq!(ChannelType::Telegram.to_string(), "telegram");
        assert_eq!(ChannelType::Discord.to_string(), "discord");
        assert_eq!(ChannelType::Slack.to_string(), "slack");
    }

    #[test]
    fn test_telegram_config_roundtrip() {
        let config = ChannelConfig::Telegram(TelegramConfig {
            bot_token: "test_token".to_string(),
            webhook_url: Some("https://example.com/webhook".to_string()),
            secret_token: Some("secret".to_string()),
            max_message_length: 4096,
            enable_media: true,
            enable_callbacks: true,
        });

        let json = serde_json::to_string(&config).unwrap();
        let parsed: ChannelConfig = serde_json::from_str(&json).unwrap();

        match parsed {
            ChannelConfig::Telegram(cfg) => {
                assert_eq!(cfg.bot_token, "test_token");
                assert_eq!(cfg.webhook_url.unwrap(), "https://example.com/webhook");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_telegram_validation() {
        let config = ChannelConfig::Telegram(TelegramConfig {
            bot_token: "".to_string(),
            ..Default::default()
        });

        assert!(ConfigValidator::validate(&config).is_err());

        let valid_config = ChannelConfig::Telegram(TelegramConfig {
            bot_token: "valid_token_12345".to_string(),
            ..Default::default()
        });

        assert!(ConfigValidator::validate(&valid_config).is_ok());
    }

    #[test]
    fn test_discord_validation() {
        let config = ChannelConfig::Discord(DiscordConfig {
            bot_token: "".to_string(),
            application_id: "".to_string(),
            ..Default::default()
        });

        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_slack_validation() {
        let config = ChannelConfig::Slack(SlackConfig {
            bot_token: "".to_string(),
            signing_secret: "".to_string(),
            ..Default::default()
        });

        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_whatsapp_validation() {
        let config = ChannelConfig::WhatsApp(WhatsAppConfig {
            account_sid: "".to_string(),
            auth_token: "".to_string(),
            from_number: "".to_string(),
            ..Default::default()
        });

        assert!(ConfigValidator::validate(&config).is_err());
    }

    #[test]
    fn test_schema_generation() {
        let schema = ChannelSchema::for_type(&ChannelType::Telegram);
        assert_eq!(schema["type"], "object");
        assert!(schema["required"].is_array());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("bot_token")));
    }

    #[test]
    fn test_config_migration() {
        let legacy = json!({
            "bot_token": "migrated_token",
            "webhook_url": "https://example.com/webhook",
            "enable_media": false
        });

        let migrated = migrate_config("telegram", &legacy);
        assert!(migrated.is_ok());

        match migrated.unwrap() {
            ChannelConfig::Telegram(cfg) => {
                assert_eq!(cfg.bot_token, "migrated_token");
                assert_eq!(cfg.webhook_url.unwrap(), "https://example.com/webhook");
                assert!(!cfg.enable_media);
            }
            _ => panic!(),
        }
    }
}
