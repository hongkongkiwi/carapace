//! Message delivery worker.
//!
//! Background loop that drains the outbound message pipeline and delivers
//! messages via channel plugins. Wakes on `Notify` or periodic 5-second poll.

use std::sync::Arc;
use std::time::Duration;

use tracing::warn;

use crate::channels::ChannelRegistry;
use crate::messages::outbound::{MessageContent, MessagePipeline};
use crate::plugins::{self, OutboundContext, PluginRegistry};
use crate::server::ws::WsServerState;

/// Run the delivery worker loop.
///
/// Wakes when notified by the pipeline, every 5 seconds, or on shutdown.
pub async fn delivery_loop(
    pipeline: Arc<MessagePipeline>,
    plugin_registry: Arc<PluginRegistry>,
    channel_registry: Arc<ChannelRegistry>,
    _state: Arc<WsServerState>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        // Wait for notification, timeout, or shutdown
        tokio::select! {
            _ = pipeline.notifier().notified() => {}
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            _ = shutdown.changed() => {
                break;
            }
        }

        // Check shutdown after waking
        if *shutdown.borrow() {
            break;
        }

        let channel_ids = pipeline.channels_with_messages();

        process_channel_messages(&channel_ids, &pipeline, &plugin_registry, &channel_registry)
            .await;
    }
}

/// Process pending messages for each connected channel.
async fn process_channel_messages(
    channel_ids: &[String],
    pipeline: &MessagePipeline,
    plugin_registry: &PluginRegistry,
    channel_registry: &ChannelRegistry,
) {
    for channel_id in channel_ids {
        if !channel_registry.is_connected(channel_id) {
            continue;
        }

        let msg = match pipeline.next_for_channel(channel_id) {
            Some(m) => m,
            None => continue,
        };

        let message_id = msg.message.id.clone();

        if let Err(e) = pipeline.mark_sending(&message_id) {
            warn!(id = %message_id, error = %e, "failed to mark message as sending");
            continue;
        }

        let plugin = match plugin_registry.get_channel(channel_id) {
            Some(p) => p,
            None => {
                let _ = pipeline.mark_failed(&message_id, "no plugin registered for channel");
                continue;
            }
        };

        let metadata = &msg.message.metadata;

        let result = deliver_message(
            &plugin,
            &msg.message.content,
            metadata.recipient_id.as_deref().unwrap_or_default(),
            metadata.reply_to.as_deref(),
            metadata.thread_id.as_deref(),
        )
        .await;

        handle_delivery_result(pipeline, &message_id, result);
    }
}

/// Handle the result of a message delivery attempt.
fn handle_delivery_result(
    pipeline: &MessagePipeline,
    message_id: &crate::messages::outbound::MessageId,
    result: Result<plugins::DeliveryResult, plugins::BindingError>,
) {
    match result {
        Ok(delivery) if delivery.ok => {
            let _ = pipeline.mark_sent(message_id);
        }
        Ok(delivery) => {
            let error = delivery
                .error
                .unwrap_or_else(|| "delivery failed".to_string());
            if delivery.retryable && pipeline.can_retry(message_id) {
                let _ = pipeline.mark_retry(message_id, &error);
                warn!(
                    id = %message_id,
                    error = %error,
                    "retryable delivery failure, reset to queued for retry"
                );
            } else {
                let _ = pipeline.mark_failed(message_id, &error);
            }
        }
        Err(e) => {
            let _ = pipeline.mark_failed(message_id, e.to_string());
        }
    }
}

/// Deliver a message via the channel plugin, dispatching to send_text or send_media.
///
/// `ChannelPluginInstance` methods are sync, so we run them via `spawn_blocking`.
async fn deliver_message(
    plugin: &Arc<dyn plugins::ChannelPluginInstance>,
    content: &MessageContent,
    to: &str,
    reply_to_id: Option<&str>,
    thread_id: Option<&str>,
) -> Result<plugins::DeliveryResult, plugins::BindingError> {
    match content {
        MessageContent::Text { text } => {
            let ctx = OutboundContext {
                to: to.to_string(),
                text: text.clone(),
                media_url: None,
                gif_playback: false,
                reply_to_id: reply_to_id.map(|s| s.to_string()),
                thread_id: thread_id.map(|s| s.to_string()),
                account_id: None,
            };
            let p = plugin.clone();
            tokio::task::spawn_blocking(move || p.send_text(ctx))
                .await
                .map_err(|e| plugins::BindingError::CallError(e.to_string()))?
        }
        MessageContent::Media {
            caption, media_ref, ..
        } => {
            let ctx = OutboundContext {
                to: to.to_string(),
                text: caption.clone().unwrap_or_default(),
                media_url: Some(media_ref.clone()),
                gif_playback: false,
                reply_to_id: reply_to_id.map(|s| s.to_string()),
                thread_id: thread_id.map(|s| s.to_string()),
                account_id: None,
            };
            let p = plugin.clone();
            tokio::task::spawn_blocking(move || p.send_media(ctx))
                .await
                .map_err(|e| plugins::BindingError::CallError(e.to_string()))?
        }
        MessageContent::Composite { parts } => {
            // Send each part sequentially; return first failure or last success
            let mut last_result = plugins::DeliveryResult {
                ok: true,
                message_id: None,
                error: None,
                retryable: false,
                conversation_id: None,
                to_jid: None,
                poll_id: None,
            };
            for part in parts {
                last_result =
                    Box::pin(deliver_message(plugin, part, to, reply_to_id, thread_id)).await?;
                if !last_result.ok {
                    return Ok(last_result);
                }
            }
            Ok(last_result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::outbound::{
        MessageContent, OutboundContext as MsgOutboundContext, OutboundMessage,
    };
    use crate::plugins::{
        BindingError, ChannelCapabilities, ChannelPluginInstance, DeliveryResult, OutboundContext,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Mock channel plugin that records calls.
    struct MockChannel {
        send_text_count: AtomicU32,
        send_media_count: AtomicU32,
        fail: bool,
        retryable: bool,
    }

    impl MockChannel {
        fn new() -> Self {
            Self {
                send_text_count: AtomicU32::new(0),
                send_media_count: AtomicU32::new(0),
                fail: false,
                retryable: false,
            }
        }

        fn failing(retryable: bool) -> Self {
            Self {
                send_text_count: AtomicU32::new(0),
                send_media_count: AtomicU32::new(0),
                fail: true,
                retryable,
            }
        }
    }

    impl ChannelPluginInstance for MockChannel {
        fn get_info(&self) -> Result<plugins::ChannelInfo, BindingError> {
            Ok(plugins::ChannelInfo {
                id: "mock".to_string(),
                label: "Mock".to_string(),
                selection_label: "Mock Channel".to_string(),
                docs_path: "".to_string(),
                blurb: "".to_string(),
                order: 0,
            })
        }

        fn get_capabilities(&self) -> Result<ChannelCapabilities, BindingError> {
            Ok(ChannelCapabilities::default())
        }

        fn send_text(&self, _ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
            self.send_text_count.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                Ok(DeliveryResult {
                    ok: false,
                    message_id: None,
                    error: Some("mock failure".to_string()),
                    retryable: self.retryable,
                    conversation_id: None,
                    to_jid: None,
                    poll_id: None,
                })
            } else {
                Ok(DeliveryResult {
                    ok: true,
                    message_id: Some("sent-1".to_string()),
                    error: None,
                    retryable: false,
                    conversation_id: None,
                    to_jid: None,
                    poll_id: None,
                })
            }
        }

        fn send_media(&self, _ctx: OutboundContext) -> Result<DeliveryResult, BindingError> {
            self.send_media_count.fetch_add(1, Ordering::Relaxed);
            Ok(DeliveryResult {
                ok: true,
                message_id: Some("sent-media-1".to_string()),
                error: None,
                retryable: false,
                conversation_id: None,
                to_jid: None,
                poll_id: None,
            })
        }
    }

    fn make_pipeline_and_registries(
        channel_id: &str,
        plugin: Option<Arc<dyn ChannelPluginInstance>>,
        connected: bool,
    ) -> (
        Arc<MessagePipeline>,
        Arc<PluginRegistry>,
        Arc<ChannelRegistry>,
    ) {
        let pipeline = Arc::new(MessagePipeline::new());

        let plugin_registry = Arc::new(PluginRegistry::new());
        if let Some(p) = plugin {
            plugin_registry.register_channel(channel_id.to_string(), p);
        }

        let channel_registry = Arc::new(ChannelRegistry::new());
        let status = if connected {
            crate::channels::ChannelStatus::Connected
        } else {
            crate::channels::ChannelStatus::Disconnected
        };
        channel_registry.register(
            crate::channels::ChannelInfo::new(channel_id, channel_id).with_status(status),
        );

        (pipeline, plugin_registry, channel_registry)
    }

    #[tokio::test]
    async fn test_delivery_sends_text_message() {
        let mock = Arc::new(MockChannel::new());
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("test-ch", Some(mock.clone()), true);

        // Queue a text message
        let msg = OutboundMessage::new("test-ch", MessageContent::text("hello"));
        pipeline.queue(msg, MsgOutboundContext::new()).unwrap();

        // Run one iteration (use shutdown to stop after one pass)
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let pl = pipeline.clone();
        let pr = plugin_reg.clone();
        let cr = channel_reg.clone();
        let st = state.clone();
        let handle = tokio::spawn(async move {
            delivery_loop(pl, pr, cr, st, shutdown_rx).await;
        });

        // Give it time to process
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = shutdown_tx.send(true);
        // Notify to unblock select
        pipeline.notifier().notify_one();
        let _ = handle.await;

        assert_eq!(mock.send_text_count.load(Ordering::Relaxed), 1);
        assert_eq!(pipeline.channels_with_messages().len(), 0);
    }

    #[tokio::test]
    async fn test_delivery_marks_failed_no_plugin() {
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("no-plugin-ch", None, true);

        let msg = OutboundMessage::new("no-plugin-ch", MessageContent::text("hello"));
        let result = pipeline.queue(msg, MsgOutboundContext::new()).unwrap();

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let pl = pipeline.clone();
        let handle = tokio::spawn(async move {
            delivery_loop(pl, plugin_reg, channel_reg, state, shutdown_rx).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = shutdown_tx.send(true);
        pipeline.notifier().notify_one();
        let _ = handle.await;

        let status = pipeline.get_status(&result.message_id);
        assert_eq!(
            status,
            Some(crate::messages::outbound::DeliveryStatus::Failed)
        );
    }

    #[tokio::test]
    async fn test_delivery_skips_disconnected_channel() {
        let mock = Arc::new(MockChannel::new());
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("disc-ch", Some(mock.clone()), false);

        let msg = OutboundMessage::new("disc-ch", MessageContent::text("hello"));
        pipeline.queue(msg, MsgOutboundContext::new()).unwrap();

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let pl = pipeline.clone();
        let handle = tokio::spawn(async move {
            delivery_loop(pl, plugin_reg, channel_reg, state, shutdown_rx).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = shutdown_tx.send(true);
        pipeline.notifier().notify_one();
        let _ = handle.await;

        // Message should still be queued (not sent, not failed)
        assert_eq!(mock.send_text_count.load(Ordering::Relaxed), 0);
        assert_eq!(pipeline.channels_with_messages().len(), 1);
    }

    #[tokio::test]
    async fn test_delivery_retries_on_retryable_failure_resets_status() {
        let mock = Arc::new(MockChannel::failing(true));
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("retry-ch", Some(mock.clone()), true);

        let msg = OutboundMessage::new("retry-ch", MessageContent::text("hello"));
        let ctx = MsgOutboundContext::new().with_retries(3);
        let result = pipeline.queue(msg, ctx).unwrap();

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let pl = pipeline.clone();
        let handle = tokio::spawn(async move {
            delivery_loop(pl, plugin_reg, channel_reg, state, shutdown_rx).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = shutdown_tx.send(true);
        pipeline.notifier().notify_one();
        let _ = handle.await;

        // Message status must be reset to Queued (not stuck at Sending, not Failed)
        let status = pipeline.get_status(&result.message_id);
        assert_eq!(
            status,
            Some(crate::messages::outbound::DeliveryStatus::Queued),
            "retryable failure must reset status to Queued, not leave it as Sending"
        );

        // Message should still be in the channel queue for retry
        assert_eq!(
            pipeline.channels_with_messages().len(),
            1,
            "message should remain in channel queue after retryable failure"
        );

        // The error from the failed attempt should be recorded
        let queued = pipeline.get_message(&result.message_id).unwrap();
        assert_eq!(queued.last_error, Some("mock failure".to_string()));
    }

    #[tokio::test]
    async fn test_delivery_non_retryable_failure_marks_failed() {
        let mock = Arc::new(MockChannel::failing(false));
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("noretry-ch", Some(mock.clone()), true);

        let msg = OutboundMessage::new("noretry-ch", MessageContent::text("hello"));
        let ctx = MsgOutboundContext::new().with_retries(3);
        let result = pipeline.queue(msg, ctx).unwrap();

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let pl = pipeline.clone();
        let handle = tokio::spawn(async move {
            delivery_loop(pl, plugin_reg, channel_reg, state, shutdown_rx).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = shutdown_tx.send(true);
        pipeline.notifier().notify_one();
        let _ = handle.await;

        // Non-retryable failure must be marked as Failed
        let status = pipeline.get_status(&result.message_id);
        assert_eq!(
            status,
            Some(crate::messages::outbound::DeliveryStatus::Failed),
            "non-retryable failure must be marked as Failed"
        );

        // Message should be removed from the channel queue
        assert_eq!(
            pipeline.queue_size("noretry-ch"),
            0,
            "failed message should be removed from channel queue"
        );
    }

    #[tokio::test]
    async fn test_retry_mechanism_picks_up_reset_messages() {
        // Use a mock that always fails with retryable=true
        let mock = Arc::new(MockChannel::failing(true));
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("pickup-ch", Some(mock.clone()), true);

        let msg = OutboundMessage::new("pickup-ch", MessageContent::text("hello"));
        // Allow 3 retries so the message can be retried multiple times
        let ctx = MsgOutboundContext::new().with_retries(3);
        let result = pipeline.queue(msg, ctx).unwrap();

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let pl = pipeline.clone();
        let handle = tokio::spawn(async move {
            delivery_loop(pl, plugin_reg, channel_reg, state, shutdown_rx).await;
        });

        // Allow enough time for multiple delivery loop iterations to run.
        // The loop wakes on notify or every 5s; we notify it repeatedly.
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(80)).await;
            pipeline.notifier().notify_one();
        }

        let _ = shutdown_tx.send(true);
        pipeline.notifier().notify_one();
        let _ = handle.await;

        // The mock should have been called more than once, proving the retry
        // mechanism picked up the message again after it was reset to Queued.
        let send_count = mock.send_text_count.load(Ordering::Relaxed);
        assert!(
            send_count > 1,
            "expected multiple delivery attempts from retry, got {}",
            send_count
        );

        // After exhausting retries (3 attempts), the message should be Failed
        // since can_retry() returns false when attempts >= max_retries.
        let queued = pipeline.get_message(&result.message_id).unwrap();
        assert_eq!(
            queued.status,
            crate::messages::outbound::DeliveryStatus::Failed,
            "message should be Failed after exhausting retries (attempts={}, max=3)",
            queued.attempts
        );
    }

    #[tokio::test]
    async fn test_delivery_shutdown() {
        let (pipeline, plugin_reg, channel_reg) =
            make_pipeline_and_registries("shutdown-ch", None, true);

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(true); // already shut down
        let state = Arc::new(crate::server::ws::WsServerState::new(
            crate::server::ws::WsServerConfig::default(),
        ));

        let handle = tokio::spawn(async move {
            delivery_loop(pipeline, plugin_reg, channel_reg, state, shutdown_rx).await;
        });

        // Should exit quickly since shutdown is already true
        let _ = shutdown_tx.send(true);
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("delivery loop should exit on shutdown")
            .expect("task should not panic");
    }
}
