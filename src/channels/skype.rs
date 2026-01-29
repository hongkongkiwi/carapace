//! Skype channel plugin.
//!
//! Placeholder implementation for Skype messaging.
//! Requires Microsoft Graph API for actual functionality.

use uuid::Uuid;

use crate::plugins::{
    BindingError, ChannelCapabilities, ChannelInfo, ChannelPluginInstance, DeliveryResult,
    OutboundContext,
};

/// A channel plugin for Skype messaging.
pub struct SkypeChannel {
    /// Microsoft Graph API endpoint
    graph_endpoint: String,
    /// Access token for API
    access_token: String,
    /// Bot's Skype ID
    bot_id: String,
}

impl SkypeChannel {
    /// Create a new Skype channel.
    pub fn new(graph_endpoint: String, access_token: String, bot_id: String) -> Self {
        Self {
            graph_endpoint,
            access_token,
            bot_id,
        }
    }
}

impl Default for SkypeChannel {
    fn default() -> Self {
        Self {
            graph_endpoint: "https://graph.microsoft.com/v1.0".to_string(),
            access_token: String::new(),
            bot_id: "skype-bot".to_string(),
        }
    }
}

impl ChannelPluginInstance for SkypeChannel {
    fn get_info(&self) -> Result<ChannelInfo, BindingError> {
        Ok(ChannelInfo {
            id: "skype".to_string(),
            label: "Skype".to_string(),
            selection_label: "Skype Channel".to_string(),
            docs_path: "".to_string(),
            blurb: "Skype messaging via Microsoft Graph API".to_string(),
            order: 50,
        })
    }

    fn get_capabilities(&self) -> Result<ChannelCapabilities, BindingError> {
        Ok(ChannelCapabilities {
            media: true,
            polls: false,
            reactions: true,
            edit: true,
            unsend: true,
            ..Default::default()
        })
    }

    fn send_text(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        // Placeholder: would call Microsoft Graph API here
        tracing::warn!(
            channel = "skype",
            to = %ctx.to,
            "[skype] Would send to {}: {}",
            ctx.to,
            ctx.text,
        );

        Ok(DeliveryResult {
            ok: true,
            message_id: Some(Uuid::new_v4().to_string()),
            error: None,
            retryable: false,
            conversation_id: Some(ctx.to.clone()),
            to_jid: Some(format!("{}@skype", ctx.to)),
            poll_id: None,
        })
    }

    fn send_media(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        tracing::warn!(
            channel = "skype",
            to = %ctx.to,
            "[skype] Would send media to {}",
            ctx.to,
        );

        Ok(DeliveryResult {
            ok: true,
            message_id: Some(Uuid::new_v4().to_string()),
            error: None,
            retryable: false,
            conversation_id: Some(ctx.to.clone()),
            to_jid: Some(format!("{}@skype", ctx.to)),
            poll_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skype_get_info() {
        let ch = SkypeChannel::default();
        let info = ch.get_info().unwrap();
        assert_eq!(info.id, "skype");
        assert_eq!(info.label, "Skype");
    }

    #[test]
    fn test_skype_get_capabilities() {
        let ch = SkypeChannel::default();
        let caps = ch.get_capabilities().unwrap();
        assert!(caps.media);
        assert!(caps.reactions);
        assert!(caps.edit);
        assert!(caps.unsend);
    }

    #[test]
    fn test_skype_send_text() {
        let ch = SkypeChannel::default();
        let ctx = OutboundContext {
            to: "user123".to_string(),
            text: "Hello Skype".to_string(),
            media_url: None,
            gif_playback: false,
            reply_to_id: None,
            thread_id: None,
            account_id: None,
        };
        let result = ch.send_text(ctx).unwrap();
        assert!(result.ok);
        assert!(result.message_id.is_some());
    }
}
