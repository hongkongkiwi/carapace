//! TypeScript/Moltbot Plugin Compatibility Layer
//!
//! This module provides compatibility utilities for Moltbot TypeScript plugins.
//! TypeScript plugins can run in Carapace via Node.js/Deno subprocesses.

use crate::plugins::loader::{LoaderError, PluginKind, PluginManifest};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// TypeScript plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypescriptPluginConfig {
    /// Path to the plugin directory or entry file
    pub entry_path: PathBuf,
    /// Runtime to use: "node" or "deno"
    pub runtime: Option<String>,
    /// Environment variables to pass
    pub env: std::collections::HashMap<String, String>,
    /// Timeout for plugin initialization (seconds)
    pub init_timeout: u64,
}

/// Moltbot plugin SDK shim for Node.js/Deno
/// This shim provides IPC-based communication with Carapace
pub const MOLTBOT_SDK_SHIM: &str = r#"
/**
 * Moltbot Plugin SDK Shim for Carapace
 * Provides compatibility layer for Moltbot plugins
 */

// Plugin metadata
let pluginId = process.env.CARAPACE_PLUGIN_ID || 'unknown';
let pluginVersion = process.env.CARAPACE_PLUGIN_VERSION || '0.1.0';

// IPC communication via stdin/stdout
function sendIPC(message) {
    return new Promise((resolve, reject) => {
        const callId = message.call_id || (crypto.randomUUID?.() || Date.now().toString());
        message.call_id = callId;

        // Write to stdout (Carapace reads from plugin stdout)
        console.log(JSON.stringify(message));
        console.log('\n');

        // For now, just resolve - full IPC would need a companion reader
        resolve({ success: true });
    });
}

// Initialize plugin
export async function init(metadata) {
    return sendIPC({
        type: 'Init',
        metadata
    });
}

// Tool call
export async function callTool(name, args) {
    return sendIPC({
        type: 'ToolCall',
        name,
        args
    });
}

// Hook handler
export async function onHook(event, payload) {
    return sendIPC({
        type: 'HookCall',
        event,
        payload
    });
}

// Logger
export const logger = {
    info: (msg) => console.log(JSON.stringify({ type: 'Log', level: 'info', message: msg })),
    error: (msg) => console.log(JSON.stringify({ type: 'Log', level: 'error', message: msg })),
    warn: (msg) => console.log(JSON.stringify({ type: 'Log', level: 'warn', message: msg })),
    debug: (msg) => console.log(JSON.stringify({ type: 'Log', level: 'debug', message: msg })),
};

export { pluginId, pluginVersion };
"#;

/// Create a Moltbot-compatible plugin template
pub fn create_moltbot_plugin_template(name: &str, description: &str) -> String {
    format!(
        r#"/**
 * {name}
 * {description}
 *
 * Compatible with Moltbot Plugin SDK via Carapace
 */

import {{ init, callTool, onHook, logger }} from "@carapace/sdk";

// Plugin configuration
const config = {{
    name: "{name}",
    version: "0.1.0",
    description: "{description}",
}};

// Initialize plugin
async function main() {{
    logger.info(`Starting plugin: ${{config.name}} v${{config.version}}`);

    await init({{
        id: "{name}".toLowerCase().replace(/\s+/g, '-'),
        name: config.name,
        version: config.version,
        description: config.description,
    }});

    logger.info("Plugin initialized successfully");
}}

// Tool definitions
export const tools = {{
    // Example tool:
    // myTool: async (args) => {{
    //     return callTool('builtin-example', args);
    // }}
}};

// Hook handlers
export const hooks = {{
    // Example hook:
    // onMessage: async (ctx) => {{
    //     logger.info(`Received message: ${{ctx.content}}`);
    //     return ctx;
    // }}
}};

// Run the plugin
main().catch(err => {{
    logger.error(`Plugin failed: ${{err.message}}`);
    process.exit(1);
}});
"#
    )
}

/// TypescriptPluginInstance - placeholder for future implementation
pub struct TypescriptPluginInstance;

/// TypescriptPluginLoader - placeholder for future implementation
pub struct TypescriptPluginLoader;

impl TypescriptPluginLoader {
    /// Create a new TypeScript plugin loader
    pub fn new(_config: TypescriptPluginConfig) -> Self {
        Self
    }

    /// Load a plugin from a directory or file
    pub async fn load(&self, path: &Path) -> Result<PluginManifest, LoaderError> {
        let manifest_path = path.join("plugin.json");

        if !manifest_path.exists() {
            // Create a default manifest
            let id = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
                .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-");

            let name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            return Ok(PluginManifest {
                id,
                version: "0.1.0".to_string(),
                name,
                description: "TypeScript plugin".to_string(),
                kind: PluginKind::Service,
            });
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| LoaderError::InvalidManifest {
                plugin_id: path.to_string_lossy().to_string(),
                message: e.to_string(),
            })?;

        let manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(|e| LoaderError::InvalidManifest {
                plugin_id: path.to_string_lossy().to_string(),
                message: e.to_string(),
            })?;

        Ok(manifest)
    }
}
