//! Auto-reply Engine
//!
//! Handles matching incoming messages against auto-reply rules and generating responses.

use super::config::{ActionType, AutoReplyConfig, AutoReplyRule, ResponseType, TriggerType};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Match result from checking a message against rules
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The rule that matched
    pub rule: AutoReplyRule,
    /// The generated response text
    pub response_text: String,
    /// Variables extracted from the match
    pub variables: HashMap<String, String>,
}

/// Context for matching a message
#[derive(Debug, Clone)]
pub struct MatchContext {
    /// Channel ID where message was received
    pub channel_id: String,
    /// User ID who sent the message
    pub user_id: String,
    /// Username of sender
    pub username: String,
    /// Message text content
    pub message_text: String,
    /// Message ID (for replying)
    pub message_id: Option<String>,
    /// Chat/Thread ID
    pub chat_id: Option<String>,
    /// Whether this is a private/direct message
    pub is_private: bool,
    /// Extra metadata from the channel
    pub extra: Option<serde_json::Value>,
}

impl MatchContext {
    /// Create a new match context
    pub fn new(
        channel_id: impl Into<String>,
        user_id: impl Into<String>,
        message_text: impl Into<String>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            user_id: user_id.into(),
            username: String::new(),
            message_text: message_text.into(),
            message_id: None,
            chat_id: None,
            is_private: false,
            extra: None,
        }
    }

    /// Set username
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    /// Set message ID
    pub fn with_message_id(mut self, id: impl Into<String>) -> Self {
        self.message_id = Some(id.into());
        self
    }

    /// Set chat ID
    pub fn with_chat_id(mut self, id: impl Into<String>) -> Self {
        self.chat_id = Some(id.into());
        self
    }

    /// Set private flag
    pub fn is_private(mut self, private: bool) -> Self {
        self.is_private = private;
        self
    }
}

/// Cooldown tracking for rules
#[derive(Debug)]
struct CooldownEntry {
    last_triggered: Instant,
    use_count: u32,
}

/// Auto-reply engine that processes messages against rules
#[derive(Debug)]
pub struct AutoReplyEngine {
    config: RwLock<AutoReplyConfig>,
    /// Track cooldowns per rule per user (rule_id -> user_id -> entry)
    cooldowns: RwLock<HashMap<String, HashMap<String, CooldownEntry>>>,
    /// Track global match log for analytics
    match_log: RwLock<Vec<MatchLogEntry>>,
}

/// Log entry for match analytics
#[derive(Debug, Clone)]
struct MatchLogEntry {
    timestamp: Instant,
    rule_id: String,
    channel_id: String,
    user_id: String,
    matched_text: String,
}

impl AutoReplyEngine {
    /// Create a new auto-reply engine
    pub fn new(config: AutoReplyConfig) -> Self {
        Self {
            config: RwLock::new(config),
            cooldowns: RwLock::new(HashMap::new()),
            match_log: RwLock::new(Vec::new()),
        }
    }

    /// Update the configuration
    pub fn update_config(&self, config: AutoReplyConfig) {
        let mut cfg = self.config.write();
        *cfg = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> AutoReplyConfig {
        self.config.read().clone()
    }

    /// Process a message and return matches
    pub fn process_message(&self, ctx: &MatchContext) -> Vec<MatchResult> {
        let config = self.config.read();

        if !config.enabled {
            return Vec::new();
        }

        let mut results = Vec::new();
        let mut rules_processed = 0;

        for rule in config.enabled_rules() {
            // Check max rules limit
            if rules_processed >= config.max_rules_per_message {
                break;
            }

            // Check if rule applies to this channel
            if !rule.applies_to_channel(&ctx.channel_id) {
                continue;
            }

            // Check if rule applies to this user
            if !rule.applies_to_user(&ctx.user_id) {
                continue;
            }

            // Check cooldown
            if self.is_on_cooldown(rule, &ctx.user_id) {
                continue;
            }

            // Check max uses per user
            if rule.max_uses_per_user > 0 && self.has_exceeded_max_uses(rule, &ctx.user_id) {
                continue;
            }

            // Try to match the trigger
            if let Some(variables) = self.match_trigger(&rule.trigger, ctx) {
                // Generate response
                if let Some(response_text) = self.generate_response(&rule.response, ctx, &variables) {
                    // Record cooldown
                    self.record_trigger(rule, &ctx.user_id);

                    // Log match if enabled
                    if config.log_matches {
                        self.log_match(rule, ctx);
                    }

                    results.push(MatchResult {
                        rule: rule.clone(),
                        response_text,
                        variables,
                    });

                    rules_processed += 1;

                    // Stop processing if rule says so
                    if rule.stop_processing {
                        break;
                    }
                }
            }
        }

        results
    }

    /// Check if a single rule matches (for testing/debugging)
    pub fn test_rule(&self, rule: &AutoReplyRule, ctx: &MatchContext) -> Option<MatchResult> {
        if !rule.enabled {
            return None;
        }

        if !rule.applies_to_channel(&ctx.channel_id) {
            return None;
        }

        if !rule.applies_to_user(&ctx.user_id) {
            return None;
        }

        let variables = self.match_trigger(&rule.trigger, ctx)?;
        let response_text = self.generate_response(&rule.response, ctx, &variables)?;

        Some(MatchResult {
            rule: rule.clone(),
            response_text,
            variables,
        })
    }

    /// Match a trigger against the message context
    fn match_trigger(
        &self,
        trigger: &TriggerType,
        ctx: &MatchContext,
    ) -> Option<HashMap<String, String>> {
        let text = ctx.message_text.trim();
        let mut variables = HashMap::new();

        match trigger {
            TriggerType::Exact(pattern) => {
                if text.eq_ignore_ascii_case(pattern) {
                    variables.insert("matched_text".to_string(), text.to_string());
                    Some(variables)
                } else {
                    None
                }
            }
            TriggerType::Contains(pattern) => {
                if text.to_lowercase().contains(&pattern.to_lowercase()) {
                    variables.insert("matched_text".to_string(), text.to_string());
                    Some(variables)
                } else {
                    None
                }
            }
            TriggerType::StartsWith(prefix) => {
                if text.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    let rest = text.chars().skip(prefix.len()).collect::<String>();
                    variables.insert("matched_text".to_string(), text.to_string());
                    variables.insert("rest".to_string(), rest);
                    Some(variables)
                } else {
                    None
                }
            }
            TriggerType::EndsWith(suffix) => {
                if text.to_lowercase().ends_with(&suffix.to_lowercase()) {
                    let len = text.len() - suffix.len();
                    let rest = text.chars().take(len).collect::<String>();
                    variables.insert("matched_text".to_string(), text.to_string());
                    variables.insert("rest".to_string(), rest);
                    Some(variables)
                } else {
                    None
                }
            }
            TriggerType::Regex(pattern) => {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if let Some(captures) = regex.captures(text) {
                        variables.insert("matched_text".to_string(), text.to_string());

                        // Add named captures
                        for name in regex.capture_names().flatten() {
                            if let Some(m) = captures.name(name) {
                                variables.insert(name.to_string(), m.as_str().to_string());
                            }
                        }

                        // Add numbered captures
                        for (i, cap) in captures.iter().enumerate() {
                            if let Some(m) = cap {
                                variables.insert(format!("group_{}", i), m.as_str().to_string());
                            }
                        }

                        Some(variables)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            TriggerType::Any => {
                variables.insert("matched_text".to_string(), text.to_string());
                Some(variables)
            }
            TriggerType::Join => None, // Handled at channel level
            TriggerType::Leave => None, // Handled at channel level
            TriggerType::Command(cmd) => {
                let prefixes = ["/", "!"];
                for prefix in &prefixes {
                    let full_cmd = format!("{}{}", prefix, cmd);
                    if text.to_lowercase().starts_with(&full_cmd.to_lowercase()) {
                        let rest = text.chars().skip(full_cmd.len()).collect::<String>().trim().to_string();
                        variables.insert("command".to_string(), cmd.to_string());
                        variables.insert("args".to_string(), rest);
                        variables.insert("matched_text".to_string(), text.to_string());
                        return Some(variables);
                    }
                }
                None
            }
        }
    }

    /// Generate response text based on response type
    fn generate_response(
        &self,
        response: &ResponseType,
        ctx: &MatchContext,
        variables: &HashMap<String, String>,
    ) -> Option<String> {
        let text = match response {
            ResponseType::Text(text) => text.clone(),
            ResponseType::Random(options) => {
                if options.is_empty() {
                    return None;
                }
                // Simple random selection
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                ctx.message_text.hash(&mut hasher);
                ctx.user_id.hash(&mut hasher);
                let index = hasher.finish() as usize % options.len();
                options[index].clone()
            }
            ResponseType::Template(template) => template.clone(),
        };

        // Process template variables
        Some(self.process_template(&text, ctx, variables))
    }

    /// Process template string with variables
    fn process_template(
        &self,
        template: &str,
        ctx: &MatchContext,
        match_vars: &HashMap<String, String>,
    ) -> String {
        let mut result = template.to_string();

        // Replace built-in variables
        result = result.replace("{{user_id}}", &ctx.user_id);
        result = result.replace("{{username}}", &ctx.username);
        result = result.replace("{{channel_id}}", &ctx.channel_id);
        result = result.replace("{{message_text}}", &ctx.message_text);

        if let Some(chat_id) = &ctx.chat_id {
            result = result.replace("{{chat_id}}", chat_id);
        }

        if let Some(msg_id) = &ctx.message_id {
            result = result.replace("{{message_id}}", msg_id);
        }

        // Replace match variables
        for (key, value) in match_vars {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }

        result
    }

    /// Check if rule is on cooldown for user
    fn is_on_cooldown(&self, rule: &AutoReplyRule, user_id: &str) -> bool {
        if rule.cooldown_seconds == 0 {
            return false;
        }

        let cooldowns = self.cooldowns.read();
        if let Some(user_cooldowns) = cooldowns.get(&rule.id) {
            if let Some(entry) = user_cooldowns.get(user_id) {
                let elapsed = entry.last_triggered.elapsed();
                return elapsed < Duration::from_secs(rule.cooldown_seconds);
            }
        }
        false
    }

    /// Check if user has exceeded max uses
    fn has_exceeded_max_uses(&self, rule: &AutoReplyRule, user_id: &str) -> bool {
        let cooldowns = self.cooldowns.read();
        if let Some(user_cooldowns) = cooldowns.get(&rule.id) {
            if let Some(entry) = user_cooldowns.get(user_id) {
                return entry.use_count >= rule.max_uses_per_user;
            }
        }
        false
    }

    /// Record a trigger for cooldown tracking
    fn record_trigger(&self, rule: &AutoReplyRule, user_id: &str) {
        let mut cooldowns = self.cooldowns.write();
        let user_cooldowns = cooldowns.entry(rule.id.clone()).or_default();

        let entry = user_cooldowns.entry(user_id.to_string()).or_insert(CooldownEntry {
            last_triggered: Instant::now(),
            use_count: 0,
        });

        entry.last_triggered = Instant::now();
        entry.use_count += 1;
    }

    /// Log a match for analytics
    fn log_match(&self, rule: &AutoReplyRule, ctx: &MatchContext) {
        let mut log = self.match_log.write();
        log.push(MatchLogEntry {
            timestamp: Instant::now(),
            rule_id: rule.id.clone(),
            channel_id: ctx.channel_id.clone(),
            user_id: ctx.user_id.clone(),
            matched_text: ctx.message_text.clone(),
        });

        // Keep only last 10000 entries
        if log.len() > 10000 {
            log.remove(0);
        }
    }

    /// Clear cooldowns for a specific rule
    pub fn clear_cooldowns(&self, rule_id: &str) {
        let mut cooldowns = self.cooldowns.write();
        cooldowns.remove(rule_id);
    }

    /// Clear all cooldowns
    pub fn clear_all_cooldowns(&self) {
        let mut cooldowns = self.cooldowns.write();
        cooldowns.clear();
    }

    /// Get match statistics for a rule
    pub fn get_rule_stats(&self, rule_id: &str) -> RuleStats {
        let cooldowns = self.cooldowns.read();
        let log = self.match_log.read();

        let trigger_count = cooldowns
            .get(rule_id)
            .map(|m| m.values().map(|e| e.use_count).sum())
            .unwrap_or(0);

        let recent_matches = log
            .iter()
            .filter(|e| e.rule_id == rule_id)
            .count();

        RuleStats {
            trigger_count,
            recent_matches,
        }
    }
}

/// Statistics for a rule
#[derive(Debug, Clone)]
pub struct RuleStats {
    /// Total trigger count
    pub trigger_count: u32,
    /// Recent matches in log
    pub recent_matches: usize,
}

/// Create a shared auto-reply engine
pub fn create_engine(config: AutoReplyConfig) -> Arc<AutoReplyEngine> {
    Arc::new(AutoReplyEngine::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoreply::config::AutoReplyRule;

    fn create_test_engine() -> AutoReplyEngine {
        let mut config = AutoReplyConfig::new().enabled();

        config.add_rule(AutoReplyRule::new(
            "ping",
            "Ping Pong",
            TriggerType::Exact("ping".to_string()),
            ResponseType::Text("pong".to_string()),
        ));

        config.add_rule(AutoReplyRule::new(
            "hello",
            "Hello Response",
            TriggerType::Contains("hello".to_string()),
            ResponseType::Text("Hello {{username}}!".to_string()),
        ));

        config.add_rule(AutoReplyRule::new(
            "echo",
            "Echo Command",
            TriggerType::Command("echo".to_string()),
            ResponseType::Template("You said: {{args}}".to_string()),
        ));

        AutoReplyEngine::new(config)
    }

    #[test]
    fn test_exact_match() {
        let engine = create_test_engine();
        let ctx = MatchContext::new("telegram", "user1", "ping");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].response_text, "pong");
    }

    #[test]
    fn test_case_insensitive_exact() {
        let engine = create_test_engine();
        let ctx = MatchContext::new("telegram", "user1", "PING");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_contains_match() {
        let engine = create_test_engine();
        let ctx = MatchContext::new("telegram", "user1", "well hello there")
            .with_username("Alice");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].response_text, "Hello Alice!");
    }

    #[test]
    fn test_no_match() {
        let engine = create_test_engine();
        let ctx = MatchContext::new("telegram", "user1", "random message");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_command_match() {
        let engine = create_test_engine();
        let ctx = MatchContext::new("telegram", "user1", "/echo testing world");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].response_text, "You said: testing world");
    }

    #[test]
    fn test_bang_command_match() {
        let engine = create_test_engine();
        let ctx = MatchContext::new("discord", "user1", "!echo test");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].response_text, "You said: test");
    }

    #[test]
    fn test_channel_filter() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(
            AutoReplyRule::new(
                "telegram-only",
                "Telegram Only",
                TriggerType::Any,
                ResponseType::Text("Hi".to_string()),
            )
            .with_channels(vec!["telegram".to_string()]),
        );

        let engine = AutoReplyEngine::new(config);

        let ctx_telegram = MatchContext::new("telegram", "user1", "anything");
        let ctx_discord = MatchContext::new("discord", "user1", "anything");

        assert_eq!(engine.process_message(&ctx_telegram).len(), 1);
        assert_eq!(engine.process_message(&ctx_discord).len(), 0);
    }

    #[test]
    fn test_user_filter() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(
            AutoReplyRule::new(
                "vip-only",
                "VIP Only",
                TriggerType::Any,
                ResponseType::Text("Welcome VIP".to_string()),
            )
            .with_users(vec!["vip1".to_string()]),
        );

        let engine = AutoReplyEngine::new(config);

        let ctx_vip = MatchContext::new("telegram", "vip1", "anything");
        let ctx_regular = MatchContext::new("telegram", "user1", "anything");

        assert_eq!(engine.process_message(&ctx_vip).len(), 1);
        assert_eq!(engine.process_message(&ctx_regular).len(), 0);
    }

    #[test]
    fn test_cooldown() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(
            AutoReplyRule::new(
                "spammy",
                "Spammy Rule",
                TriggerType::Any,
                ResponseType::Text("Hi".to_string()),
            )
            .with_cooldown(3600), // 1 hour cooldown
        );

        let engine = AutoReplyEngine::new(config);
        let ctx = MatchContext::new("telegram", "user1", "test");

        // First trigger should work
        assert_eq!(engine.process_message(&ctx).len(), 1);

        // Second trigger should be on cooldown
        assert_eq!(engine.process_message(&ctx).len(), 0);

        // Clear cooldown and try again
        engine.clear_all_cooldowns();
        assert_eq!(engine.process_message(&ctx).len(), 1);
    }

    #[test]
    fn test_stop_processing() {
        let mut config = AutoReplyConfig::new().enabled();

        config.add_rule(
            AutoReplyRule::new(
                "first",
                "First Rule",
                TriggerType::Any,
                ResponseType::Text("First".to_string()),
            )
            .with_priority(10)
            .with_stop_processing(true),
        );

        config.add_rule(
            AutoReplyRule::new(
                "second",
                "Second Rule",
                TriggerType::Any,
                ResponseType::Text("Second".to_string()),
            )
            .with_priority(5),
        );

        let engine = AutoReplyEngine::new(config);
        let ctx = MatchContext::new("telegram", "user1", "test");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].response_text, "First");
    }

    #[test]
    fn test_disabled_engine() {
        let config = AutoReplyConfig::new(); // Not enabled
        let engine = AutoReplyEngine::new(config);
        let ctx = MatchContext::new("telegram", "user1", "ping");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_regex_trigger() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(AutoReplyRule::new(
            "weather",
            "Weather Query",
            TriggerType::Regex(r"what's the weather in (?P<city>\w+)".to_string()),
            ResponseType::Template("The weather in {{city}} is sunny!".to_string()),
        ));

        let engine = AutoReplyEngine::new(config);
        let ctx = MatchContext::new("telegram", "user1", "what's the weather in London");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].response_text, "The weather in London is sunny!");
    }

    #[test]
    fn test_random_response() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(AutoReplyRule::new(
            "greet",
            "Greeting",
            TriggerType::Exact("hi".to_string()),
            ResponseType::Random(vec![
                "Hello!".to_string(),
                "Hi there!".to_string(),
                "Hey!".to_string(),
            ]),
        ));

        let engine = AutoReplyEngine::new(config);
        let ctx = MatchContext::new("telegram", "user1", "hi");

        let results = engine.process_message(&ctx);
        assert_eq!(results.len(), 1);
        // Should be one of the random options
        let response = &results[0].response_text;
        assert!(response == "Hello!" || response == "Hi there!" || response == "Hey!");
    }

    #[test]
    fn test_max_uses_per_user() {
        let mut config = AutoReplyConfig::new().enabled();
        config.add_rule(
            AutoReplyRule::new(
                "limited",
                "Limited Rule",
                TriggerType::Any,
                ResponseType::Text("Hi".to_string()),
            )
            .with_max_uses(2),
        );

        let engine = AutoReplyEngine::new(config);
        let ctx = MatchContext::new("telegram", "user1", "test");

        // First two should work
        assert_eq!(engine.process_message(&ctx).len(), 1);
        assert_eq!(engine.process_message(&ctx).len(), 1);

        // Third should fail (max uses reached)
        assert_eq!(engine.process_message(&ctx).len(), 0);

        // Different user should still work
        let ctx2 = MatchContext::new("telegram", "user2", "test");
        assert_eq!(engine.process_message(&ctx2).len(), 1);
    }
}
