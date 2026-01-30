//! Webhook channel plugin.
//!
//! Delivers messages by POSTing JSON to a configured URL.
//! Uses `reqwest::blocking::Client` since `ChannelPluginInstance` methods are sync.

use std::collections::HashMap;

use crate::plugins::capabilities::SsrfProtection;
use crate::plugins::{
    BindingError, ChannelCapabilities, ChannelInfo, ChannelPluginInstance, DeliveryResult,
    OutboundContext,
};

/// A channel plugin that delivers messages via HTTP webhooks.
pub struct WebhookChannel {
    client: reqwest::blocking::Client,
    url: String,
    headers: HashMap<String, String>,
}

impl WebhookChannel {
    /// Create a new webhook channel targeting the given URL.
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            url,
            headers: HashMap::new(),
        }
    }

    /// Set custom headers to include in webhook requests.
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}

impl ChannelPluginInstance for WebhookChannel {
    fn get_info(&self) -> Result<ChannelInfo, BindingError> {
        Ok(ChannelInfo {
            id: "webhook".to_string(),
            label: "Webhook".to_string(),
            selection_label: "Webhook Channel".to_string(),
            docs_path: "".to_string(),
            blurb: "Delivers messages via HTTP POST".to_string(),
            order: 100,
        })
    }

    fn get_capabilities(&self) -> Result<ChannelCapabilities, BindingError> {
        Ok(ChannelCapabilities {
            media: true,
            ..Default::default()
        })
    }

    fn send_text(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        let body = serde_json::json!({
            "to": ctx.to,
            "text": ctx.text,
            "replyTo": ctx.reply_to_id,
        });

        self.post_json(&body)
    }

    fn send_media(&self, ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
        let body = serde_json::json!({
            "to": ctx.to,
            "mediaUrl": ctx.media_url,
            "caption": ctx.text,
        });

        self.post_json(&body)
    }
}

impl WebhookChannel {
    fn post_json(&self, body: &serde_json::Value) -> Result<DeliveryResult, BindingError> {
        // SSRF defense-in-depth: re-validate URL before each request
        if let Err(e) = SsrfProtection::validate_url(&self.url) {
            return Ok(DeliveryResult {
                ok: false,
                message_id: None,
                error: Some(format!("webhook URL blocked by SSRF protection: {}", e)),
                retryable: false,
                conversation_id: None,
                to_jid: None,
                poll_id: None,
            });
        }

        let mut req = self.client.post(&self.url).json(body);

        for (key, value) in &self.headers {
            req = req.header(key, value);
        }

        match req.send() {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    Ok(DeliveryResult {
                        ok: true,
                        message_id: None,
                        error: None,
                        retryable: false,
                        conversation_id: None,
                        to_jid: None,
                        poll_id: None,
                    })
                } else {
                    let retryable = status.is_server_error();
                    Ok(DeliveryResult {
                        ok: false,
                        message_id: None,
                        error: Some(format!("HTTP {}", status)),
                        retryable,
                        conversation_id: None,
                        to_jid: None,
                        poll_id: None,
                    })
                }
            }
            Err(e) => Ok(DeliveryResult {
                ok: false,
                message_id: None,
                error: Some(e.to_string()),
                retryable: true,
                conversation_id: None,
                to_jid: None,
                poll_id: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_get_info() {
        let wh = WebhookChannel::new("https://example.com/hook".to_string());
        let info = wh.get_info().unwrap();
        assert_eq!(info.id, "webhook");
        assert_eq!(info.label, "Webhook");
    }

    #[test]
    fn test_webhook_get_capabilities() {
        let wh = WebhookChannel::new("https://example.com/hook".to_string());
        let caps = wh.get_capabilities().unwrap();
        assert!(caps.media);
    }

    #[test]
    fn test_webhook_send_text_connection_failure() {
        // Uses a public IP on an unreachable port to verify request construction
        // doesn't panic. The SSRF check passes but the connection fails.
        let wh = WebhookChannel::new("http://192.0.2.1:1/nonexistent".to_string());
        let ctx = OutboundContext {
            to: "user123".to_string(),
            text: "Hello".to_string(),
            media_url: None,
            gif_playback: false,
            reply_to_id: Some("msg-456".to_string()),
            thread_id: None,
            account_id: None,
        };
        // Will fail SSRF check (TEST-NET-1 is blocked) — verifies SSRF protection
        let result = wh.send_text(ctx).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable); // SSRF block is not retryable
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));
    }

    #[test]
    fn test_webhook_send_media_connection_failure() {
        // Uses a public IP on an unreachable port to verify request construction
        let wh = WebhookChannel::new("http://192.0.2.1:1/nonexistent".to_string());
        let ctx = OutboundContext {
            to: "user123".to_string(),
            text: "A photo".to_string(),
            media_url: Some("https://example.com/img.jpg".to_string()),
            gif_playback: false,
            reply_to_id: None,
            thread_id: None,
            account_id: None,
        };
        let result = wh.send_media(ctx).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable);
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));
    }

    #[test]
    fn test_webhook_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        let wh = WebhookChannel::new("https://example.com/hook".to_string()).with_headers(headers);
        assert_eq!(
            wh.headers.get("Authorization"),
            Some(&"Bearer token123".to_string())
        );
    }

    // ============== Webhook SSRF Protection Tests ==============

    #[test]
    fn test_webhook_ssrf_blocks_localhost() {
        let wh = WebhookChannel::new("http://127.0.0.1:8080/internal".to_string());
        let body = serde_json::json!({"to": "user", "text": "test"});
        let result = wh.post_json(&body).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable);
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));
        assert!(result.error.as_ref().unwrap().contains("localhost"));
    }

    #[test]
    fn test_webhook_ssrf_blocks_metadata() {
        let wh = WebhookChannel::new("http://169.254.169.254/latest/meta-data/".to_string());
        let body = serde_json::json!({"to": "user", "text": "test"});
        let result = wh.post_json(&body).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable);
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));
    }

    #[test]
    fn test_webhook_ssrf_blocks_private_ip() {
        // 10.0.0.0/8
        let wh = WebhookChannel::new("http://10.0.0.1:3000/webhook".to_string());
        let body = serde_json::json!({"to": "user", "text": "test"});
        let result = wh.post_json(&body).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable);
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));

        // 192.168.0.0/16
        let wh = WebhookChannel::new("http://192.168.1.100/webhook".to_string());
        let result = wh.post_json(&body).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable);
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));

        // 172.16.0.0/12
        let wh = WebhookChannel::new("http://172.16.0.1/webhook".to_string());
        let result = wh.post_json(&body).unwrap();
        assert!(!result.ok);
        assert!(!result.retryable);
        assert!(result.error.as_ref().unwrap().contains("SSRF protection"));
    }

    #[test]
    fn test_webhook_ssrf_allows_public_url() {
        // Public URL should pass SSRF validation (will fail at HTTP level since
        // the server doesn't exist, but the SSRF check itself should pass)
        let wh = WebhookChannel::new("https://hooks.example.com/webhook".to_string());
        let body = serde_json::json!({"to": "user", "text": "test"});
        let result = wh.post_json(&body).unwrap();
        // The request will fail due to DNS/connection error, but NOT due to SSRF
        assert!(!result.ok);
        assert!(
            !result.error.as_ref().unwrap().contains("SSRF protection"),
            "Public URL should not be blocked by SSRF protection, but got: {}",
            result.error.as_ref().unwrap()
        );
    }

    #[test]
    fn test_webhook_post_json_ssrf_check() {
        // Verify that post_json performs SSRF validation and returns a non-retryable
        // DeliveryResult when blocked, rather than attempting the HTTP request
        let wh = WebhookChannel::new("http://localhost:9090/admin".to_string());
        let body = serde_json::json!({"to": "user", "text": "test"});
        let result = wh.post_json(&body).unwrap();
        assert!(!result.ok, "SSRF-blocked request should not be ok");
        assert!(
            !result.retryable,
            "SSRF-blocked request should not be retryable"
        );
        assert!(result.message_id.is_none());
        assert!(result.conversation_id.is_none());
        let err = result.error.expect("should have error message");
        assert!(
            err.contains("webhook URL blocked by SSRF protection"),
            "Error should contain SSRF message, got: {}",
            err
        );
    }
}
