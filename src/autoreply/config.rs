//! Auto-reply Configuration
//!
//! Configuration for automated responses based on message triggers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trigger type for auto-reply rules
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Match exact text
    Exact(String),
    /// Match text containing substring (case-insensitive)
    Contains(String),
    /// Match text starting with prefix
    StartsWith(String),
    /// Match text ending with suffix
    EndsWith(String),
    /// Match using regex pattern
    Regex(String),
    /// Match any message (catch-all)
    Any,
    /// Match when user joins chat
    Join,
    /// Match when user leaves chat
    Leave,
    /// Match specific command (starts with / or !)
    Command(String),
}

/// Response type for auto-reply rules
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    /// Static text response
    Text(String),
    /// Random selection from list
    Random(Vec<String>),
    /// Dynamic response with template variables
    Template(String),
}

/// Action to take when rule matches
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Send a reply message
    #[default]
    Reply,
    /// React to the message (if channel supports it)
    React(String),
    /// Delete the message (if channel supports it)
    Delete,
    /// Kick/ban user (if channel supports it)
    Kick { reason: Option<String> },
    /// Forward to another channel
    Forward { channel_id: String },
}

/// Auto-reply rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplyRule {
    /// Rule ID (unique identifier)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Whether rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Trigger condition
    pub trigger: TriggerType,
    /// Response to send
    pub response: ResponseType,
    /// Action to take
    #[serde(default)]
    pub action: ActionType,
    /// Channels this rule applies to (empty = all)
    #[serde(default)]
    pub channels: Vec<String>,
    /// Users this rule applies to (empty = all)
    #[serde(default)]
    pub users: Vec<String>,
    /// Users to exclude from this rule
    #[serde(default)]
    pub exclude_users: Vec<String>,
    /// Cooldown period in seconds (0 = no cooldown)
    #[serde(default)]
    pub cooldown_seconds: u64,
    /// Maximum uses per user (0 = unlimited)
    #[serde(default)]
    pub max_uses_per_user: u32,
    /// Priority (higher = checked first)
    #[serde(default)]
    pub priority: i32,
    /// Whether to stop processing other rules after this matches
    #[serde(default)]
    pub stop_processing: bool,
    /// Template variables (key -> value)
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Reply to specific message (quote)
    #[serde(default)]
    pub reply_to_message: bool,
    /// Mentions to include in reply
    #[serde(default)]
    pub mentions: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl AutoReplyRule {
    /// Create a new auto-reply rule
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        trigger: TriggerType,
        response: ResponseType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled: true,
            trigger,
            response,
            action: ActionType::Reply,
            channels: Vec::new(),
            users: Vec::new(),
            exclude_users: Vec::new(),
            cooldown_seconds: 0,
            max_uses_per_user: 0,
            priority: 0,
            stop_processing: false,
            variables: HashMap::new(),
            reply_to_message: true,
            mentions: Vec::new(),
        }
    }

    /// Set channels filter
    pub fn with_channels(mut self, channels: Vec<String>) -> Self {
        self.channels = channels;
        self
    }

    /// Set users filter
    pub fn with_users(mut self, users: Vec<String>) -> Self {
        self.users = users;
        self
    }

    /// Set excluded users
    pub fn with_exclude_users(mut self, users: Vec<String>) -> Self {
        self.exclude_users = users;
        self
    }

    /// Set cooldown
    pub fn with_cooldown(mut self, seconds: u64) -> Self {
        self.cooldown_seconds = seconds;
        self
    }

    /// Set max uses per user
    pub fn with_max_uses(mut self, max: u32) -> Self {
        self.max_uses_per_user = max;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set stop processing flag
    pub fn with_stop_processing(mut self, stop: bool) -> Self {
        self.stop_processing = stop;
        self
    }

    /// Set action type
    pub fn with_action(mut self, action: ActionType) -> Self {
        self.action = action;
        self
    }

    /// Add a variable
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Check if rule applies to a channel
    pub fn applies_to_channel(&self, channel_id: &str) -> bool {
        self.channels.is_empty() || self.channels.iter().any(|c| c == channel_id)
    }

    /// Check if rule applies to a user
    pub fn applies_to_user(&self, user_id: &str) -> bool {
        if self.exclude_users.iter().any(|u| u == user_id) {
            return false;
        }
        self.users.is_empty() || self.users.iter().any(|u| u == user_id)
    }
}

/// Auto-reply configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplyConfig {
    /// Global enable/disable
    #[serde(default)]
    pub enabled: bool,
    /// List of auto-reply rules
    #[serde(default)]
    pub rules: Vec<AutoReplyRule>,
    /// Default cooldown for all rules (seconds)
    #[serde(default)]
    pub default_cooldown_seconds: u64,
    /// Maximum rules to process per message (prevents infinite loops)
    #[serde(default = "default_max_rules")]
    pub max_rules_per_message: u32,
    /// Log matches for analytics
    #[serde(default)]
    pub log_matches: bool,
}

fn default_max_rules() -> u32 {
    10
}

impl Default for AutoReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: Vec::new(),
            default_cooldown_seconds: 0,
            max_rules_per_message: 10,
            log_matches: false,
        }
    }
}

impl AutoReplyConfig {
    /// Create a new auto-reply config
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable auto-reply
    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: AutoReplyRule) {
        self.rules.push(rule);
        // Sort by priority (highest first)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove a rule by ID
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != rule_id);
        self.rules.len() < before
    }

    /// Get a rule by ID
    pub fn get_rule(&self, rule_id: &str) -> Option<&AutoReplyRule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    /// Get mutable reference to rule
    pub fn get_rule_mut(&mut self, rule_id: &str) -> Option<&mut AutoReplyRule> {
        self.rules.iter_mut().find(|r| r.id == rule_id)
    }

    /// Enable a rule
    pub fn enable_rule(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.get_rule_mut(rule_id) {
            rule.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a rule
    pub fn disable_rule(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.get_rule_mut(rule_id) {
            rule.enabled = false;
            true
        } else {
            false
        }
    }

    /// Get enabled rules sorted by priority
    pub fn enabled_rules(&self) -> Vec<&AutoReplyRule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.max_rules_per_message == 0 {
            return Err("max_rules_per_message must be greater than 0".to_string());
        }

        // Check for duplicate rule IDs
        let mut seen_ids = std::collections::HashSet::new();
        for rule in &self.rules {
            if !seen_ids.insert(&rule.id) {
                return Err(format!("Duplicate rule ID: {}", rule.id));
            }

            // Validate regex patterns
            if let TriggerType::Regex(pattern) = &rule.trigger {
                if let Err(e) = regex::Regex::new(pattern) {
                    return Err(format!("Invalid regex pattern in rule {}: {}", rule.id, e));
                }
            }

            // Validate random responses aren't empty
            if let ResponseType::Random(options) = &rule.response {
                if options.is_empty() {
                    return Err(format!(
                        "Random response in rule {} has no options",
                        rule.id
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_reply_rule_builder() {
        let rule = AutoReplyRule::new(
            "welcome",
            "Welcome Message",
            TriggerType::Contains("hello".to_string()),
            ResponseType::Text("Hello there!".to_string()),
        )
        .with_channels(vec!["telegram".to_string()])
        .with_cooldown(60)
        .with_priority(10)
        .with_stop_processing(true);

        assert_eq!(rule.id, "welcome");
        assert_eq!(rule.name, "Welcome Message");
        assert!(rule.enabled);
        assert_eq!(rule.channels, vec!["telegram"]);
        assert_eq!(rule.cooldown_seconds, 60);
        assert_eq!(rule.priority, 10);
        assert!(rule.stop_processing);
    }

    #[test]
    fn test_rule_applies_to_channel() {
        let rule = AutoReplyRule::new(
            "test",
            "Test",
            TriggerType::Any,
            ResponseType::Text("Hi".to_string()),
        )
        .with_channels(vec!["telegram".to_string(), "discord".to_string()]);

        assert!(rule.applies_to_channel("telegram"));
        assert!(rule.applies_to_channel("discord"));
        assert!(!rule.applies_to_channel("slack"));
    }

    #[test]
    fn test_rule_applies_to_all_channels_when_empty() {
        let rule = AutoReplyRule::new(
            "test",
            "Test",
            TriggerType::Any,
            ResponseType::Text("Hi".to_string()),
        );

        assert!(rule.applies_to_channel("any-channel"));
    }

    #[test]
    fn test_rule_excludes_users() {
        let rule = AutoReplyRule::new(
            "test",
            "Test",
            TriggerType::Any,
            ResponseType::Text("Hi".to_string()),
        )
        .with_exclude_users(vec!["spammer".to_string()]);

        assert!(rule.applies_to_user("normal_user"));
        assert!(!rule.applies_to_user("spammer"));
    }

    #[test]
    fn test_config_add_and_remove_rule() {
        let mut config = AutoReplyConfig::new();
        let rule = AutoReplyRule::new(
            "test",
            "Test Rule",
            TriggerType::Exact("ping".to_string()),
            ResponseType::Text("pong".to_string()),
        );

        config.add_rule(rule);
        assert_eq!(config.rules.len(), 1);
        assert!(config.get_rule("test").is_some());

        assert!(config.remove_rule("test"));
        assert!(config.get_rule("test").is_none());
        assert!(!config.remove_rule("nonexistent"));
    }

    #[test]
    fn test_config_rules_sorted_by_priority() {
        let mut config = AutoReplyConfig::new();

        config.add_rule(
            AutoReplyRule::new(
                "low",
                "Low Priority",
                TriggerType::Any,
                ResponseType::Text("Low".to_string()),
            )
            .with_priority(1),
        );

        config.add_rule(
            AutoReplyRule::new(
                "high",
                "High Priority",
                TriggerType::Any,
                ResponseType::Text("High".to_string()),
            )
            .with_priority(10),
        );

        config.add_rule(
            AutoReplyRule::new(
                "medium",
                "Medium Priority",
                TriggerType::Any,
                ResponseType::Text("Medium".to_string()),
            )
            .with_priority(5),
        );

        let enabled = config.enabled_rules();
        assert_eq!(enabled[0].id, "high");
        assert_eq!(enabled[1].id, "medium");
        assert_eq!(enabled[2].id, "low");
    }

    #[test]
    fn test_config_enable_disable_rule() {
        let mut config = AutoReplyConfig::new();
        config.add_rule(AutoReplyRule::new(
            "test",
            "Test",
            TriggerType::Any,
            ResponseType::Text("Hi".to_string()),
        ));

        assert!(config.disable_rule("test"));
        assert!(!config.get_rule("test").unwrap().enabled);

        assert!(config.enable_rule("test"));
        assert!(config.get_rule("test").unwrap().enabled);

        assert!(!config.disable_rule("nonexistent"));
    }

    #[test]
    fn test_config_validate_duplicate_ids() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(AutoReplyRule::new(
            "duplicate",
            "First",
            TriggerType::Any,
            ResponseType::Text("First".to_string()),
        ));
        config.add_rule(AutoReplyRule::new(
            "duplicate",
            "Second",
            TriggerType::Any,
            ResponseType::Text("Second".to_string()),
        ));

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate rule ID"));
    }

    #[test]
    fn test_config_validate_invalid_regex() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(AutoReplyRule::new(
            "bad-regex",
            "Bad Regex",
            TriggerType::Regex("[invalid".to_string()),
            ResponseType::Text("Hi".to_string()),
        ));

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid regex"));
    }

    #[test]
    fn test_config_validate_empty_random() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(AutoReplyRule::new(
            "empty-random",
            "Empty Random",
            TriggerType::Any,
            ResponseType::Random(vec![]),
        ));

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no options"));
    }

    #[test]
    fn test_trigger_type_serialization() {
        let trigger = TriggerType::Contains("hello".to_string());
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("contains"));

        let parsed: TriggerType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, trigger);
    }

    #[test]
    fn test_response_type_serialization() {
        let response = ResponseType::Random(vec!["A".to_string(), "B".to_string()]);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("random"));

        let parsed: ResponseType = serde_json::from_str(&json).unwrap();
        match parsed {
            ResponseType::Random(options) => {
                assert_eq!(options.len(), 2);
                assert_eq!(options[0], "A");
            }
            _ => panic!("Expected Random response"),
        }
    }

    #[test]
    fn test_action_type_serialization() {
        let action = ActionType::Kick {
            reason: Some("Spam".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("kick"));
        assert!(json.contains("Spam"));

        let parsed: ActionType = serde_json::from_str(&json).unwrap();
        match parsed {
            ActionType::Kick { reason } => assert_eq!(reason, Some("Spam".to_string())),
            _ => panic!("Expected Kick action"),
        }
    }
}
