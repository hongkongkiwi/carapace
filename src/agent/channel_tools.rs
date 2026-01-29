//! Channel-specific built-in tools.
//!
//! Provides tools that are only available when the agent conversation originated
//! from a specific messaging channel (Telegram, Discord, Slack). These tools
//! return structured intent objects that the delivery pipeline picks up and
//! routes to the appropriate channel plugin — they do NOT make actual API calls.

use serde_json::{json, Value};

use crate::plugins::tools::{BuiltinTool, ToolInvokeContext, ToolInvokeResult};

/// Return channel-specific tools for the given channel.
/// Returns an empty Vec if channel is None or unrecognized.
pub fn channel_tools(channel: Option<&str>) -> Vec<BuiltinTool> {
    match channel {
        Some("telegram") => telegram_tools(),
        Some("discord") => discord_tools(),
        Some("slack") => slack_tools(),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate that the invocation context matches the expected channel.
/// Returns an error result if the channel does not match.
fn require_channel(ctx: &ToolInvokeContext, expected: &str) -> Option<ToolInvokeResult> {
    match ctx.message_channel.as_deref() {
        Some(ch) if ch == expected => None,
        _ => Some(ToolInvokeResult::tool_error(format!(
            "this tool requires the '{expected}' channel"
        ))),
    }
}

/// Extract a required string parameter from `args`, returning an error result on failure.
fn require_str(args: &Value, key: &str) -> Result<String, ToolInvokeResult> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolInvokeResult::tool_error(format!("missing required parameter: {key}")))
}

// ===========================================================================
// Telegram tools
// ===========================================================================

fn telegram_tools() -> Vec<BuiltinTool> {
    vec![
        telegram_edit_message(),
        telegram_delete_message(),
        telegram_pin_message(),
        telegram_reply_markup(),
        telegram_send_photo(),
    ]
}

fn telegram_edit_message() -> BuiltinTool {
    BuiltinTool {
        name: "telegram_edit_message".to_string(),
        description: "Edit an existing Telegram message by message_id.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to edit."
                },
                "text": {
                    "type": "string",
                    "description": "The new text content for the message."
                }
            },
            "required": ["message_id", "text"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "telegram") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let text = match require_str(&args, "text") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id,
                "text": text
            }))
        }),
    }
}

fn telegram_delete_message() -> BuiltinTool {
    BuiltinTool {
        name: "telegram_delete_message".to_string(),
        description: "Delete a Telegram message by message_id.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to delete."
                }
            },
            "required": ["message_id"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "telegram") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id
            }))
        }),
    }
}

fn telegram_pin_message() -> BuiltinTool {
    BuiltinTool {
        name: "telegram_pin_message".to_string(),
        description: "Pin a Telegram message in the chat.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to pin."
                },
                "silent": {
                    "type": "boolean",
                    "description": "If true, pin without notification. Defaults to false."
                }
            },
            "required": ["message_id"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "telegram") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let silent = args
                .get("silent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id,
                "silent": silent
            }))
        }),
    }
}

fn telegram_reply_markup() -> BuiltinTool {
    BuiltinTool {
        name: "telegram_reply_markup".to_string(),
        description: "Set inline keyboard markup on a Telegram message.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to set markup on."
                },
                "buttons": {
                    "type": "array",
                    "description": "Array of button objects with 'text' and either 'callback_data' or 'url'.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string" },
                            "callback_data": { "type": "string" },
                            "url": { "type": "string" }
                        },
                        "required": ["text"]
                    }
                }
            },
            "required": ["message_id", "buttons"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "telegram") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let buttons = match args.get("buttons").and_then(|v| v.as_array()) {
                Some(arr) => arr,
                None => return ToolInvokeResult::tool_error("missing required parameter: buttons"),
            };
            let button_count = buttons.len();
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id,
                "button_count": button_count
            }))
        }),
    }
}

fn telegram_send_photo() -> BuiltinTool {
    BuiltinTool {
        name: "telegram_send_photo".to_string(),
        description: "Send a photo to the Telegram chat with an optional caption.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL of the photo to send."
                },
                "caption": {
                    "type": "string",
                    "description": "Optional caption for the photo."
                }
            },
            "required": ["url"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "telegram") {
                return err;
            }
            let url = match require_str(&args, "url") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let caption = args
                .get("caption")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            ToolInvokeResult::success(json!({
                "queued": true,
                "url": url,
                "caption": caption
            }))
        }),
    }
}

// ===========================================================================
// Discord tools
// ===========================================================================

fn discord_tools() -> Vec<BuiltinTool> {
    vec![
        discord_add_reaction(),
        discord_send_embed(),
        discord_create_thread(),
        discord_edit_message(),
        discord_delete_message(),
    ]
}

fn discord_add_reaction() -> BuiltinTool {
    BuiltinTool {
        name: "discord_add_reaction".to_string(),
        description: "Add a reaction emoji to a Discord message.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to react to."
                },
                "emoji": {
                    "type": "string",
                    "description": "The emoji to add as a reaction."
                }
            },
            "required": ["message_id", "emoji"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "discord") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let emoji = match require_str(&args, "emoji") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id,
                "emoji": emoji
            }))
        }),
    }
}

fn discord_send_embed() -> BuiltinTool {
    BuiltinTool {
        name: "discord_send_embed".to_string(),
        description: "Send a rich embed message to the Discord channel.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "The embed title."
                },
                "description": {
                    "type": "string",
                    "description": "The embed description."
                },
                "color": {
                    "type": "integer",
                    "description": "The embed color as an integer (hex color value)."
                },
                "fields": {
                    "type": "array",
                    "description": "Array of field objects with 'name', 'value', and optional 'inline'.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "value": { "type": "string" },
                            "inline": { "type": "boolean" }
                        },
                        "required": ["name", "value"]
                    }
                },
                "footer": {
                    "type": "string",
                    "description": "The embed footer text."
                }
            },
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "discord") {
                return err;
            }
            let embed_fields = args
                .get("fields")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            ToolInvokeResult::success(json!({
                "queued": true,
                "embed_fields": embed_fields
            }))
        }),
    }
}

fn discord_create_thread() -> BuiltinTool {
    BuiltinTool {
        name: "discord_create_thread".to_string(),
        description: "Create a thread from a Discord message.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to create a thread from."
                },
                "name": {
                    "type": "string",
                    "description": "The name of the thread."
                },
                "auto_archive_minutes": {
                    "type": "integer",
                    "description": "Auto-archive duration in minutes. Defaults to 1440 (24 hours)."
                }
            },
            "required": ["message_id", "name"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "discord") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let name = match require_str(&args, "name") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let _auto_archive = args
                .get("auto_archive_minutes")
                .and_then(|v| v.as_i64())
                .unwrap_or(1440);
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id,
                "thread_name": name
            }))
        }),
    }
}

fn discord_edit_message() -> BuiltinTool {
    BuiltinTool {
        name: "discord_edit_message".to_string(),
        description: "Edit a Discord message by message_id.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to edit."
                },
                "content": {
                    "type": "string",
                    "description": "The new content for the message."
                }
            },
            "required": ["message_id", "content"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "discord") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let content = match require_str(&args, "content") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id,
                "content": content
            }))
        }),
    }
}

fn discord_delete_message() -> BuiltinTool {
    BuiltinTool {
        name: "discord_delete_message".to_string(),
        description: "Delete a Discord message by message_id.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message_id": {
                    "type": "string",
                    "description": "The ID of the message to delete."
                }
            },
            "required": ["message_id"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "discord") {
                return err;
            }
            let message_id = match require_str(&args, "message_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "message_id": message_id
            }))
        }),
    }
}

// ===========================================================================
// Slack tools
// ===========================================================================

fn slack_tools() -> Vec<BuiltinTool> {
    vec![
        slack_send_blocks(),
        slack_send_ephemeral(),
        slack_add_reaction(),
        slack_update_message(),
        slack_delete_message(),
    ]
}

fn slack_send_blocks() -> BuiltinTool {
    BuiltinTool {
        name: "slack_send_blocks".to_string(),
        description: "Send a message with Block Kit blocks to a Slack channel.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "The Slack channel ID to send to."
                },
                "blocks": {
                    "type": "array",
                    "description": "Block Kit JSON blocks."
                }
            },
            "required": ["channel", "blocks"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "slack") {
                return err;
            }
            let channel = match require_str(&args, "channel") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let blocks = match args.get("blocks").and_then(|v| v.as_array()) {
                Some(arr) => arr,
                None => return ToolInvokeResult::tool_error("missing required parameter: blocks"),
            };
            let block_count = blocks.len();
            ToolInvokeResult::success(json!({
                "queued": true,
                "channel": channel,
                "block_count": block_count
            }))
        }),
    }
}

fn slack_send_ephemeral() -> BuiltinTool {
    BuiltinTool {
        name: "slack_send_ephemeral".to_string(),
        description: "Send an ephemeral message visible only to one user in a Slack channel."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "The Slack channel ID."
                },
                "user": {
                    "type": "string",
                    "description": "The Slack user ID who will see the ephemeral message."
                },
                "text": {
                    "type": "string",
                    "description": "The message text."
                }
            },
            "required": ["channel", "user", "text"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "slack") {
                return err;
            }
            let channel = match require_str(&args, "channel") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let user = match require_str(&args, "user") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let text = match require_str(&args, "text") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "queued": true,
                "channel": channel,
                "user": user,
                "text": text
            }))
        }),
    }
}

fn slack_add_reaction() -> BuiltinTool {
    BuiltinTool {
        name: "slack_add_reaction".to_string(),
        description: "Add a reaction emoji to a Slack message.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "The Slack channel ID."
                },
                "timestamp": {
                    "type": "string",
                    "description": "The Slack message timestamp (ts)."
                },
                "emoji": {
                    "type": "string",
                    "description": "The emoji name without colons (e.g. 'thumbsup')."
                }
            },
            "required": ["channel", "timestamp", "emoji"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "slack") {
                return err;
            }
            let channel = match require_str(&args, "channel") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let timestamp = match require_str(&args, "timestamp") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let emoji = match require_str(&args, "emoji") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "channel": channel,
                "timestamp": timestamp,
                "emoji": emoji
            }))
        }),
    }
}

fn slack_update_message() -> BuiltinTool {
    BuiltinTool {
        name: "slack_update_message".to_string(),
        description: "Update an existing Slack message.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "The Slack channel ID."
                },
                "timestamp": {
                    "type": "string",
                    "description": "The Slack message timestamp (ts) to update."
                },
                "text": {
                    "type": "string",
                    "description": "The new text content."
                }
            },
            "required": ["channel", "timestamp", "text"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "slack") {
                return err;
            }
            let channel = match require_str(&args, "channel") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let timestamp = match require_str(&args, "timestamp") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let text = match require_str(&args, "text") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "channel": channel,
                "timestamp": timestamp,
                "text": text
            }))
        }),
    }
}

fn slack_delete_message() -> BuiltinTool {
    BuiltinTool {
        name: "slack_delete_message".to_string(),
        description: "Delete a Slack message.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "The Slack channel ID."
                },
                "timestamp": {
                    "type": "string",
                    "description": "The Slack message timestamp (ts) to delete."
                }
            },
            "required": ["channel", "timestamp"],
            "additionalProperties": false
        }),
        handler: Box::new(|args, ctx| {
            if let Some(err) = require_channel(ctx, "slack") {
                return err;
            }
            let channel = match require_str(&args, "channel") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let timestamp = match require_str(&args, "timestamp") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ToolInvokeResult::success(json!({
                "ok": true,
                "channel": channel,
                "timestamp": timestamp
            }))
        }),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Create a test context with the given channel.
    fn ctx_for(channel: &str) -> ToolInvokeContext {
        ToolInvokeContext {
            message_channel: Some(channel.to_string()),
            ..Default::default()
        }
    }

    /// Assert the result is a success and return the inner value.
    fn unwrap_success(result: ToolInvokeResult) -> Value {
        match result {
            ToolInvokeResult::Success { result, .. } => result,
            ToolInvokeResult::Error { error, .. } => {
                panic!("expected success, got error: {}", error.message)
            }
        }
    }

    /// Assert the result is an error.
    fn assert_error(result: ToolInvokeResult) {
        match result {
            ToolInvokeResult::Error { .. } => {}
            ToolInvokeResult::Success { result, .. } => {
                panic!("expected error, got success: {result}")
            }
        }
    }

    // -----------------------------------------------------------------------
    // Gating tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_channel_tools_none_returns_empty() {
        let tools = channel_tools(None);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_channel_tools_unknown_returns_empty() {
        let tools = channel_tools(Some("unknown"));
        assert!(tools.is_empty());
    }

    #[test]
    fn test_channel_tools_telegram_returns_five() {
        let tools = channel_tools(Some("telegram"));
        assert_eq!(tools.len(), 5, "telegram should have 5 tools");
    }

    #[test]
    fn test_channel_tools_discord_returns_five() {
        let tools = channel_tools(Some("discord"));
        assert_eq!(tools.len(), 5, "discord should have 5 tools");
    }

    #[test]
    fn test_channel_tools_slack_returns_five() {
        let tools = channel_tools(Some("slack"));
        assert_eq!(tools.len(), 5, "slack should have 5 tools");
    }

    // -----------------------------------------------------------------------
    // Schema validity tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_channel_tools_have_valid_schemas() {
        for ch in &["telegram", "discord", "slack"] {
            let tools = channel_tools(Some(ch));
            for tool in &tools {
                assert!(
                    !tool.name.is_empty(),
                    "tool name should not be empty (channel: {ch})"
                );
                assert!(
                    !tool.description.is_empty(),
                    "tool description should not be empty: {} (channel: {ch})",
                    tool.name
                );
                assert_eq!(
                    tool.input_schema["type"], "object",
                    "tool {} schema should have type: object (channel: {ch})",
                    tool.name
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Telegram tool tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_telegram_edit_message_success() {
        let tool = telegram_edit_message();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(json!({"message_id": "42", "text": "edited"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "42");
        assert_eq!(val["text"], "edited");
    }

    #[test]
    fn test_telegram_edit_message_missing_params() {
        let tool = telegram_edit_message();
        let ctx = ctx_for("telegram");
        assert_error((tool.handler)(json!({}), &ctx));
        assert_error((tool.handler)(json!({"message_id": "42"}), &ctx));
        assert_error((tool.handler)(json!({"text": "hello"}), &ctx));
    }

    #[test]
    fn test_telegram_delete_message_success() {
        let tool = telegram_delete_message();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(json!({"message_id": "99"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "99");
    }

    #[test]
    fn test_telegram_delete_message_missing_params() {
        let tool = telegram_delete_message();
        let ctx = ctx_for("telegram");
        assert_error((tool.handler)(json!({}), &ctx));
    }

    #[test]
    fn test_telegram_pin_message_success() {
        let tool = telegram_pin_message();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(json!({"message_id": "10", "silent": true}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "10");
        assert_eq!(val["silent"], true);
    }

    #[test]
    fn test_telegram_pin_message_default_silent() {
        let tool = telegram_pin_message();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(json!({"message_id": "10"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["silent"], false);
    }

    #[test]
    fn test_telegram_reply_markup_success() {
        let tool = telegram_reply_markup();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(
            json!({
                "message_id": "5",
                "buttons": [
                    {"text": "OK", "callback_data": "ok"},
                    {"text": "Visit", "url": "https://example.com"}
                ]
            }),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "5");
        assert_eq!(val["button_count"], 2);
    }

    #[test]
    fn test_telegram_reply_markup_missing_buttons() {
        let tool = telegram_reply_markup();
        let ctx = ctx_for("telegram");
        assert_error((tool.handler)(json!({"message_id": "5"}), &ctx));
    }

    #[test]
    fn test_telegram_send_photo_success() {
        let tool = telegram_send_photo();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(
            json!({"url": "https://img.example.com/a.jpg", "caption": "A photo"}),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["queued"], true);
        assert_eq!(val["url"], "https://img.example.com/a.jpg");
        assert_eq!(val["caption"], "A photo");
    }

    #[test]
    fn test_telegram_send_photo_no_caption() {
        let tool = telegram_send_photo();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(json!({"url": "https://img.example.com/a.jpg"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["queued"], true);
        assert!(val["caption"].is_null());
    }

    #[test]
    fn test_telegram_send_photo_missing_url() {
        let tool = telegram_send_photo();
        let ctx = ctx_for("telegram");
        assert_error((tool.handler)(json!({}), &ctx));
    }

    // -----------------------------------------------------------------------
    // Discord tool tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_discord_add_reaction_success() {
        let tool = discord_add_reaction();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(json!({"message_id": "1001", "emoji": "thumbsup"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "1001");
        assert_eq!(val["emoji"], "thumbsup");
    }

    #[test]
    fn test_discord_add_reaction_missing_params() {
        let tool = discord_add_reaction();
        let ctx = ctx_for("discord");
        assert_error((tool.handler)(json!({}), &ctx));
        assert_error((tool.handler)(json!({"message_id": "1001"}), &ctx));
    }

    #[test]
    fn test_discord_send_embed_success() {
        let tool = discord_send_embed();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(
            json!({
                "title": "Hello",
                "description": "World",
                "color": 0xFF0000,
                "fields": [{"name": "f1", "value": "v1"}],
                "footer": "foot"
            }),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["queued"], true);
        assert_eq!(val["embed_fields"], 1);
    }

    #[test]
    fn test_discord_send_embed_no_fields() {
        let tool = discord_send_embed();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(json!({"title": "Hello"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["queued"], true);
        assert_eq!(val["embed_fields"], 0);
    }

    #[test]
    fn test_discord_create_thread_success() {
        let tool = discord_create_thread();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(json!({"message_id": "200", "name": "my-thread"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "200");
        assert_eq!(val["thread_name"], "my-thread");
    }

    #[test]
    fn test_discord_create_thread_missing_params() {
        let tool = discord_create_thread();
        let ctx = ctx_for("discord");
        assert_error((tool.handler)(json!({}), &ctx));
        assert_error((tool.handler)(json!({"message_id": "200"}), &ctx));
    }

    #[test]
    fn test_discord_edit_message_success() {
        let tool = discord_edit_message();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(json!({"message_id": "300", "content": "new content"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "300");
        assert_eq!(val["content"], "new content");
    }

    #[test]
    fn test_discord_delete_message_success() {
        let tool = discord_delete_message();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(json!({"message_id": "400"}), &ctx);
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["message_id"], "400");
    }

    // -----------------------------------------------------------------------
    // Slack tool tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_slack_send_blocks_success() {
        let tool = slack_send_blocks();
        let ctx = ctx_for("slack");
        let result = (tool.handler)(
            json!({
                "channel": "C123",
                "blocks": [{"type": "section", "text": {"type": "mrkdwn", "text": "hi"}}]
            }),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["queued"], true);
        assert_eq!(val["channel"], "C123");
        assert_eq!(val["block_count"], 1);
    }

    #[test]
    fn test_slack_send_blocks_missing_params() {
        let tool = slack_send_blocks();
        let ctx = ctx_for("slack");
        assert_error((tool.handler)(json!({}), &ctx));
        assert_error((tool.handler)(json!({"channel": "C123"}), &ctx));
    }

    #[test]
    fn test_slack_send_ephemeral_success() {
        let tool = slack_send_ephemeral();
        let ctx = ctx_for("slack");
        let result = (tool.handler)(
            json!({"channel": "C123", "user": "U456", "text": "secret message"}),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["queued"], true);
        assert_eq!(val["channel"], "C123");
        assert_eq!(val["user"], "U456");
        assert_eq!(val["text"], "secret message");
    }

    #[test]
    fn test_slack_send_ephemeral_missing_params() {
        let tool = slack_send_ephemeral();
        let ctx = ctx_for("slack");
        assert_error((tool.handler)(json!({}), &ctx));
        assert_error((tool.handler)(json!({"channel": "C123"}), &ctx));
        assert_error((tool.handler)(
            json!({"channel": "C123", "user": "U456"}),
            &ctx,
        ));
    }

    #[test]
    fn test_slack_add_reaction_success() {
        let tool = slack_add_reaction();
        let ctx = ctx_for("slack");
        let result = (tool.handler)(
            json!({"channel": "C123", "timestamp": "1234567890.123456", "emoji": "thumbsup"}),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["channel"], "C123");
        assert_eq!(val["timestamp"], "1234567890.123456");
        assert_eq!(val["emoji"], "thumbsup");
    }

    #[test]
    fn test_slack_update_message_success() {
        let tool = slack_update_message();
        let ctx = ctx_for("slack");
        let result = (tool.handler)(
            json!({"channel": "C123", "timestamp": "1234567890.123456", "text": "updated"}),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["channel"], "C123");
        assert_eq!(val["timestamp"], "1234567890.123456");
        assert_eq!(val["text"], "updated");
    }

    #[test]
    fn test_slack_delete_message_success() {
        let tool = slack_delete_message();
        let ctx = ctx_for("slack");
        let result = (tool.handler)(
            json!({"channel": "C123", "timestamp": "1234567890.123456"}),
            &ctx,
        );
        let val = unwrap_success(result);
        assert_eq!(val["ok"], true);
        assert_eq!(val["channel"], "C123");
        assert_eq!(val["timestamp"], "1234567890.123456");
    }

    #[test]
    fn test_slack_delete_message_missing_params() {
        let tool = slack_delete_message();
        let ctx = ctx_for("slack");
        assert_error((tool.handler)(json!({}), &ctx));
        assert_error((tool.handler)(json!({"channel": "C123"}), &ctx));
    }

    // -----------------------------------------------------------------------
    // Channel mismatch (defense in depth) tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_telegram_tool_rejects_discord_channel() {
        let tool = telegram_edit_message();
        let ctx = ctx_for("discord");
        let result = (tool.handler)(json!({"message_id": "1", "text": "x"}), &ctx);
        assert_error(result);
    }

    #[test]
    fn test_discord_tool_rejects_telegram_channel() {
        let tool = discord_add_reaction();
        let ctx = ctx_for("telegram");
        let result = (tool.handler)(json!({"message_id": "1", "emoji": "x"}), &ctx);
        assert_error(result);
    }

    #[test]
    fn test_slack_tool_rejects_no_channel() {
        let tool = slack_send_blocks();
        let ctx = ToolInvokeContext::default(); // message_channel is None
        let result = (tool.handler)(json!({"channel": "C1", "blocks": []}), &ctx);
        assert_error(result);
    }
}
