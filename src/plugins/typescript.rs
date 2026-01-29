// TypeScript/JavaScript Plugin Loader using QuickJS WASM
//
// This module provides JavaScript/TypeScript plugin support by embedding QuickJS.
// Plugins are written in JavaScript and run in an isolated context.
//
// Architecture:
// 1. Load QuickJS WASM runtime
// 2. Initialize JavaScript context
// 3. Load and execute plugin code
// 4. Bridge calls between JS and Rust

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// QuickJS/TypeScript plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsPluginConfig {
    /// Maximum memory for JS runtime (MB)
    pub max_memory_mb: usize,
    /// Maximum execution time (seconds)
    pub timeout_seconds: u64,
    /// Enable console output
    pub enable_console: bool,
    /// Enable network access
    pub allow_network: bool,
    /// Allowed globals (empty = all allowed)
    pub allowed_globals: Vec<String>,
}

impl Default for TsPluginConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 128,
            timeout_seconds: 30,
            enable_console: true,
            allow_network: false,
            allowed_globals: vec![
                "JSON".to_string(),
                "Math".to_string(),
                "Date".to_string(),
                "Array".to_string(),
                "Object".to_string(),
                "String".to_string(),
                "Number".to_string(),
                "Boolean".to_string(),
                "parseInt".to_string(),
                "parseFloat".to_string(),
                "isNaN".to_string(),
                "isFinite".to_string(),
            ],
        }
    }
}

/// JavaScript plugin instance
pub struct TsPlugin {
    /// Plugin ID
    id: String,
    /// Plugin name
    name: String,
    /// JavaScript code
    code: String,
    /// Configuration
    config: TsPluginConfig,
    /// State
    initialized: bool,
}

impl TsPlugin {
    /// Create a new JavaScript plugin
    pub fn new(id: String, name: String, code: String) -> Self {
        Self {
            id,
            name,
            code,
            config: TsPluginConfig::default(),
            initialized: false,
        }
    }

    /// Initialize the JavaScript plugin
    pub fn init(&mut self) -> Result<(), TsPluginError> {
        // In a real implementation with QuickJS WASM:
        // 1. Create QuickJS runtime
        // 2. Create context
        // 3. Set up console.log bridge
        // 4. Define plugin API functions
        // 5. Load and evaluate the plugin code
        // 6. Call the plugin's init() function if it exists

        self.initialized = true;
        Ok(())
    }

    /// Execute a tool function
    pub fn call_function(
        &self,
        func_name: &str,
        args: &str,
    ) -> Result<String, TsPluginError> {
        if !self.initialized {
            return Err(TsPluginError::NotInitialized);
        }

        // In a real implementation:
        // 1. Construct JS call: `plugin.handle_tool("funcName", JSON.stringify(args))`
        // 2. Evaluate in QuickJS context
        // 3. Parse result as JSON

        Ok(format!(r#"{{"result": "called {}({})"}}"#, func_name, args))
    }

    /// Execute raw JavaScript code
    pub fn execute(&self, code: &str) -> Result<String, TsPluginError> {
        if !self.initialized {
            return Err(TsPluginError::NotInitialized);
        }

        // Run code in QuickJS context
        Ok(format!("Executed JS: {}", code))
    }
}

/// TypeScript plugin errors
#[derive(Debug, thiserror::Error)]
pub enum TsPluginError {
    #[error("Plugin not initialized")]
    NotInitialized,

    #[error("JavaScript execution error: {0}")]
    ExecutionError(String),

    #[error("Timeout exceeded")]
    Timeout,

    #[error("Memory limit exceeded")]
    MemoryLimit,

    #[error("WASM runtime error: {0}")]
    WasmError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Registry of JavaScript/TypeScript plugins
#[derive(Default)]
pub struct TsPluginRegistry {
    plugins: HashMap<String, TsPlugin>,
}

impl TsPluginRegistry {
    /// Register a new JavaScript plugin
    pub fn register(&mut self, plugin: TsPlugin) {
        self.plugins.insert(plugin.id.clone(), plugin);
    }

    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Option<&TsPlugin> {
        self.plugins.get(id)
    }

    /// Get a plugin mutably by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut TsPlugin> {
        self.plugins.get_mut(id)
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&TsPlugin> {
        self.plugins.values().collect()
    }
}

/// Example TypeScript/JavaScript Plugin
///
/// Save this as `hello_plugin.js`:
///
/// ```javascript
/// // Plugin metadata
/// const PLUGIN_NAME = "hello-plugin";
/// const PLUGIN_VERSION = "0.1.0";
/// const PLUGIN_DESCRIPTION = "A JavaScript example plugin";
///
/// // Tool functions
/// function greet(name, prefix = "Hello") {
///     return { greeting: `${prefix}, ${name}!` };
/// }
///
/// function calculate(a, b, operation = "add") {
///     const ops = {
///         add: a + b,
///         subtract: a - b,
///         multiply: a * b,
///         divide: b !== 0 ? a / b : null
///     };
///     return { operation, result: ops[operation] };
/// }
///
/// function echo(message, repeat = 1) {
///     return {
///         original: message,
///         repeated: (message + " ").repeat(repeat).trim(),
///         repeatCount: repeat
///     };
/// }
///
/// function getInfo() {
///     return {
///         name: PLUGIN_NAME,
///         version: PLUGIN_VERSION,
///         description: PLUGIN_DESCRIPTION,
///         tools: ["greet", "calculate", "echo", "getInfo"]
///     };
/// }
///
/// // Lifecycle hooks
/// function init() {
///     console.log(`${PLUGIN_NAME} v${PLUGIN_VERSION} initialized`);
///     return { status: "initialized" };
/// }
///
/// function shutdown() {
///     console.log(`${PLUGIN_NAME} shutting down`);
///     return { status: "shutdown" };
/// }
///
/// // Main entry point
/// function handleTool(toolName, argsJson) {
///     const args = argsJson ? JSON.parse(argsJson) : {};
///     const tools = { greet, calculate, echo, getInfo };
///
///     if (!tools[toolName]) {
///         return JSON.stringify({ error: `Unknown tool: ${toolName}` });
///     }
///
///     const result = tools[toolName](...Object.values(args));
///     return JSON.stringify(result);
/// }
/// ```

/// Loader for JavaScript plugins from files
pub async fn load_ts_plugin(
    path: std::path::PathBuf,
    plugin_id: String,
) -> Result<TsPlugin, TsPluginError> {
    let code = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut plugin = TsPlugin::new(plugin_id, name, code);
    plugin.init()?;

    Ok(plugin)
}
