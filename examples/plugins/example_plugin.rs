// Example WASM Plugin for Carapace
//
// This is a sample Rust plugin that can be compiled to WASM
// and loaded by the carapace gateway.
//
// To compile:
//   cargo build --target wasm32-wasi --release
//
// Or use wasm-pack:
//   wasm-pack build --target wasi

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Plugin metadata - required
#[no_mangle]
pub static PLUGIN_NAME: &str = "example-plugin";

#[no_mangle]
pub static PLUGIN_VERSION: &str = "0.1.0";

#[no_mangle]
pub static PLUGIN_DESCRIPTION: &str = "An example plugin demonstrating WASM plugin development";

// Plugin input/output types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreetInput {
    pub name: String,
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreetOutput {
    pub greeting: String,
    pub timestamp: i64,
}

// Host functions that plugins can call
// These are provided by the carapace host
extern "C" {
    fn log_message(level: i32, message: *const u8, message_len: i32);
    fn get_config(key: *const u8, key_len: i32) -> i32; // Returns JSON result pointer
    fn free_string(ptr: i32);
}

// Helper function to log messages from the plugin
fn log(level: &str, message: &str) {
    let level_num = match level {
        "debug" => 0,
        "info" => 1,
        "warn" => 2,
        "error" => 3,
        _ => 1,
    };
    unsafe {
        log_message(level_num, message.as_ptr(), message.len() as i32);
    }
}

// Tool: greet - A simple greeting tool
#[no_mangle]
pub extern "C" fn tool_greet(
    input_ptr: i32,  // JSON input as pointer
    input_len: i32,  // Input length
    output_ptr: *mut i32,  // Output pointer to set
) -> i32 {
    // Read input
    let input_json = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        std::str::from_utf8(slice).unwrap_or("{}")
    };

    log("info", &format!("greet tool called with input: {}", input_json));

    // Parse input
    let input: GreetInput = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            log("error", &format!("Failed to parse input: {}", e));
            return -1; // Error
        }
    };

    // Generate greeting
    let prefix = input.prefix.unwrap_or_else(|| "Hello".to_string());
    let greeting = format!("{}, {}!", prefix, input.name);
    let timestamp = chrono::Utc::now().timestamp();

    // Create output
    let output = GreetOutput {
        greeting,
        timestamp,
    };

    // Serialize output to JSON
    let output_json = match serde_json::to_string(&output) {
        Ok(s) => s,
        Err(e) => {
            log("error", &format!("Failed to serialize output: {}", e));
            return -1;
        }
    };

    // Allocate output in host memory and return pointer
    // In a real implementation, this would use the host's memory allocator
    let result_ptr = output_json.as_ptr() as i32;
    let result_len = output_json.len() as i32;

    // For this example, we're returning the string directly
    // Real implementation would copy to host-allocated memory
    unsafe {
        output_ptr.write(result_ptr);
        output_len.write(result_len);
    }

    0 // Success
}

// Tool: echo - Echo back the input with modifications
#[no_mangle]
pub extern "C" fn tool_echo(
    input_ptr: i32,
    input_len: i32,
    output_ptr: *mut i32,
    output_len: *mut i32,
) -> i32 {
    let input_json = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        std::str::from_utf8(slice).unwrap_or("{}")
    };

    log("debug", &format!("echo tool received: {}", input_json));

    // Echo back with a prefix
    let echo = format!("[ECHO] {}", input_json);

    unsafe {
        output_ptr.write(echo.as_ptr() as i32);
        output_len.write(echo.len() as i32);
    }

    0
}

// Tool: get_plugin_info - Return information about this plugin
#[no_mangle]
pub extern "C" fn tool_get_info(
    _input_ptr: i32,
    _input_len: i32,
    output_ptr: *mut i32,
    output_len: *mut i32,
) -> i32 {
    let info = serde_json::json!({
        "name": PLUGIN_NAME,
        "version": PLUGIN_VERSION,
        "description": PLUGIN_DESCRIPTION,
        "tools": [
            {"name": "greet", "description": "Generate a personalized greeting"},
            {"name": "echo", "description": "Echo back the input message"},
            {"name": "get_info", "description": "Get plugin information"}
        ]
    });

    let info_str = info.to_string();

    unsafe {
        output_ptr.write(info_str.as_ptr() as i32);
        output_len.write(info_str.len() as i32);
    }

    0
}

// Initialize the plugin (called once when loaded)
#[no_mangle]
pub extern "C" fn init() -> i32 {
    log("info", &format!("Initializing plugin: {} v{}", PLUGIN_NAME, PLUGIN_VERSION));
    0
}

// Shutdown the plugin (called once when unloaded)
#[no_mangle]
pub extern "C" fn shutdown() -> i32 {
    log("info", &format!("Shutting down plugin: {}", PLUGIN_NAME));
    0
}
