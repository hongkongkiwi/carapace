//! CLI subcommand definitions and handlers.
//!
//! Uses clap derive to define the subcommand hierarchy:
//! - `start` (default) -- start the gateway server
//! - `config show|get|set|path` -- read/write configuration
//! - `status` -- query a running instance for health info
//! - `logs` -- tail log entries from a running instance
//! - `version` -- print build/version info

use clap::{Parser, Subcommand};

/// Carapace gateway server for AI assistants.
#[derive(Parser, Debug)]
#[command(
    name = "carapace",
    version = env!("CARGO_PKG_VERSION"),
    about = "Carapace — a secure gateway server for AI assistants"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start the gateway server (default when no subcommand is given).
    Start,

    /// Read or write configuration values.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Query a running instance for health/status information.
    Status {
        /// Port of the running instance (default: from config or 18789).
        #[arg(short, long)]
        port: Option<u16>,

        /// Host of the running instance.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Tail log entries from a running instance.
    Logs {
        /// Number of recent log lines to show (default: 50).
        #[arg(short = 'n', long, default_value_t = 50)]
        lines: usize,

        /// Port of the running instance (default: from config or 18789).
        #[arg(short, long)]
        port: Option<u16>,

        /// Host of the running instance.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Print version, build date, and git commit information.
    Version,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Print the fully loaded configuration (secrets redacted) as JSON.
    Show,

    /// Print a specific configuration value by dot-notation path.
    Get {
        /// Dot-notation key (e.g. "server.port", "gateway.bind").
        key: String,
    },

    /// Set a configuration value and write to disk.
    Set {
        /// Dot-notation key (e.g. "gateway.port").
        key: String,

        /// Value to set (interpreted as JSON; bare strings allowed).
        value: String,
    },

    /// Print the resolved configuration file path.
    Path,
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

use crate::config;
use crate::logging::buffer::LOG_BUFFER;
use crate::server::bind::DEFAULT_PORT;
use serde_json::Value;

/// Secrets that should be redacted when printing config.
const SECRET_KEYS: &[&str] = &[
    "apiKey",
    "apikey",
    "api_key",
    "token",
    "secret",
    "password",
    "credentials",
];

/// Run the `config show` subcommand.
pub fn handle_config_show() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_config()?;
    let redacted = redact_secrets(cfg);
    let pretty = serde_json::to_string_pretty(&redacted)?;
    println!("{}", pretty);
    Ok(())
}

/// Run the `config get <key>` subcommand.
pub fn handle_config_get(key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_config()?;
    match get_value_at_path(&cfg, key) {
        Some(value) => {
            let pretty = serde_json::to_string_pretty(&value)?;
            println!("{}", pretty);
        }
        None => {
            eprintln!("Key not found: {}", key);
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Run the `config set <key> <value>` subcommand.
pub fn handle_config_set(key: &str, raw_value: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse value as JSON first; fall back to treating it as a plain string.
    let value: Value =
        serde_json::from_str(raw_value).unwrap_or_else(|_| Value::String(raw_value.to_string()));

    // Load current config from disk (bypassing cache).
    let config_path = config::get_config_path();
    let mut cfg = config::load_config_uncached(&config_path)?;

    // Walk the dot-path and set the value, creating intermediate objects as needed.
    set_value_at_path(&mut cfg, key, value.clone());

    // Write atomically (write to temp, rename).
    use crate::server::ws::persist_config_file;
    persist_config_file(&config_path, &cfg).map_err(std::io::Error::other)?;

    println!("Set {} = {}", key, serde_json::to_string(&value)?);
    Ok(())
}

/// Run the `config path` subcommand.
pub fn handle_config_path() {
    println!("{}", config::get_config_path().display());
}

/// Run the `status` subcommand -- connect to a running instance's health endpoint.
pub async fn handle_status(
    host: &str,
    port: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let port = resolve_port(port);
    let url = format!("http://{}:{}/health", host, port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Could not connect to carapace at {}:{}", host, port);
            eprintln!("  Error: {}", e);
            eprintln!();
            eprintln!("Is the server running? Start it with: carapace start");
            std::process::exit(1);
        }
    };

    if !response.status().is_success() {
        eprintln!(
            "Health endpoint returned HTTP {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
        std::process::exit(1);
    }

    let body: Value = response.json().await?;

    // Pretty-print the status summary.
    println!("Carapace gateway status");
    println!("=======================");
    if let Some(version) = body.get("version").and_then(|v| v.as_str()) {
        println!("  Version:  {}", version);
    }
    if let Some(uptime) = body.get("uptimeSeconds").and_then(|v| v.as_i64()) {
        println!("  Uptime:   {}", format_duration(uptime));
    }
    println!("  Address:  {}:{}", host, port);
    if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
        println!("  Status:   {}", status);
    }

    // If the control endpoint is available, try to get richer info.
    let control_url = format!("http://{}:{}/control/status", host, port);
    if let Ok(resp) = client.get(&control_url).send().await {
        if resp.status().is_success() {
            if let Ok(ctrl) = resp.json::<Value>().await {
                if let Some(ch) = ctrl.get("connectedChannels").and_then(|v| v.as_u64()) {
                    let total = ctrl
                        .get("totalChannels")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    println!("  Channels: {}/{} connected", ch, total);
                }
                if let Some(rt) = ctrl.get("runtime").and_then(|v| v.as_object()) {
                    if let (Some(platform), Some(arch)) = (
                        rt.get("platform").and_then(|v| v.as_str()),
                        rt.get("arch").and_then(|v| v.as_str()),
                    ) {
                        println!("  Platform: {} ({})", platform, arch);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run the `logs` subcommand -- fetch recent logs from a running instance.
pub async fn handle_logs(
    host: &str,
    port: Option<u16>,
    lines: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let port = resolve_port(port);

    // First, try the local in-process log buffer (only works if we *are* the
    // running process, which normally we are not). This is a graceful no-op for
    // the common CLI case.
    let buffer_entries = LOG_BUFFER.len();
    if buffer_entries > 0 {
        let filter = crate::logging::buffer::LogFilter::new().with_limit(lines);
        let result = LOG_BUFFER.query(&filter);
        for entry in &result.entries {
            println!(
                "{} [{}] {}: {}",
                format_timestamp(entry.timestamp),
                entry.level,
                entry.target,
                entry.message
            );
        }
        return Ok(());
    }

    // Otherwise, try to read from the log file on disk.
    let log_path = crate::server::ws::resolve_state_dir()
        .join("logs")
        .join("moltbot.log");

    if log_path.exists() {
        let content = std::fs::read_to_string(&log_path)?;
        let all_lines: Vec<&str> = content.lines().collect();
        let start = all_lines.len().saturating_sub(lines);
        for line in &all_lines[start..] {
            println!("{}", line);
        }
        return Ok(());
    }

    // Last resort: hit the health endpoint to confirm the server is running,
    // then inform the user that log streaming is not yet available via this path.
    let url = format!("http://{}:{}/health", host, port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            eprintln!(
                "Server is running at {}:{}, but no log file found at {}",
                host,
                port,
                log_path.display()
            );
            eprintln!("Hint: enable file logging or use the WebSocket logs.tail method.");
        }
        _ => {
            eprintln!("Could not connect to carapace at {}:{}", host, port);
            eprintln!("Is the server running? Start it with: carapace start");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run the `version` subcommand.
pub fn handle_version() {
    println!("carapace {}", env!("CARGO_PKG_VERSION"));
    println!("  Build date: {}", env!("CARAPACE_BUILD_DATE"));
    println!("  Git commit: {}", env!("CARAPACE_GIT_HASH"));
    println!(
        "  Platform:   {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Navigate a JSON value by dot-notation path and return the leaf value.
fn get_value_at_path(root: &Value, path: &str) -> Option<Value> {
    let mut current = root;
    for part in path.split('.') {
        current = current.as_object()?.get(part)?;
    }
    Some(current.clone())
}

/// Set a value at a dot-notation path, creating intermediate objects as needed.
fn set_value_at_path(root: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = current {
                map.insert(part.to_string(), value);
            }
            return;
        }
        if !current.get(*part).is_some_and(|v| v.is_object()) {
            if let Value::Object(map) = current {
                map.insert(part.to_string(), Value::Object(serde_json::Map::new()));
            }
        }
        current = current.get_mut(*part).expect("just inserted");
    }
}

/// Redact known secret keys in a JSON value (recursive).
fn redact_secrets(mut value: Value) -> Value {
    match &mut value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let lower = key.to_lowercase();
                if SECRET_KEYS.iter().any(|s| lower.contains(s)) {
                    map.insert(key, Value::String("[REDACTED]".to_string()));
                } else if let Some(child) = map.remove(&key) {
                    map.insert(key, redact_secrets(child));
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                *item = redact_secrets(item.clone());
            }
        }
        _ => {}
    }
    value
}

/// Resolve the port to use for connecting to a running instance.
/// Tries (in order): explicit flag, config file value, DEFAULT_PORT.
fn resolve_port(explicit: Option<u16>) -> u16 {
    if let Some(p) = explicit {
        return p;
    }
    // Try reading from config.
    if let Ok(cfg) = config::load_config() {
        if let Some(port) = cfg
            .get("gateway")
            .and_then(|g| g.get("port"))
            .and_then(|v| v.as_u64())
        {
            return port as u16;
        }
    }
    DEFAULT_PORT
}

/// Format seconds into a human-readable duration string.
fn format_duration(seconds: i64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, mins, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Format a Unix-ms timestamp for display.
fn format_timestamp(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        .unwrap_or_else(|| ms.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_no_args_defaults_to_none() {
        let cli = Cli::try_parse_from(["carapace"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_start_subcommand() {
        let cli = Cli::try_parse_from(["carapace", "start"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Start)));
    }

    #[test]
    fn test_cli_version_subcommand() {
        let cli = Cli::try_parse_from(["carapace", "version"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Version)));
    }

    #[test]
    fn test_cli_config_show() {
        let cli = Cli::try_parse_from(["carapace", "config", "show"]).unwrap();
        match cli.command {
            Some(Command::Config(ConfigCommand::Show)) => {}
            other => panic!("Expected Config(Show), got {:?}", other),
        }
    }

    #[test]
    fn test_cli_config_get() {
        let cli = Cli::try_parse_from(["carapace", "config", "get", "gateway.port"]).unwrap();
        match cli.command {
            Some(Command::Config(ConfigCommand::Get { ref key })) => {
                assert_eq!(key, "gateway.port");
            }
            other => panic!("Expected Config(Get), got {:?}", other),
        }
    }

    #[test]
    fn test_cli_config_set() {
        let cli =
            Cli::try_parse_from(["carapace", "config", "set", "gateway.port", "9000"]).unwrap();
        match cli.command {
            Some(Command::Config(ConfigCommand::Set { ref key, ref value })) => {
                assert_eq!(key, "gateway.port");
                assert_eq!(value, "9000");
            }
            other => panic!("Expected Config(Set), got {:?}", other),
        }
    }

    #[test]
    fn test_cli_config_path() {
        let cli = Cli::try_parse_from(["carapace", "config", "path"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Config(ConfigCommand::Path))
        ));
    }

    #[test]
    fn test_cli_status_defaults() {
        let cli = Cli::try_parse_from(["carapace", "status"]).unwrap();
        match cli.command {
            Some(Command::Status { port, ref host }) => {
                assert_eq!(port, None);
                assert_eq!(host, "127.0.0.1");
            }
            other => panic!("Expected Status, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_status_with_port() {
        let cli = Cli::try_parse_from(["carapace", "status", "--port", "9000"]).unwrap();
        match cli.command {
            Some(Command::Status { port, .. }) => {
                assert_eq!(port, Some(9000));
            }
            other => panic!("Expected Status, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_logs_defaults() {
        let cli = Cli::try_parse_from(["carapace", "logs"]).unwrap();
        match cli.command {
            Some(Command::Logs {
                lines,
                port,
                ref host,
            }) => {
                assert_eq!(lines, 50);
                assert_eq!(port, None);
                assert_eq!(host, "127.0.0.1");
            }
            other => panic!("Expected Logs, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_logs_with_lines() {
        let cli = Cli::try_parse_from(["carapace", "logs", "--lines", "100"]).unwrap();
        match cli.command {
            Some(Command::Logs { lines, .. }) => {
                assert_eq!(lines, 100);
            }
            other => panic!("Expected Logs, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_logs_with_short_flag() {
        let cli = Cli::try_parse_from(["carapace", "logs", "-n", "25"]).unwrap();
        match cli.command {
            Some(Command::Logs { lines, .. }) => {
                assert_eq!(lines, 25);
            }
            other => panic!("Expected Logs, got {:?}", other),
        }
    }

    #[test]
    fn test_get_value_at_path_simple() {
        let val = serde_json::json!({"gateway": {"port": 9000}});
        let result = get_value_at_path(&val, "gateway.port");
        assert_eq!(result, Some(serde_json::json!(9000)));
    }

    #[test]
    fn test_get_value_at_path_top_level() {
        let val = serde_json::json!({"key": "value"});
        let result = get_value_at_path(&val, "key");
        assert_eq!(result, Some(serde_json::json!("value")));
    }

    #[test]
    fn test_get_value_at_path_missing() {
        let val = serde_json::json!({"a": 1});
        let result = get_value_at_path(&val, "b.c");
        assert_eq!(result, None);
    }

    #[test]
    fn test_set_value_at_path_creates_intermediate() {
        let mut val = serde_json::json!({});
        set_value_at_path(&mut val, "a.b.c", serde_json::json!(42));
        assert_eq!(val["a"]["b"]["c"], 42);
    }

    #[test]
    fn test_set_value_at_path_overwrites() {
        let mut val = serde_json::json!({"gateway": {"port": 8080}});
        set_value_at_path(&mut val, "gateway.port", serde_json::json!(9000));
        assert_eq!(val["gateway"]["port"], 9000);
    }

    #[test]
    fn test_redact_secrets() {
        let val = serde_json::json!({
            "gateway": {
                "port": 9000,
                "auth": {
                    "token": "my-secret-token"
                }
            },
            "anthropic": {
                "apiKey": "sk-ant-abc123"
            },
            "safe": "visible"
        });
        let redacted = redact_secrets(val);
        assert_eq!(redacted["gateway"]["auth"]["token"], "[REDACTED]");
        assert_eq!(redacted["anthropic"]["apiKey"], "[REDACTED]");
        assert_eq!(redacted["gateway"]["port"], 9000);
        assert_eq!(redacted["safe"], "visible");
    }

    #[test]
    fn test_redact_secrets_array() {
        let val = serde_json::json!([{"apiKey": "secret"}, {"safe": "ok"}]);
        let redacted = redact_secrets(val);
        assert_eq!(redacted[0]["apiKey"], "[REDACTED]");
        assert_eq!(redacted[1]["safe"], "ok");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(5), "5s");
        assert_eq!(format_duration(65), "1m 5s");
        assert_eq!(format_duration(3665), "1h 1m 5s");
        assert_eq!(format_duration(90061), "1d 1h 1m 1s");
    }

    #[test]
    fn test_resolve_port_explicit() {
        assert_eq!(resolve_port(Some(1234)), 1234);
    }

    #[test]
    fn test_resolve_port_default() {
        // When config is unavailable, should fall back to DEFAULT_PORT.
        assert_eq!(resolve_port(None), DEFAULT_PORT);
    }
}
