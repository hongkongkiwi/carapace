//! Signal channel plugin.
//!
//! Placeholder implementation for Signal messaging.
//! Requires signal-cli-rest-api or similar backend for actual functionality.

use uuid::Uuid;

use crate::plugins::{
    BindingError, ChannelCapabilities, ChannelInfo, ChannelPluginInstance, DeliveryResult,
    OutboundContext,
};

/// A channel plugin for Signal messaging.
pub struct SignalChannel {
    /// Signal service URL
    service_url: String,
    /// Phone number identifier
    phone_number: String,
}

impl SignalChannel {
    /// Create a new Signal channel.
    pub fn new(service_url: String, phone_number: String) -> Self {
        Self {
            service_url,
            phone_number,
        }
    }
}

impl Default for SignalChannel {
    fn default() -> Self {
        Self {
            service_url: "http://localhost:8080".to_string(),
            phone_number: "+1234567890".to_string(),
        }
    }
}

impl ChannelPluginInstance for SignalChannel {
    fn get_info(&self) -> Result<ChannelInfo, BindingError> {
        Ok(ChannelInfo {
            id: "signal".to_string(),
            label: "Signal".to_string(),
            selection_label: "Signal Channel".to_string(),
            docs_path: "".to_string(),
            blurb: "Signal messaging integration".to_string(),
            order: 45,
        })
    }

    fn get_capabilities(&self) -> Result<ChannelCapabilities, BindingError> {
        Ok(ChannelCapabilities {
            media: true,
            polls: false,
            reactions: false,
            ..Default::default()
        })
    }

    fn send_text(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        // Placeholder: would call Signal API here
        tracing::warn!(
            channel = "signal",
            to = %ctx.to,
            "[signal] Would send to {}: {}",
            ctx.to,
            ctx.text,
        );

        Ok(DeliveryResult {
            ok: true,
            message_id: Some(Uuid::new_v4().to_string()),
            error: None,
            retryable: false,
            conversation_id: Some(ctx.to.clone()),
            to_jid: Some(format!("{}@signal", ctx.to)),
            poll_id: None,
        })
    }

    fn send_media(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        tracing::warn!(
            channel = "signal",
            to = %ctx.to,
            "[signal] Would send media to {}",
            ctx.to,
        );

        Ok(DeliveryResult {
            ok: true,
            message_id: Some(Uuid::new_v4().to_string()),
            error: None,
            retryable: false,
            conversation_id: Some(ctx.to.clone()),
            to_jid: Some(format!("{}@signal", ctx.to)),
            poll_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_get_info() {
        let ch = SignalChannel::default();
        let info = ch.get_info().unwrap();
        assert_eq!(info.id, "signal");
        assert_eq!(info.label, "Signal");
    }

    #[test]
    fn test_signal_get_capabilities() {
        let ch = SignalChannel::default();
        let caps = ch.get_capabilities().unwrap();
        assert!(caps.media);
    }

    #[test]
    fn test_signal_send_text() {
        let ch = SignalChannel::default();
        let ctx = OutboundContext {
            to: "+1234567890".to_string(),
            text: "Hello Signal".to_string(),
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
