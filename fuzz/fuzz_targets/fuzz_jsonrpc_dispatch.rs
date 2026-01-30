#![no_main]

use libfuzzer_sys::fuzz_target;

use serde_json::Value;

/// Simulates the JSON-RPC frame parsing performed by the WebSocket handler.
///
/// The real `parse_request_frame` in `src/server/ws/mod.rs` is private, but
/// its logic is: parse JSON -> extract "type", "id", "method", "params" fields.
/// We replicate that extraction here to fuzz the same code path without needing
/// to export internal types.
///
/// This catches:
/// - Panics in serde_json on malformed input
/// - Unexpected panics from field extraction on adversarial JSON structures
/// - Memory issues from deeply nested or very large JSON payloads
fn parse_jsonrpc_frame(data: &[u8]) {
    // Step 1: Parse raw bytes as JSON (mirrors the WS handler's serde_json::from_slice)
    let value: Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => return, // Invalid JSON is fine, just not a panic
    };

    // Step 2: Extract fields the same way parse_request_frame does
    let obj = match value.as_object() {
        Some(o) => o,
        None => return,
    };

    // Extract "type" field and check it equals "req"
    let frame_type = obj.get("type").and_then(|v| v.as_str());
    if frame_type != Some("req") {
        return;
    }

    // Extract "id" field (must be a non-empty string)
    let id = obj.get("id").and_then(|v| v.as_str());
    let id = match id {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return,
    };

    // Extract "method" field (must be a non-empty string)
    let method = obj.get("method").and_then(|v| v.as_str());
    let _method = match method {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return,
    };

    // Extract optional "params" field
    let _params = obj.get("params").cloned();

    // If we got here, the frame parsed successfully.
    // Use id to prevent the compiler from optimizing away the work.
    let _ = id.len();
}

fuzz_target!(|data: &[u8]| {
    parse_jsonrpc_frame(data);
});
