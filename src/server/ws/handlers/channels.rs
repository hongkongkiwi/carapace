//! Channel handlers.

use serde_json::{json, Value};

use super::super::*;

pub(super) fn handle_channels_status(state: &WsServerState) -> Result<Value, ErrorShape> {
    let snapshot = state.channel_registry.snapshot();
    let connected = snapshot
        .channels
        .iter()
        .filter(|c| c.status == channels::ChannelStatus::Connected)
        .count();
    Ok(json!({
        "total": snapshot.channels.len(),
        "connected": connected,
        "channels": snapshot.channels,
        "ts": snapshot.timestamp
    }))
}

pub(super) fn handle_channels_logout(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let channel = params
        .and_then(|v| v.get("channel").or_else(|| v.get("channelId")))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "channel is required", None))?;
    if !state.channel_registry.logout(channel) {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "unknown channel",
            Some(json!({ "channel": channel })),
        ));
    }
    Ok(json!({
        "ok": true,
        "channel": channel
    }))
}
