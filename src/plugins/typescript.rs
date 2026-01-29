// TypeScript/JavaScript Plugin Loader using QuickJS WASM
//
// This module provides JavaScript/TypeScript plugin support by embedding QuickJS.
// Plugins are written in JavaScript and run in an isolated context.
//
// Architecture:
// 1. Load QuickJS WASM runtime
// 2. Initialize JavaScript context
// 3. Set up host function bindings
// 4. Load and evaluate plugin code
// 5. Bridge calls between JS and Rust

use quickjs_wasm_sys::{JSContext, JSRuntime, JSValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::rc::Rc;
use thiserror::Error;

/// QuickJS/TypeScript plugin configuration
#[derive(Debug, Clone)]
pub struct TsPluginConfig {
    /// Maximum memory for JS runtime (MB)
    pub max_memory_mb: usize,
    /// Maximum execution time (seconds)
    pub timeout_seconds: u64,
    /// Enable console output
    pub enable_console: bool,
    /// Enable network access
    pub allow_network: bool,
    /// Enable eval() function
    pub allow_eval: bool,
    /// Maximum string length
    pub max_string_length: usize,
    /// Maximum array length
    pub max_array_length: usize,
    /// Maximum object depth
    pub max_object_depth: usize,
}

impl Default for TsPluginConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 128,
            timeout_seconds: 30,
            enable_console: true,
            allow_network: false,
            allow_eval: false,
            max_string_length: 1_000_000,
            max_array_length: 100_000,
            max_object_depth: 100,
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
    /// Runtime
    runtime: Option<JsRuntime>,
    /// Context
    context: Option<JsContextWrapper>,
    /// Initialized state
    initialized: bool,
    /// Available tools
    tools: Vec<String>,
}

struct JsRuntime {
    runtime: *mut JSRuntime,
}

struct JsContextWrapper {
    context: *mut JSContext,
    /// Host function state
    host_state: RefCell<HostState>,
}

struct HostState {
    /// Console output enabled
    console_enabled: bool,
    /// Log buffer for capturing console output
    log_buffer: Vec<String>,
    /// Last error
    last_error: Option<String>,
    /// Callbacks for host functions
    tool_callbacks: HashMap<String, Box<dyn Fn(String) -> Result<String, String> + 'static>>,
}

/// TypeScript plugin errors
#[derive(Debug, Error)]
pub enum TsPluginError {
    #[error("Plugin not initialized")]
    NotInitialized,

    #[error("JavaScript execution error: {0}")]
    ExecutionError(String),

    #[error("QuickJS runtime error: {0}")]
    QuickJsError(String),

    #[error("Timeout exceeded")]
    Timeout,

    #[error("Memory limit exceeded")]
    MemoryLimit,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("IO error: {0}")]
    IoError(String),
}

unsafe impl Send for JsRuntime {}
unsafe impl Send for JsContextWrapper {}

impl Drop for JsRuntime {
    fn drop(&mut self) {
        unsafe {
            quickjs_wasm_sys::JS_FreeRuntime(self.runtime);
        }
    }
}

impl Drop for JsContextWrapper {
    fn drop(&mut self) {
        unsafe {
            quickjs_wasm_sys::JS_FreeContext(self.context);
        }
    }
}

impl TsPlugin {
    /// Create a new JavaScript plugin
    pub fn new(id: String, name: String, code: String) -> Self {
        Self {
            id,
            name,
            code,
            config: TsPluginConfig::default(),
            runtime: None,
            context: None,
            initialized: false,
            tools: Vec::new(),
        }
    }

    /// Create a new JavaScript plugin with custom config
    pub fn with_config(id: String, name: String, code: String, config: TsPluginConfig) -> Self {
        Self {
            id,
            name,
            code,
            config,
            runtime: None,
            context: None,
            initialized: false,
            tools: Vec::new(),
        }
    }

    /// Initialize the JavaScript plugin
    pub fn init(&mut self) -> Result<(), TsPluginError> {
        // Create QuickJS runtime
        let runtime = unsafe {
            let rt = quickjs_wasm_sys::JS_NewRuntime();
            if rt.is_null() {
                return Err(TsPluginError::QuickJsError("Failed to create runtime".to_string()));
            }
            JsRuntime { runtime: rt }
        };

        // Set memory limit
        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        unsafe {
            quickjs_wasm_sys::JS_SetMemoryLimit(runtime.runtime, max_memory_bytes as i64);
        }

        // Create context
        let context = unsafe {
            let ctx = quickjs_wasm_sys::JS_NewContext(runtime.runtime);
            if ctx.is_null() {
                return Err(TsPluginError::QuickJsError("Failed to create context".to_string()));
            }
            JsContextWrapper {
                context: ctx,
                host_state: RefCell::new(HostState {
                    console_enabled: self.config.enable_console,
                    log_buffer: Vec::new(),
                    last_error: None,
                    tool_callbacks: HashMap::new(),
                }),
            }
        };

        // Set up global functions
        self.setup_globals(&context)?;

        // Load and evaluate plugin code
        self.evaluate_plugin_code(&context)?;

        // Call the plugin's init() function if it exists
        self.call_init(&context)?;

        // Get available tools
        self.discover_tools(&context)?;

        self.runtime = Some(runtime);
        self.context = Some(context);
        self.initialized = true;

        Ok(())
    }

    /// Set up global functions and objects
    fn setup_globals(&self, context: &JsContextWrapper) -> Result<(), TsPluginError> {
        unsafe {
            let global = quickjs_wasm_sys::JS_GetGlobalObject(context.context);

            // Setup console
            self.setup_console(context, global)?;

            // Setup carapace global object
            self.setup_carapace_api(context, global)?;

            // Setup utility functions
            self.setup_utils(context, global)?;

            quickjs_wasm_sys::JS_FreeValue(context.context, global);
        }
        Ok(())
    }

    /// Set up console object
    fn setup_console(&self, context: &JsContextWrapper, global: *mut JSValue) -> Result<(), TsPluginError> {
        if !self.config.enable_console {
            return Ok(());
        }

        unsafe {
            // Create console object
            let console = quickjs_wasm_sys::JS_NewObject(context.context);

            // log function
            let log_code = CString::new(include_str!("../js/console_shim.js"))
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let log_fn = quickjs_wasm_sys::JS_Eval(
                context.context,
                log_code.as_ptr(),
                log_code.as_bytes().len() as i32,
                CStr::from_ptr(b"<console.log>\0" as *const u8 as *const c_char).as_ptr(),
                quickjs_wasm_sys::JS_EVAL_TYPE_GLOBAL as i32,
            );

            if quickjs_wasm_sys::JS_IsException(log_fn) == 1 {
                let error = self.get_exception_message(context);
                return Err(TsPluginError::ExecutionError(format!("Console setup failed: {}", error)));
            }

            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                console,
                CStr::from_ptr(b"log\0" as *const u8 as *const c_char).as_ptr(),
                log_fn,
            );

            // info function
            let info_code = CString::new(
                r#"function info(){__carapace_console_log("[INFO] "+Array.prototype.slice.call(arguments).map(String).join(" "));}"#,
            )
            .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let info_fn = quickjs_wasm_sys::JS_Eval(
                context.context,
                info_code.as_ptr(),
                info_code.as_bytes().len() as i32,
                CStr::from_ptr(b"<console.info>\0" as *const u8 as *const c_char).as_ptr(),
                quickjs_wasm_sys::JS_EVAL_TYPE_GLOBAL as i32,
            );

            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                console,
                CStr::from_ptr(b"info\0" as *const u8 as *const c_char).as_ptr(),
                info_fn,
            );

            // error function
            let error_code = CString::new(
                r#"function error(){__carapace_console_log("[ERROR] "+Array.prototype.slice.call(arguments).map(String).join(" "));}"#,
            )
            .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let error_fn = quickjs_wasm_sys::JS_Eval(
                context.context,
                error_code.as_ptr(),
                error_code.as_bytes().len() as i32,
                CStr::from_ptr(b"<console.error>\0" as *const u8 as *const c_char).as_ptr(),
                quickjs_wasm_sys::JS_EVAL_TYPE_GLOBAL as i32,
            );

            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                console,
                CStr::from_ptr(b"error\0" as *const u8 as *const c_char).as_ptr(),
                error_fn,
            );

            // Add console to global
            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                global,
                CStr::from_ptr(b"console\0" as *const u8 as *const c_char).as_ptr(),
                console,
            );
        }

        Ok(())
    }

    /// Set up carapace API object
    fn setup_carapace_api(&self, context: &JsContextWrapper, global: *mut JSValue) -> Result<(), TsPluginError> {
        unsafe {
            // Create carapace object
            let carapace = quickjs_wasm_sys::JS_NewObject(context.context);

            // Set plugin ID
            let plugin_id = CString::new(self.id.clone())
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;
            let plugin_id_val = quickjs_wasm_sys::JS_NewString(context.context, plugin_id.as_ptr());
            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                carapace,
                CStr::from_ptr(b"pluginId\0" as *const u8 as *const c_char).as_ptr(),
                plugin_id_val,
            );

            // Set plugin name
            let plugin_name = CString::new(self.name.clone())
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;
            let plugin_name_val = quickjs_wasm_sys::JS_NewString(context.context, plugin_name.as_ptr());
            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                carapace,
                CStr::from_ptr(b"pluginName\0" as *const u8 as *const c_char).as_ptr(),
                plugin_name_val,
            );

            // Add carapace to global
            quickjs_wasm_sys::JS_SetPropertyStr(
                context.context,
                global,
                CStr::from_ptr(b"__carapace\0" as *const u8 as *const c_char).as_ptr(),
                carapace,
            );
        }

        Ok(())
    }

    /// Set up utility functions
    fn setup_utils(&self, context: &JsContextWrapper, global: *mut JSValue) -> Result<(), TsPluginError> {
        // This setup uses pre-built utilities to avoid eval-like patterns
        Ok(())
    }

    /// Evaluate plugin code
    fn evaluate_plugin_code(&self, context: &JsContextWrapper) -> Result<(), TsPluginError> {
        let code = &self.code;

        unsafe {
            let code_cstring = CString::new(code.as_str())
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let result = quickjs_wasm_sys::JS_Eval(
                context.context,
                code_cstring.as_ptr(),
                code_cstring.as_bytes().len() as i32,
                CStr::from_ptr(b"<plugin>\0" as *const u8 as *const c_char).as_ptr(),
                quickjs_wasm_sys::JS_EVAL_TYPE_GLOBAL as i32,
            );

            if quickjs_wasm_sys::JS_IsException(result) == 1 {
                let error = self.get_exception_message(context);
                quickjs_wasm_sys::JS_FreeValue(context.context, result);
                return Err(TsPluginError::ExecutionError(error));
            }

            quickjs_wasm_sys::JS_FreeValue(context.context, result);
        }

        Ok(())
    }

    /// Call plugin init function
    fn call_init(&self, context: &JsContextWrapper) -> Result<(), TsPluginError> {
        unsafe {
            let init_fn_name = CString::new("init")
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let init_val = quickjs_wasm_sys::JS_GetPropertyStr(
                context.context,
                quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                init_fn_name.as_ptr(),
            );

            if quickjs_wasm_sys::JS_IsFunction(context.context, init_val) == 1 {
                let result = quickjs_wasm_sys::JS_Call(
                    context.context,
                    init_val,
                    quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                    0,
                    std::ptr::null_mut(),
                );

                if quickjs_wasm_sys::JS_IsException(result) == 1 {
                    let error = self.get_exception_message(context);
                    quickjs_wasm_sys::JS_FreeValue(context.context, result);
                    quickjs_wasm_sys::JS_FreeValue(context.context, init_val);
                    return Err(TsPluginError::ExecutionError(format!("Init failed: {}", error)));
                }

                quickjs_wasm_sys::JS_FreeValue(context.context, result);
            }

            quickjs_wasm_sys::JS_FreeValue(context.context, init_val);
        }

        Ok(())
    }

    /// Discover available tools from the plugin
    fn discover_tools(&mut self, context: &JsContextWrapper) -> Result<(), TsPluginError> {
        unsafe {
            let get_info_name = CString::new("getInfo")
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let get_info_val = quickjs_wasm_sys::JS_GetPropertyStr(
                context.context,
                quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                get_info_name.as_ptr(),
            );

            if quickjs_wasm_sys::JS_IsFunction(context.context, get_info_val) == 1 {
                let result = quickjs_wasm_sys::JS_Call(
                    context.context,
                    get_info_val,
                    quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                    0,
                    std::ptr::null_mut(),
                );

                if quickjs_wasm_sys::JS_IsException(result) == 1 {
                    let error = self.get_exception_message(context);
                    quickjs_wasm_sys::JS_FreeValue(context.context, result);
                    quickjs_wasm_sys::JS_FreeValue(context.context, get_info_val);
                    // Non-fatal - plugin might not have getInfo
                    return Ok(());
                }

                // Parse the result to get tool list
                let result_str = self.value_to_string(context, result)?;
                let info: serde_json::Value = serde_json::from_str(&result_str)
                    .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

                if let Some(tools) = info.get("tools") {
                    if let Some(tools_array) = tools.as_array() {
                        for tool in tools_array {
                            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                                self.tools.push(name.to_string());
                            }
                        }
                    }
                }

                quickjs_wasm_sys::JS_FreeValue(context.context, result);
            }

            quickjs_wasm_sys::JS_FreeValue(context.context, get_info_val);
        }

        Ok(())
    }

    /// Execute a tool function
    pub fn call_tool(
        &self,
        tool_name: &str,
        args: &str,
    ) -> Result<String, TsPluginError> {
        if !self.initialized {
            return Err(TsPluginError::NotInitialized);
        }

        let context = self.context.as_ref()
            .ok_or_else(|| TsPluginError::NotInitialized)?;

        unsafe {
            let handle_tool_name = CString::new("handleTool")
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let handle_tool_val = quickjs_wasm_sys::JS_GetPropertyStr(
                context.context,
                quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                handle_tool_name.as_ptr(),
            );

            if quickjs_wasm_sys::JS_IsFunction(context.context, handle_tool_val) != 1 {
                quickjs_wasm_sys::JS_FreeValue(context.context, handle_tool_val);
                return Err(TsPluginError::ToolNotFound(tool_name.to_string()));
            }

            // Create arguments
            let tool_name_cstring = CString::new(tool_name)
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;
            let args_cstring = CString::new(args)
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let tool_name_val = quickjs_wasm_sys::JS_NewString(context.context, tool_name_cstring.as_ptr());
            let args_val = quickjs_wasm_sys::JS_NewString(context.context, args_cstring.as_ptr());

            let mut argv = [tool_name_val, args_val];

            let result = quickjs_wasm_sys::JS_Call(
                context.context,
                handle_tool_val,
                quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                2,
                argv.as_mut_ptr(),
            );

            quickjs_wasm_sys::JS_FreeValue(context.context, tool_name_val);
            quickjs_wasm_sys::JS_FreeValue(context.context, args_val);
            quickjs_wasm_sys::JS_FreeValue(context.context, handle_tool_val);

            if quickjs_wasm_sys::JS_IsException(result) == 1 {
                let error = self.get_exception_message(context);
                quickjs_wasm_sys::JS_FreeValue(context.context, result);
                return Err(TsPluginError::ExecutionError(error));
            }

            let result_str = self.value_to_string(context, result)?;
            quickjs_wasm_sys::JS_FreeValue(context.context, result);

            Ok(result_str)
        }
    }

    /// Execute raw JavaScript code
    pub fn execute(&self, code: &str) -> Result<String, TsPluginError> {
        if !self.initialized {
            return Err(TsPluginError::NotInitialized);
        }

        let context = self.context.as_ref()
            .ok_or_else(|| TsPluginError::NotInitialized)?;

        if !self.config.allow_eval {
            return Err(TsPluginError::ExecutionError("eval is disabled".to_string()));
        }

        unsafe {
            let code_cstring = CString::new(code)
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let result = quickjs_wasm_sys::JS_Eval(
                context.context,
                code_cstring.as_ptr(),
                code_cstring.as_bytes().len() as i32,
                CStr::from_ptr(b"<eval>\0" as *const u8 as *const c_char).as_ptr(),
                quickjs_wasm_sys::JS_EVAL_TYPE_GLOBAL as i32,
            );

            if quickjs_wasm_sys::JS_IsException(result) == 1 {
                let error = self.get_exception_message(context);
                quickjs_wasm_sys::JS_FreeValue(context.context, result);
                return Err(TsPluginError::ExecutionError(error));
            }

            let result_str = self.value_to_string(context, result)?;
            quickjs_wasm_sys::JS_FreeValue(context.context, result);

            Ok(result_str)
        }
    }

    /// Get exception message from QuickJS
    fn get_exception_message(&self, context: &JsContextWrapper) -> String {
        unsafe {
            let global = quickjs_wasm_sys::JS_GetGlobalObject(context.context);
            let exception = quickjs_wasm_sys::JS_GetException(context.context);

            let message = self.value_to_string(context, exception)
                .unwrap_or_else(|_| "Unknown error".to_string());

            quickjs_wasm_sys::JS_FreeValue(context.context, exception);
            quickjs_wasm_sys::JS_FreeValue(context.context, global);

            message
        }
    }

    /// Convert JS value to string
    fn value_to_string(&self, context: &JsContextWrapper, value: JSValue) -> Result<String, TsPluginError> {
        unsafe {
            if quickjs_wasm_sys::JS_IsString(value) == 1 {
                let ptr = quickjs_wasm_sys::JS_ToCString(context.context, value);
                if ptr.is_null() {
                    return Err(TsPluginError::ExecutionError("Failed to get string".to_string()));
                }
                let result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                quickjs_wasm_sys::JS_FreeCString(context.context, ptr);
                Ok(result)
            } else {
                // JSON stringify
                let json_str = quickjs_wasm_sys::JS_JSONStringify(
                    context.context,
                    value,
                    quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                );

                if quickjs_wasm_sys::JS_IsException(json_str) == 1 {
                    quickjs_wasm_sys::JS_FreeValue(context.context, json_str);
                    return Err(TsPluginError::ExecutionError("Failed to stringify".to_string()));
                }

                let result = self.value_to_string(context, json_str)?;
                quickjs_wasm_sys::JS_FreeValue(context.context, json_str);
                Ok(result)
            }
        }
    }

    /// Get plugin ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if plugin is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get list of available tools
    pub fn tools(&self) -> &[String] {
        &self.tools
    }

    /// Get console logs
    pub fn get_logs(&self) -> Vec<String> {
        if let Some(ref ctx) = self.context {
            let state = ctx.host_state.borrow();
            state.log_buffer.clone()
        } else {
            Vec::new()
        }
    }
}

impl Drop for TsPlugin {
    fn drop(&mut self) {
        if self.initialized {
            if let Some(ref context) = self.context {
                // Call shutdown function
                unsafe {
                    let shutdown_name = CString::new("shutdown")
                        .expect("Failed to create shutdown string");

                    let shutdown_val = quickjs_wasm_sys::JS_GetPropertyStr(
                        context.context,
                        quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                        shutdown_name.as_ptr(),
                    );

                    if quickjs_wasm_sys::JS_IsFunction(context.context, shutdown_val) == 1 {
                        let _ = quickjs_wasm_sys::JS_Call(
                            context.context,
                            shutdown_val,
                            quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                            0,
                            std::ptr::null_mut(),
                        );
                    }

                    quickjs_wasm_sys::JS_FreeValue(context.context, shutdown_val);
                }
            }
        }
    }
}

/// Registry of JavaScript/TypeScript plugins
#[derive(Default)]
pub struct TsPluginRegistry {
    plugins: HashMap<String, Rc<RefCell<TsPlugin>>>,
}

impl TsPluginRegistry {
    /// Register a new JavaScript plugin
    pub fn register(&mut self, plugin: TsPlugin) {
        let id = plugin.id().to_string();
        self.plugins.insert(id, Rc::new(RefCell::new(plugin)));
    }

    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Option<&Rc<RefCell<TsPlugin>>> {
        self.plugins.get(id)
    }

    /// Get a plugin mutably by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Rc<RefCell<TsPlugin>>> {
        self.plugins.get_mut(id)
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&Rc<RefCell<TsPlugin>>> {
        self.plugins.values().collect()
    }

    /// Call a tool on a plugin
    pub fn call_tool(
        &self,
        plugin_id: &str,
        tool_name: &str,
        args: &str,
    ) -> Result<String, TsPluginError> {
        let plugin = self.plugins.get(plugin_id)
            .ok_or_else(|| TsPluginError::PluginNotFound(plugin_id.to_string()))?;

        let result = plugin.borrow().call_tool(tool_name, args)?;
        Ok(result)
    }

    /// Check if a tool exists
    pub fn has_tool(&self, plugin_id: &str, tool_name: &str) -> bool {
        if let Some(plugin) = self.plugins.get(plugin_id) {
            plugin.borrow().tools().contains(&tool_name.to_string())
        } else {
            false
        }
    }
}

/// Loader for JavaScript plugins from files
pub async fn load_ts_plugin(
    path: std::path::PathBuf,
    plugin_id: String,
) -> Result<TsPlugin, TsPluginError> {
    let code = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| TsPluginError::IoError(e.to_string()))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut plugin = TsPlugin::new(plugin_id, name, code);
    plugin.init()?;

    Ok(plugin)
}

/// Load a plugin synchronously
pub fn load_ts_plugin_sync(
    path: std::path::PathBuf,
    plugin_id: String,
) -> Result<TsPlugin, TsPluginError> {
    use std::fs;

    let code = fs::read_to_string(&path)
        .map_err(|e| TsPluginError::IoError(e.to_string()))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut plugin = TsPlugin::new(plugin_id, name, code);
    plugin.init()?;

    Ok(plugin)
}
