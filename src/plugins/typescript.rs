// TypeScript/JavaScript Plugin Loader using QuickJS WASM
//
// This module provides JavaScript/TypeScript plugin support by embedding QuickJS.
// Plugins are written in JavaScript and run in an isolated context.

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
}

impl Default for TsPluginConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 128,
            timeout_seconds: 30,
            enable_console: true,
            allow_network: false,
            allow_eval: false,
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

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

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

    /// Initialize the JavaScript plugin
    pub fn init(&mut self) -> Result<(), TsPluginError> {
        let runtime = unsafe {
            let rt = quickjs_wasm_sys::JS_NewRuntime();
            if rt.is_null() {
                return Err(TsPluginError::QuickJsError(
                    "Failed to create runtime".to_string(),
                ));
            }
            JsRuntime { runtime: rt }
        };

        // Set memory limit
        let max_memory_bytes = (self.config.max_memory_mb * 1024 * 1024) as u64;
        unsafe {
            quickjs_wasm_sys::JS_SetMemoryLimit(runtime.runtime, max_memory_bytes);
        }

        let context = unsafe {
            let ctx = quickjs_wasm_sys::JS_NewContext(runtime.runtime);
            if ctx.is_null() {
                return Err(TsPluginError::QuickJsError(
                    "Failed to create context".to_string(),
                ));
            }
            JsContextWrapper {
                context: ctx,
                host_state: RefCell::new(HostState {
                    console_enabled: self.config.enable_console,
                    log_buffer: Vec::new(),
                }),
            }
        };

        // Load and evaluate plugin code
        self.evaluate_plugin_code(&context)?;

        // Call init if exists
        self.call_init(&context)?;

        // Discover tools
        self.discover_tools(&context)?;

        self.runtime = Some(runtime);
        self.context = Some(context);
        self.initialized = true;

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
                code_cstring.as_bytes().len() as u64,
                CStr::from_ptr(b"<plugin>\0" as *const u8 as *const c_char).as_ptr(),
                quickjs_wasm_sys::JS_EVAL_TYPE_GLOBAL as i32,
            );

            if self.is_error(context, result) {
                let error = self.get_error_message(context);
                self.free_value(context, result);
                return Err(TsPluginError::ExecutionError(error));
            }

            self.free_value(context, result);
        }

        Ok(())
    }

    /// Call plugin init function
    fn call_init(&self, context: &JsContextWrapper) -> Result<(), TsPluginError> {
        unsafe {
            let init_fn_name =
                CString::new("init").map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

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

                if self.is_error(context, result) {
                    self.free_value(context, result);
                    self.free_value(context, init_val);
                    return Err(TsPluginError::ExecutionError("Init failed".to_string()));
                }

                self.free_value(context, result);
            }

            self.free_value(context, init_val);
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

                if !self.is_error(context, result) {
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
                }

                self.free_value(context, result);
            }

            self.free_value(context, get_info_val);
        }

        Ok(())
    }

    /// Execute a tool function
    pub fn call_tool(&self, tool_name: &str, args: &str) -> Result<String, TsPluginError> {
        if !self.initialized {
            return Err(TsPluginError::NotInitialized);
        }

        let context = self
            .context
            .as_ref()
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
                self.free_value(context, handle_tool_val);
                return Err(TsPluginError::ToolNotFound(tool_name.to_string()));
            }

            let tool_name_cstring = CString::new(tool_name)
                .map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;
            let args_cstring =
                CString::new(args).map_err(|e| TsPluginError::ExecutionError(e.to_string()))?;

            let tool_name_val =
                quickjs_wasm_sys::JS_NewString(context.context, tool_name_cstring.as_ptr());
            let args_val = quickjs_wasm_sys::JS_NewString(context.context, args_cstring.as_ptr());

            let mut argv = [tool_name_val, args_val];

            let result = quickjs_wasm_sys::JS_Call(
                context.context,
                handle_tool_val,
                quickjs_wasm_sys::JS_GetGlobalObject(context.context),
                2,
                argv.as_mut_ptr(),
            );

            self.free_value(context, tool_name_val);
            self.free_value(context, args_val);
            self.free_value(context, handle_tool_val);

            if self.is_error(context, result) {
                let error = self.get_error_message(context);
                self.free_value(context, result);
                return Err(TsPluginError::ExecutionError(error));
            }

            let result_str = self.value_to_string(context, result)?;
            self.free_value(context, result);

            Ok(result_str)
        }
    }

    /// Check if a value is an error
    fn is_error(&self, context: &JsContextWrapper, value: JSValue) -> bool {
        unsafe { quickjs_wasm_sys::JS_IsError(context.context, value) == 1 }
    }

    /// Free a JS value
    fn free_value(&self, context: &JsContextWrapper, value: JSValue) {
        unsafe {
            quickjs_wasm_sys::__JS_FreeValue(context.context, value);
        }
    }

    /// Get error message from QuickJS
    fn get_error_message(&self, context: &JsContextWrapper) -> String {
        unsafe {
            let exception = quickjs_wasm_sys::JS_GetException(context.context);
            let message = self
                .value_to_string(context, exception)
                .unwrap_or_else(|_| "Unknown error".to_string());
            self.free_value(context, exception);
            message
        }
    }

    /// Convert JS value to string
    fn value_to_string(
        &self,
        context: &JsContextWrapper,
        value: JSValue,
    ) -> Result<String, TsPluginError> {
        unsafe {
            let str_val = quickjs_wasm_sys::JS_ToString(context.context, value);
            // JS_ToString returns 0 on error
            if str_val == 0 {
                return Err(TsPluginError::ExecutionError(
                    "Failed to convert to string".to_string(),
                ));
            }
            let ptr = quickjs_wasm_sys::JS_AtomToCString(
                context.context,
                quickjs_wasm_sys::JS_ValueToAtom(context.context, str_val),
            );
            if ptr.is_null() {
                self.free_value(context, str_val);
                return Err(TsPluginError::ExecutionError(
                    "Failed to get C string".to_string(),
                ));
            }
            let result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            quickjs_wasm_sys::JS_FreeCString(context.context, ptr);
            self.free_value(context, str_val);
            Ok(result)
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
}

impl Drop for TsPlugin {
    fn drop(&mut self) {
        if self.initialized {
            if let Some(ref context) = self.context {
                unsafe {
                    let shutdown_name =
                        CString::new("shutdown").expect("Failed to create shutdown string");

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

                    quickjs_wasm_sys::__JS_FreeValue(context.context, shutdown_val);
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

    /// Call a tool on a plugin
    pub fn call_tool(
        &self,
        plugin_id: &str,
        tool_name: &str,
        args: &str,
    ) -> Result<String, TsPluginError> {
        let plugin = self
            .plugins
            .get(plugin_id)
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

    let code = fs::read_to_string(&path).map_err(|e| TsPluginError::IoError(e.to_string()))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut plugin = TsPlugin::new(plugin_id, name, code);
    plugin.init()?;

    Ok(plugin)
}
