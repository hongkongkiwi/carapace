// Example JavaScript/TypeScript Plugin for Carapace
//
// This is a sample JavaScript plugin that can be loaded by the carapace gateway.
// Uses QuickJS WASM as the JavaScript runtime.
//
// To use:
// 1. Save this as hello_plugin.js
// 2. Place in ~/.carapace/plugins/typescript/
// 3. Load via the TypeScript plugin loader

// ============================================================================
// Plugin Metadata (Required)
// ============================================================================

const PLUGIN_NAME = "hello-plugin";
const PLUGIN_VERSION = "0.1.0";
const PLUGIN_DESCRIPTION = "A JavaScript example plugin for carapace";

// ============================================================================
// Tool Functions
// ============================================================================

/**
 * Generate a personalized greeting.
 * @param {string} name - Name to greet
 * @param {string} prefix - Greeting prefix (default: "Hello")
 * @returns {object} Greeting result
 */
function greet(name, prefix = "Hello") {
    const greeting = `${prefix}, ${name}!`;
    return {
        greeting: greeting,
        length: greeting.length,
        uppercase: greeting.toUpperCase(),
    };
}

/**
 * Perform a simple calculation.
 * @param {number} a - First number
 * @param {number} b - Second number
 * @param {string} operation - One of: add, subtract, multiply, divide
 * @returns {object} Calculation result
 */
function calculate(a, b, operation = "add") {
    const ops = {
        add: a + b,
        subtract: a - b,
        multiply: a * b,
        divide: b !== 0 ? a / b : null,
    };

    const result = ops[operation];

    if (result === undefined) {
        return { error: `Unknown operation: ${operation}` };
    }

    return {
        operation: operation,
        a: a,
        b: b,
        result: result,
    };
}

/**
 * Echo a message back (optionally repeated).
 * @param {string} message - Message to echo
 * @param {number} repeat - Number of times to repeat (default: 1)
 * @returns {object} Echo result
 */
function echo(message, repeat = 1) {
    const repeated = (message + " ").repeat(repeat).trim();
    return {
        original: message,
        repeated: repeated,
        repeatCount: repeat,
    };
}

/**
 * Get information about this plugin.
 * @returns {object} Plugin metadata
 */
function getInfo() {
    return {
        name: PLUGIN_NAME,
        version: PLUGIN_VERSION,
        description: PLUGIN_DESCRIPTION,
        tools: [
            {
                name: "greet",
                description: "Generate a personalized greeting",
                params: {
                    name: { type: "string", required: true },
                    prefix: { type: "string", required: false, default: "Hello" },
                },
            },
            {
                name: "calculate",
                description: "Perform a simple calculation",
                params: {
                    a: { type: "number", required: true },
                    b: { type: "number", required: true },
                    operation: { type: "string", required: false, default: "add" },
                },
            },
            {
                name: "echo",
                description: "Echo a message back",
                params: {
                    message: { type: "string", required: true },
                    repeat: { type: "number", required: false, default: 1 },
                },
            },
            {
                name: "getInfo",
                description: "Get plugin information",
                params: {},
            },
        ],
    };
}

// ============================================================================
// Advanced Tools
// ============================================================================

/**
 * Process text with various transformations.
 * @param {string} text - Input text
 * @param {object} options - Transformation options
 * @returns {object} Transformed text
 */
function transformText(text, options = {}) {
    const {
        uppercase = false,
        lowercase = false,
        reverse = false,
        trim = true,
        capitalize = false,
    } = options;

    let result = text;

    if (trim) result = result.trim();
    if (uppercase) result = result.toUpperCase();
    else if (lowercase) result = result.toLowerCase();
    else if (capitalize) result = result.charAt(0).toUpperCase() + result.slice(1);
    if (reverse) result = result.split("").reverse().join("");

    return {
        original: text,
        transformed: result,
        options: options,
    };
}

/**
 * Format a date in various ways.
 * @param {string|Date} date - Date to format
 * @param {string} format - Format: iso, us, eu, unix, relative
 * @returns {object} Formatted date
 */
function formatDate(date, format = "iso") {
    const d = typeof date === "string" ? new Date(date) : date;

    if (isNaN(d.getTime())) {
        return { error: "Invalid date" };
    }

    const formats = {
        iso: d.toISOString().split("T")[0],
        us: `${d.getMonth() + 1}/${d.getDate()}/${d.getFullYear()}`,
        eu: `${d.getDate()}/${d.getMonth() + 1}/${d.getFullYear()}`,
        unix: Math.floor(d.getTime() / 1000),
        full: d.toLocaleString(),
    };

    return {
        input: date,
        format: format,
        result: formats[format] || formats.iso,
        timestamp: d.getTime(),
    };
}

// ============================================================================
// Lifecycle Hooks (Optional)
// ============================================================================

/**
 * Called when plugin is loaded.
 * Use for initialization, loading resources, etc.
 */
function init() {
    console.log(`${PLUGIN_NAME} v${PLUGIN_VERSION} initialized`);
    return {
        status: "initialized",
        message: `${PLUGIN_NAME} loaded successfully`,
    };
}

/**
 * Called when plugin is unloaded.
 * Use for cleanup, saving state, etc.
 */
function shutdown() {
    console.log(`${PLUGIN_NAME} shutting down`);
    return {
        status: "shutdown",
        message: `${PLUGIN_NAME} cleanup complete`,
    };
}

// ============================================================================
// Main Entry Point
// ============================================================================

/**
 * Main entry point for tool calls from carapace.
 * @param {string} toolName - Name of the tool to call
 * @param {string} argsJson - JSON-encoded arguments
 * @returns {string} JSON-encoded result
 */
function handleTool(toolName, argsJson) {
    try {
        const args = argsJson ? JSON.parse(argsJson) : {};

        // All available tools
        const tools = {
            greet,
            calculate,
            echo,
            getInfo,
            transformText,
            formatDate,
        };

        // Check if tool exists
        if (!tools[toolName]) {
            return JSON.stringify({
                error: `Unknown tool: ${toolName}`,
                availableTools: Object.keys(tools),
            });
        }

        // Call the tool
        const result = tools[toolName](...Object.values(args));
        return JSON.stringify(result);

    } catch (error) {
        return JSON.stringify({
            error: error.message,
            stack: error.stack,
        });
    }
}

// ============================================================================
// Testing (when run directly)
// ============================================================================

if (typeof window !== "undefined" || typeof global !== "undefined") {
    // Simulate running as a test
    console.log("Testing hello-plugin...");

    // Test greet
    const greetResult = handleTool("greet", JSON.stringify({ name: "World" }));
    console.log("greet:", greetResult);

    // Test calculate
    const calcResult = handleTool(
        "calculate",
        JSON.stringify({ a: 10, b: 5, operation: "multiply" })
    );
    console.log("calculate:", calcResult);

    // Test getInfo
    const infoResult = handleTool("getInfo", "{}");
    console.log("getInfo:", infoResult);

    console.log("All tests passed!");
}
