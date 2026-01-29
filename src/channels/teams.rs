//! Microsoft Teams channel plugin.
//!
//! Placeholder implementation for Microsoft Teams messaging.
//! Requires Microsoft Graph API for actual functionality.

use uuid::Uuid;

use crate::plugins::{
    BindingError, ChannelCapabilities, ChannelInfo, ChannelPluginInstance, DeliveryResult,
    OutboundContext,
};

/// A channel plugin for Microsoft Teams messaging.
pub struct TeamsChannel {
    /// Microsoft Graph API endpoint
    graph_endpoint: String,
    /// Access token for API
    access_token: String,
    /// Tenant ID
    tenant_id: String,
    /// Bot's Teams ID
    bot_id: String,
}

impl TeamsChannel {
    /// Create a new Teams channel.
    pub fn new(
        graph_endpoint: String,
        access_token: String,
        tenant_id: String,
        bot_id: String,
    ) -> Self {
        Self {
            graph_endpoint,
            access_token,
            tenant_id,
            bot_id,
        }
    }
}

impl Default for TeamsChannel {
    fn default() -> Self {
        Self {
            graph_endpoint: "https://graph.microsoft.com/v1.0".to_string(),
            access_token: String::new(),
            tenant_id: String::new(),
            bot_id: "teams-bot".to_string(),
        }
    }
}

impl ChannelPluginInstance for TeamsChannel {
    fn get_info(&self) -> Result<ChannelInfo, BindingError> {
        Ok(ChannelInfo {
            id: "teams".to_string(),
            label: "Microsoft Teams".to_string(),
            selection_label: "Teams Channel".to_string(),
            docs_path: "".to_string(),
            blurb: "Microsoft Teams messaging via Graph API".to_string(),
            order: 55,
        })
    }

    fn get_capabilities(&self) -> Result<ChannelCapabilities, BindingError> {
        Ok(ChannelCapabilities {
            media: true,
            polls: true,
            reactions: true,
            edit: true,
            unsend: true,
            threads: true,
            ..Default::default()
        })
    }

    fn send_text(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        // Placeholder: would call Microsoft Graph API here
        tracing::warn!(
            channel = "teams",
            to = %ctx.to,
            "[teams] Would send to {}: {}",
            ctx.to,
            ctx.text,
        );

        Ok(DeliveryResult {
            ok: true,
            message_id: Some(Uuid::new_v4().to_string()),
            error: None,
            retryable: false,
            conversation_id: Some(ctx.to.clone()),
            to_jid: Some(format!("{}@teams", ctx.to)),
            poll_id: None,
        })
    }

    fn send_media(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        tracing::warn!(
            channel = "teams",
            to = %ctx.to,
            "[teams] Would send media to {}",
            ctx.to,
        );

        Ok(DeliveryResult {
            ok: true,
            message_id: Some(Uuid::new_v4().to_string()),
            error: None,
            retryable: false,
            conversation_id: Some(ctx.to.clone()),
            to_jid: Some(format!("{}@teams", ctx.to)),
            poll_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teams_get_info() {
        let ch = TeamsChannel::default();
        let info = ch.get_info().unwrap();
        assert_eq!(info.id, "teams");
        assert_eq!(info.label, "Microsoft Teams");
    }

    #[test]
    fn test_teams_get_capabilities() {
        let ch = TeamsChannel::default();
        let caps = ch.get_capabilities().unwrap();
        assert!(caps.media);
        assert!(caps.polls);
        assert!(caps.reactions);
        assert!(caps.edit);
        assert!(caps.unsend);
        assert!(caps.threads);
    }

    #[test]
    fn test_teams_send_text() {
        let ch = TeamsChannel::default();
        let ctx = OutboundContext {
            to: "user@company.onmicrosoft.com".to_string(),
            text: "Hello Teams".to_string(),
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
