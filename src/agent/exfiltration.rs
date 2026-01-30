//! Exfiltration-sensitive tool policy.
//!
//! Identifies tools that can send data to external services (e.g. POST to
//! arbitrary URLs, send messages to messaging channels). When the
//! `exfiltration_guard` flag is enabled on an [`AgentConfig`], these tools are:
//!
//! 1. **Filtered from definitions** — the LLM never sees them.
//! 2. **Blocked at dispatch** — even if the model hallucinates the tool name,
//!    the executor returns an error instead of executing it.
//!
//! This two-layer approach prevents prompt-injection attacks from silently
//! exfiltrating user data through outbound tool calls.

use std::collections::HashSet;
use std::sync::LazyLock;

/// The canonical set of tool names considered exfiltration-sensitive.
///
/// A tool is exfiltration-sensitive if it can transmit data to an external
/// service (HTTP endpoint, messaging platform, etc.) under the control of the
/// agent's prompt — making it a vector for prompt-injection data exfiltration.
static EXFILTRATION_SENSITIVE_TOOLS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        // Built-in tools that reach external services
        "web_fetch",
        "message_send",
        // Telegram channel tools
        "telegram_edit_message",
        "telegram_delete_message",
        "telegram_pin_message",
        "telegram_reply_markup",
        "telegram_send_photo",
        // Discord channel tools
        "discord_add_reaction",
        "discord_send_embed",
        "discord_create_thread",
        "discord_edit_message",
        "discord_delete_message",
        // Slack channel tools
        "slack_send_blocks",
        "slack_send_ephemeral",
        "slack_add_reaction",
        "slack_update_message",
        "slack_delete_message",
    ])
});

/// Returns `true` if `tool_name` is classified as exfiltration-sensitive.
///
/// Exfiltration-sensitive tools are those that can send data to external
/// services (HTTP endpoints, messaging channels, etc.). When the agent's
/// `exfiltration_guard` is enabled, these tools are blocked at both the
/// definition-filtering and dispatch levels.
pub fn is_exfiltration_sensitive(tool_name: &str) -> bool {
    EXFILTRATION_SENSITIVE_TOOLS.contains(tool_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Positive cases: all known exfiltration-sensitive tools =====

    #[test]
    fn test_web_fetch_is_sensitive() {
        assert!(is_exfiltration_sensitive("web_fetch"));
    }

    #[test]
    fn test_message_send_is_sensitive() {
        assert!(is_exfiltration_sensitive("message_send"));
    }

    // -- Telegram --

    #[test]
    fn test_telegram_edit_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("telegram_edit_message"));
    }

    #[test]
    fn test_telegram_delete_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("telegram_delete_message"));
    }

    #[test]
    fn test_telegram_pin_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("telegram_pin_message"));
    }

    #[test]
    fn test_telegram_reply_markup_is_sensitive() {
        assert!(is_exfiltration_sensitive("telegram_reply_markup"));
    }

    #[test]
    fn test_telegram_send_photo_is_sensitive() {
        assert!(is_exfiltration_sensitive("telegram_send_photo"));
    }

    // -- Discord --

    #[test]
    fn test_discord_add_reaction_is_sensitive() {
        assert!(is_exfiltration_sensitive("discord_add_reaction"));
    }

    #[test]
    fn test_discord_send_embed_is_sensitive() {
        assert!(is_exfiltration_sensitive("discord_send_embed"));
    }

    #[test]
    fn test_discord_create_thread_is_sensitive() {
        assert!(is_exfiltration_sensitive("discord_create_thread"));
    }

    #[test]
    fn test_discord_edit_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("discord_edit_message"));
    }

    #[test]
    fn test_discord_delete_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("discord_delete_message"));
    }

    // -- Slack --

    #[test]
    fn test_slack_send_blocks_is_sensitive() {
        assert!(is_exfiltration_sensitive("slack_send_blocks"));
    }

    #[test]
    fn test_slack_send_ephemeral_is_sensitive() {
        assert!(is_exfiltration_sensitive("slack_send_ephemeral"));
    }

    #[test]
    fn test_slack_add_reaction_is_sensitive() {
        assert!(is_exfiltration_sensitive("slack_add_reaction"));
    }

    #[test]
    fn test_slack_update_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("slack_update_message"));
    }

    #[test]
    fn test_slack_delete_message_is_sensitive() {
        assert!(is_exfiltration_sensitive("slack_delete_message"));
    }

    // ===== Negative cases: tools that are NOT exfiltration-sensitive =====

    #[test]
    fn test_time_is_not_sensitive() {
        assert!(!is_exfiltration_sensitive("time"));
    }

    #[test]
    fn test_search_is_not_sensitive() {
        assert!(!is_exfiltration_sensitive("search"));
    }

    #[test]
    fn test_empty_string_is_not_sensitive() {
        assert!(!is_exfiltration_sensitive(""));
    }

    #[test]
    fn test_unknown_tool_is_not_sensitive() {
        assert!(!is_exfiltration_sensitive("totally_unknown_tool"));
    }

    #[test]
    fn test_similar_name_is_not_sensitive() {
        // Ensure partial matches don't count
        assert!(!is_exfiltration_sensitive("web_fetch_v2"));
        assert!(!is_exfiltration_sensitive("message_send_batch"));
        assert!(!is_exfiltration_sensitive("telegram_edit"));
        assert!(!is_exfiltration_sensitive("slack_send"));
    }

    // ===== Completeness =====

    #[test]
    fn test_exactly_17_sensitive_tools() {
        // 2 built-in + 5 Telegram + 5 Discord + 5 Slack = 17
        assert_eq!(EXFILTRATION_SENSITIVE_TOOLS.len(), 17);
    }

    #[test]
    fn test_all_channel_tools_are_covered() {
        use crate::agent::channel_tools::channel_tools;

        for channel in &["telegram", "discord", "slack"] {
            let tools = channel_tools(Some(channel));
            for tool in &tools {
                assert!(
                    is_exfiltration_sensitive(&tool.name),
                    "channel tool '{}' (channel: {}) should be exfiltration-sensitive",
                    tool.name,
                    channel,
                );
            }
        }
    }
}
