//! Shared helpers for plugin hook dispatch.

use serde_json::Value;
use std::sync::Arc;

use super::{HookDispatchResult, HookDispatcher, PluginRegistry};

pub fn dispatch_hook(
    registry: Arc<PluginRegistry>,
    hook_name: &str,
    payload: &Value,
) -> Option<HookDispatchResult> {
    let dispatcher = HookDispatcher::new(registry);
    let payload = match serde_json::to_string(payload) {
        Ok(payload) => payload,
        Err(err) => {
            tracing::warn!(hook = %hook_name, error = %err, "Failed to serialize hook payload");
            return None;
        }
    };

    match dispatcher.dispatch(hook_name, &payload) {
        Ok(result) => Some(result),
        Err(err) => {
            tracing::warn!(hook = %hook_name, error = %err, "Hook dispatch failed");
            None
        }
    }
}

pub fn parse_hook_payload(result: &HookDispatchResult, hook_name: &str) -> Option<Value> {
    let payload = result.final_payload.as_ref()?;
    match serde_json::from_str(payload) {
        Ok(payload) => Some(payload),
        Err(err) => {
            tracing::warn!(hook = %hook_name, error = %err, "Hook returned invalid JSON payload");
            None
        }
    }
}
