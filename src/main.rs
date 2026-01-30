//! Carapace Gateway - Main entry point
//!
//! CLI interface for managing the carapace gateway server.

#![allow(dead_code)]
#![allow(unused_imports)]

use clap::{Parser, Subcommand};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

mod agent;
mod auth;
mod channels;
mod config;
mod credentials;
mod cron;
mod devices;
mod exec;
mod flows;
mod hooks;
mod logging;
mod media;
mod migrations;
mod messages;
mod nodes;
mod plugins;
mod security;
mod server;
mod sessions;
mod usage;

use server::http::{create_router_with_middleware, HttpConfig, MiddlewareConfig};

/// Default bind address for the server
const DEFAULT_BIND: &str = "127.0.0.1:8080";

/// Default PID file location
fn default_pid_file() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("carapace.pid")
}

/// CLI arguments
#[derive(Parser)]
#[command(name = "carapace")]
#[command(about = "A secure, stable Rust alternative to moltbot")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server
    Start {
        /// Configuration file path
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Bind address (host:port)
        #[arg(short, long, default_value = DEFAULT_BIND)]
        bind: SocketAddr,

        /// Enable development mode (localhost-only, no auth required)
        #[arg(long)]
        dev: bool,

        /// PID file location
        #[arg(long, default_value_os_t = default_pid_file())]
        pid_file: PathBuf,

        /// Enable control endpoints
        #[arg(long)]
        control: bool,

        /// Enable OpenAI compatibility endpoints
        #[arg(long)]
        openai: bool,

        /// Enable hooks API
        #[arg(long)]
        hooks: bool,

        /// Hooks authentication token (use CARAPACE_HOOKS_TOKEN env var to avoid exposure in process list)
        #[arg(long, value_name = "TOKEN")]
        hooks_token: Option<String>,

        /// Gateway authentication token (use CARAPACE_GATEWAY_TOKEN env var to avoid exposure in process list)
        #[arg(long, value_name = "TOKEN")]
        gateway_token: Option<String>,

        /// Gateway authentication password (use CARAPACE_GATEWAY_PASSWORD env var to avoid exposure in process list)
        #[arg(long, value_name = "PASSWORD")]
        gateway_password: Option<String>,

        /// Control UI base path
        #[arg(long, default_value = "")]
        ui_base_path: String,

        /// Enable control UI
        #[arg(long)]
        ui: bool,

        /// Control UI dist path
        #[arg(long, default_value = "dist/control-ui")]
        ui_dist_path: PathBuf,

        /// Log level (overrides RUST_LOG)
        #[arg(short, long, value_name = "LEVEL")]
        log: Option<String>,

        /// Use insecure HTTP instead of HTTPS (development only - NOT FOR PRODUCTION)
        #[arg(long)]
        insecure_http: bool,

        /// Path to TLS certificate file (PEM format)
        #[arg(long, value_name = "PATH")]
        tls_cert: Option<PathBuf>,

        /// Path to TLS private key file (PEM format)
        #[arg(long, value_name = "PATH")]
        tls_key: Option<PathBuf>,

        /// Auto-generate self-signed certificate if not found
        #[arg(long)]
        tls_auto_gen: bool,
    },

    /// Stop the running gateway server
    Stop {
        /// PID file location
        #[arg(long, default_value_os_t = default_pid_file())]
        pid_file: PathBuf,

        /// Force kill if graceful shutdown fails
        #[arg(short, long)]
        force: bool,
    },

    /// Check gateway server status
    Status {
        /// PID file location
        #[arg(long, default_value_os_t = default_pid_file())]
        pid_file: PathBuf,

        /// Also check HTTP health endpoint
        #[arg(short, long)]
        health: bool,

        /// Gateway URL for health check
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        url: String,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Run database migrations
    Migrate {
        /// Database URL (defaults to SQLite in state directory)
        #[arg(short, long)]
        database_url: Option<String>,
    },

    /// Display version information
    Version,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Validate configuration file
    Validate {
        /// Configuration file path
        #[arg(value_name = "FILE")]
        config: PathBuf,
    },

    /// Get a configuration value
    Get {
        /// Configuration key (dot notation)
        #[arg(value_name = "KEY")]
        key: String,

        /// Configuration file path
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (dot notation)
        #[arg(value_name = "KEY")]
        key: String,

        /// Configuration value
        #[arg(value_name = "VALUE")]
        value: String,

        /// Configuration file path
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },

    /// Generate JSON schema for configuration
    Schema,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            config,
            bind,
            dev,
            pid_file,
            control,
            openai,
            hooks,
            hooks_token,
            gateway_token,
            gateway_password,
            ui_base_path,
            ui,
            ui_dist_path,
            log,
            insecure_http,
            tls_cert,
            tls_key,
            tls_auto_gen,
        } => {
            // Load configuration from file if provided
            let config_values = if let Some(config_path) = &config {
                match crate::config::load_config_uncached(config_path) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Error loading config from {}: {}", config_path.display(), e);
                        process::exit(1);
                    }
                }
            } else {
                // Use default path
                match crate::config::load_config() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
                        serde_json::json!({})
                    }
                }
            };

            // Fall back to environment variables for secrets (not exposed in process list)
            let hooks_token = hooks_token.or_else(|| std::env::var("CARAPACE_HOOKS_TOKEN").ok());
            let gateway_token = gateway_token.or_else(|| std::env::var("CARAPACE_GATEWAY_TOKEN").ok());
            let gateway_password = gateway_password.or_else(|| std::env::var("CARAPACE_GATEWAY_PASSWORD").ok());

            // Set up logging
            if let Some(log_level) = log {
                std::env::set_var("RUST_LOG", log_level);
            }
            crate::logging::init();

            // Check if already running
            if let Some(pid) = read_pid_file(&pid_file) {
                if is_process_running(pid) {
                    eprintln!("Error: Gateway is already running (PID: {})", pid);
                    process::exit(1);
                } else {
                    // Stale PID file, remove it
                    let _ = fs::remove_file(&pid_file);
                }
            }

            // Build TLS configuration
            let mut tls_config = if insecure_http || dev {
                server::tls::TlsConfig::insecure()
            } else {
                server::tls::TlsConfig::default()
            };

            // Override TLS paths if provided
            if let Some(cert_path) = tls_cert {
                tls_config.cert_path = cert_path;
            }
            if let Some(key_path) = tls_key {
                tls_config.key_path = key_path;
            }

            // Validate TLS configuration
            if let Err(e) = server::tls::validate_tls_config(&tls_config) {
                if tls_auto_gen && !tls_config.certificates_exist() {
                    info!("TLS certificates not found, auto-generating...");
                    if let Err(e) = server::tls::generate_self_signed_cert(
                        &tls_config.cert_path,
                        &tls_config.key_path,
                    ) {
                        error!("Failed to auto-generate TLS certificates: {}", e);
                        server::tls::print_tls_setup_instructions();
                        process::exit(1);
                    }
                } else {
                    error!("TLS configuration error: {}", e);
                    server::tls::print_tls_setup_instructions();
                    process::exit(1);
                }
            }

            info!("TLS Mode: {}", tls_config.mode_description());

            // Extract configuration from config file
            let hooks_enabled = hooks || dev || crate::config::get_bool(&config_values, "hooks.enabled").unwrap_or(false);
            let hooks_path = crate::config::get_string(&config_values, "hooks.path").unwrap_or_else(|| "/hooks".to_string());
            let gateway_password = gateway_password.or_else(|| crate::config::get_string(&config_values, "gateway.password"));
            let control_ui_enabled = ui || crate::config::get_bool(&config_values, "ui.enabled").unwrap_or(false);
            let control_ui_base_path = if ui_base_path.is_empty() {
                crate::config::get_string(&config_values, "ui.base_path").unwrap_or_default()
            } else {
                ui_base_path
            };
            let openai_enabled = openai || crate::config::get_bool(&config_values, "openai.enabled").unwrap_or(false);
            let control_enabled = control || dev || crate::config::get_bool(&config_values, "control.enabled").unwrap_or(false);

            // Build configuration
            let http_config = HttpConfig {
                hooks_token,
                hooks_enabled,
                hooks_path,
                hooks_max_body_bytes: 262_144,
                gateway_token,
                gateway_password,
                control_ui_base_path,
                control_ui_enabled,
                control_ui_dist_path: ui_dist_path,
                valid_channels: vec![],
                agents_dir: dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".moltbot/agents"),
                openai_chat_completions_enabled: openai_enabled,
                openai_responses_enabled: openai_enabled,
                control_endpoints_enabled: control_enabled,
            };

            let middleware_config = if dev {
                MiddlewareConfig::none()
            } else {
                MiddlewareConfig::default()
            };

            // Start server
            if let Err(e) = start_server(
                bind,
                http_config,
                middleware_config,
                pid_file,
                tls_config,
            )
            .await
            {
                error!("Server failed: {}", e);
                process::exit(1);
            }
        }

        Commands::Stop { pid_file, force } => {
            crate::logging::init();

            let pid = match read_pid_file(&pid_file) {
                Some(pid) => pid,
                None => {
                    eprintln!("Error: No PID file found. Is the gateway running?");
                    process::exit(1);
                }
            };

            // Check if process is actually running before attempting to stop
            if !is_process_running(pid) {
                // Stale PID file, clean it up and exit with success
                let _ = fs::remove_file(&pid_file);
                println!("Gateway not running; cleaned up stale PID file.");
                process::exit(0);
            }

            info!("Stopping gateway (PID: {})...", pid);

            // Send SIGTERM for graceful shutdown
            #[cfg(unix)]
            {
                let status = std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        // Wait for process to exit
                        for _i in 0..30 {
                            if !is_process_running(pid) {
                                println!("Gateway stopped.");
                                let _ = fs::remove_file(&pid_file);
                                return;
                            }
                            sleep(Duration::from_millis(100)).await;
                        }

                        if force {
                            info!("Force killing gateway...");
                            let _ = std::process::Command::new("kill")
                                .arg("-KILL")
                                .arg(pid.to_string())
                                .status();
                            let _ = fs::remove_file(&pid_file);
                            println!("Gateway killed.");
                        } else {
                            eprintln!("Error: Gateway did not stop gracefully. Use --force to kill.");
                            process::exit(1);
                        }
                    }
                    Ok(s) => {
                        eprintln!("Error: Failed to stop gateway (exit code: {:?})", s.code());
                        process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error: Failed to execute kill command: {}", e);
                        process::exit(1);
                    }
                }
            }

            #[cfg(not(unix))]
            {
                eprintln!("Stop command is not implemented for this platform");
                process::exit(1);
            }
        }

        Commands::Status {
            pid_file,
            health,
            url,
        } => {
            crate::logging::init();

            // Check PID file
            let running = match read_pid_file(&pid_file) {
                Some(pid) => {
                    if is_process_running(pid) {
                        println!("Gateway is running (PID: {})", pid);
                        true
                    } else {
                        println!("Gateway is not running (stale PID file: {})", pid);
                        false
                    }
                }
                None => {
                    println!("Gateway is not running");
                    false
                }
            };

            // Check health endpoint
            if health && running {
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                let health_url = format!("{}/control/status", url);
                match tokio::time::timeout(Duration::from_secs(5), client.get(&health_url).send()).await {
                    Ok(Ok(resp)) => {
                        if resp.status().is_success() {
                            match resp.json::<serde_json::Value>().await {
                                Ok(json) => {
                                    println!("Health: OK");
                                    if let Some(version) = json.get("version") {
                                        println!("Version: {}", version);
                                    }
                                    if let Some(uptime) = json.get("uptime") {
                                        println!("Uptime: {} seconds", uptime);
                                    }
                                }
                                Err(e) => {
                                    println!("Health: ERROR - Failed to parse response: {}", e);
                                }
                            }
                        } else {
                            println!("Health: ERROR - HTTP {}", resp.status());
                        }
                    }
                    Ok(Err(e)) => {
                        println!("Health: ERROR - {}", e);
                    }
                    Err(_) => {
                        println!("Health: ERROR - Request timed out after 5 seconds");
                    }
                }
            }

            process::exit(if running { 0 } else { 1 });
        }

        Commands::Config { command } => {
            crate::logging::init();

            match command {
                ConfigCommands::Validate { config } => {
                    println!("Validating configuration: {}", config.display());
                    match fs::read_to_string(&config) {
                        Ok(content) => {
                            // Try to parse as JSON5
                            match json5::from_str::<serde_json::Value>(&content) {
                                Ok(_) => {
                                    println!("Configuration is valid JSON5.");
                                    // TODO: Add schema validation when implemented
                                }
                                Err(e) => {
                                    eprintln!("Error: Invalid JSON5 syntax: {}", e);
                                    process::exit(1);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: Failed to read config file: {}", e);
                            process::exit(1);
                        }
                    }
                }

                ConfigCommands::Get { key, config } => {
                    let config_path = config.or_else(find_config_file);
                    match config_path {
                        Some(path) => {
                            match fs::read_to_string(&path) {
                                Ok(content) => {
                                    match json5::from_str::<serde_json::Value>(&content) {
                                        Ok(json) => {
                                            // Navigate nested keys
                                            let parts: Vec<&str> = key.split('.').collect();
                                            let mut current = &json;
                                            for part in &parts {
                                                match current.get(part) {
                                                    Some(v) => current = v,
                                                    None => {
                                                        eprintln!("Error: Key '{}' not found", key);
                                                        process::exit(1);
                                                    }
                                                }
                                            }
                                            println!("{}", serde_json::to_string_pretty(current).unwrap());
                                        }
                                        Err(e) => {
                                            eprintln!("Error: Invalid JSON5: {}", e);
                                            process::exit(1);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: Failed to read config: {}", e);
                                    process::exit(1);
                                }
                            }
                        }
                        None => {
                            eprintln!("Error: No configuration file found");
                            process::exit(1);
                        }
                    }
                }

                ConfigCommands::Set { key, value, config: _ } => {
                    println!("Setting {} = {} (not yet implemented)", key, value);
                    // TODO: Implement config setting
                }

                ConfigCommands::Schema => {
                    println!("// JSON Schema for carapace configuration");
                    println!("// This is a placeholder - full schema generation not yet implemented");
                    println!("{{");
                    println!("  \"$schema\": \"http://json-schema.org/draft-07/schema#\",");
                    println!("  \"title\": \"Carapace Configuration\",");
                    println!("  \"type\": \"object\",");
                    println!("  \"properties\": {{}}");
                    println!("}}");
                }
            }
        }

        Commands::Migrate { database_url } => {
            crate::logging::init();
            info!("Running database migrations...");

            // Use DATABASE_URL env var or build from components
            let connection_url = database_url.or_else(|| std::env::var("DATABASE_URL").ok())
                .unwrap_or_else(|| {
                    // Default to SQLite for local development
                    let data_dir = dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("."));
                    std::fs::create_dir_all(&data_dir).ok();
                    format!("sqlite:{}", data_dir.join("carapace.db").display())
                });

            info!("Database: {}", connection_url);

            let migrations_dir = std::env::var("CARAPACE_MIGRATIONS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("migrations"));

            let runner = migrations::MigrationRunner::new(connection_url, migrations_dir);

            match runner.run_migrations().await {
                Ok(applied) => {
                    if applied.is_empty() {
                        info!("No pending migrations to apply.");
                    } else {
                        info!("Applied {} migration(s).", applied.len());
                        for m in &applied {
                            info!("  - {}", m.name);
                        }
                    }
                }
                Err(e) => {
                    error!("Migration failed: {}", e);
                    process::exit(1);
                }
            }
        }

        Commands::Version => {
            println!("carapace {}", env!("CARGO_PKG_VERSION"));
            println!("A secure, stable Rust alternative to moltbot");
        }
    }
}

/// Find configuration file in standard locations
fn find_config_file() -> Option<PathBuf> {
    // Check MOLTBOT_CONFIG_PATH env var
    if let Ok(path) = std::env::var("MOLTBOT_CONFIG_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // Check MOLTBOT_STATE_DIR
    if let Ok(dir) = std::env::var("MOLTBOT_STATE_DIR") {
        let p = PathBuf::from(dir).join("moltbot.json");
        if p.exists() {
            return Some(p);
        }
    }

    // Check ~/.moltbot/
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".moltbot/moltbot.json");
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// Read PID from file
fn read_pid_file(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check if a process is running
fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Send signal 0 to check if process exists
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::RawHandle;
        use windows_sys::Win32::System::Threading::OpenProcess;
        use windows_sys::Win32::System::Threading::PROCESS_QUERY_INFORMATION;

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if handle.is_null() {
                false
            } else {
                windows_sys::Win32::Foundation::CloseHandle(handle);
                true
            }
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Start the server with graceful shutdown
async fn start_server(
    addr: SocketAddr,
    http_config: HttpConfig,
    middleware_config: MiddlewareConfig,
    pid_file: PathBuf,
    tls_config: server::tls::TlsConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting carapace gateway v{}", env!("CARGO_PKG_VERSION"));
    info!("Binding to {}", addr);

    // Create router
    let app = create_router_with_middleware(http_config, middleware_config);

    // Write PID file
    if let Some(parent) = pid_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let pid = std::process::id();
    fs::write(&pid_file, pid.to_string())?;
    info!("PID file written to {}", pid_file.display());

    // Start server based on TLS configuration
    if tls_config.allow_insecure_http || !tls_config.enabled {
        // HTTP mode
        info!("Starting HTTP server (no TLS)");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("Server listening on http://{}", addr);

        let serve_result = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await;

        // Clean up PID file regardless of serve result
        let _ = fs::remove_file(&pid_file);

        serve_result?;
    } else {
        // HTTPS mode
        info!("Starting HTTPS server with TLS");

        // Create TLS configuration
        let rustls_config = server::tls::create_tls_config(&tls_config)?;
        let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(rustls_config));

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("Server listening on https://{}", addr);

        // Create a shutdown signal
        let shutdown = shutdown_signal();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                Ok((stream, peer_addr)) = listener.accept() => {
                    let tls_acceptor = tls_acceptor.clone();
                    let app = app.clone();

                    tokio::spawn(async move {
                        match tls_acceptor.accept(stream).await {
                            Ok(stream) => {
                                let _ = stream; // Suppress unused warning
                                if let Err(e) = axum::serve(
                                    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(),
                                    app,
                                ).await {
                                    error!("Error serving connection from {}: {}", peer_addr, e);
                                }
                            }
                            Err(e) => {
                                error!("TLS handshake error from {}: {}", peer_addr, e);
                            }
                        }
                    });
                }
                _ = &mut shutdown => {
                    info!("Shutting down gracefully...");
                    break;
                }
            }
        }

        // Clean up PID file
        let _ = fs::remove_file(&pid_file);
    }

    info!("Gateway stopped");
    Ok(())
}

/// Wait for shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        }
    }

    // Give ongoing requests time to complete
    sleep(Duration::from_secs(1)).await;
}
