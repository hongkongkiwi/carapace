# Carapace Plugin System

Carapace supports plugins written in Rust, Go, and TypeScript/JavaScript, all compiled to WebAssembly.

## Supported Languages

| Language | Support Level | Runtime | Notes |
|----------|---------------|---------|-------|
| **Rust** | Full | wasmtime | Native WASM, best performance |
| **Go** | Full | wasmtime | Via TinyGo, WASI support |
| **TypeScript/JavaScript** | Full | QuickJS WASM | Embedded JS engine |

---

## Rust Plugins

### 1. Create a new library crate

```bash
cargo new --lib my-carapace-plugin
cd my-carapace-plugin
```

### 2. Update Cargo.toml

```toml
[package]
name = "my-carapace-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
```

### 3. Implement the plugin

See `examples/plugins/example_plugin.rs` for a complete example.

Required exports:
- `PLUGIN_NAME` - Plugin identifier
- `PLUGIN_VERSION` - Semantic version
- `PLUGIN_DESCRIPTION` - Human-readable description
- `init()` - Called when plugin loads
- `shutdown()` - Called when plugin unloads
- Tool functions

### 4. Compile to WASM

```bash
# Install wasm32-wasi target
rustup target add wasm32-wasi

# Build
cargo build --target wasm32-wasi --release

# Output: target/wasm32-wasi/release/my_carapace_plugin.wasm
```

---

## Go Plugins

Go plugins require [TinyGo](https://tinygo.org/) for WASI compilation.

### 1. Install TinyGo

```bash
# macOS
brew install tinygo

# Or download from https://tinygo.org/getting-started/install/
```

### 2. Create a Go module

```bash
mkdir my-go-plugin && cd my-go-plugin
go mod init my-go-plugin
```

### 3. Implement the plugin

See `examples/plugins/go_plugin.go` for a complete example.

### 4. Compile to WASM

```bash
tinygo build -target wasi -o my-go-plugin.wasm main.go
```

---

## TypeScript/JavaScript Plugins

JavaScript plugins run via QuickJS WASM, a lightweight JavaScript engine.

### 1. Create a JavaScript file

See `examples/plugins/hello_plugin.js` for a complete example.

```javascript
// Required metadata
const PLUGIN_NAME = "hello-plugin";
const PLUGIN_VERSION = "0.1.0";
const PLUGIN_DESCRIPTION = "A JavaScript plugin example";

// Tool function
function greet(name, prefix = "Hello") {
    return { greeting: `${prefix}, ${name}!` };
}

// Tool function
function calculate(a, b, operation = "add") {
    const ops = {
        add: a + b,
        subtract: a - b,
        multiply: a * b,
        divide: b !== 0 ? a / b : null,
    };
    return { operation, result: ops[operation] };
}

// Get plugin info
function getInfo() {
    return {
        name: PLUGIN_NAME,
        version: PLUGIN_VERSION,
        tools: ["greet", "calculate", "getInfo"],
    };
}

// Lifecycle hooks
function init() {
    console.log("Plugin initialized");
    return { status: "initialized" };
}

function shutdown() {
    return { status: "shutdown" };
}

// Main entry point - handle tool calls from carapace
function handleTool(toolName, argsJson) {
    const args = argsJson ? JSON.parse(argsJson) : {};

    const tools = {
        greet: () => greet(args.name, args.prefix),
        calculate: () => calculate(args.a, args.b, args.operation),
        getInfo: () => getInfo(),
    };

    if (!tools[toolName]) {
        return JSON.stringify({ error: `Unknown tool: ${toolName}` });
    }
    return JSON.stringify(tools[toolName]());
}
```

### 2. Load the plugin

Place JavaScript plugins in:
```
~/.carapace/plugins/typescript/
└── hello_plugin.js
```

---

## Security Model

All plugins run in a sandboxed environment:

| Limit | Value |
|-------|-------|
| Memory | 64-128MB per plugin |
| Timeout | 30 seconds per call |
| HTTP requests | 100/minute |
| Logging | 1000 messages/minute |

**Security restrictions:**
- Network: SSRF protection blocks private IPs
- Credentials: Plugins can only access their own prefixed credentials
- JavaScript: No access to `eval()`, `Function()`, `require()`

---

## Plugin Location

```
~/.carapace/
├── plugins/
│   ├── rust/
│   │   ├── my-plugin.wasm
│   │   └── another.wasm
│   ├── go/
│   │   └── my-go-plugin.wasm
│   └── typescript/
│       ├── hello_plugin.js
│       └── another.js
└── config.json5
```

---

## Best Practices

1. **One plugin, one responsibility**
2. **Validate all inputs**
3. **Handle errors gracefully**
4. **Log appropriately** (console.log for JS, log_message for Rust)
5. **Test before deploying**

---

## Built-in Capabilities

The `plugins/caps/` directory contains ready-to-use integrations:

- GitHub, Stripe, Redis, PostgreSQL, SendGrid, Twilio, Notion, Linear, Jira, Teams

These serve as reference implementations for your own plugins.
