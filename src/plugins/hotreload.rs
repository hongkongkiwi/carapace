//! Plugin Hot Reload
//!
//! Watches plugin directories and hot-reloads plugins when files change.
//! Uses polling-based file system watching for compatibility.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::broadcast;

/// Plugin error types
#[derive(Debug, Error)]
pub enum HotReloadError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin load error: {0}")]
    LoadError(String),

    #[error("Watcher error: {0}")]
    WatcherError(String),

    #[error("Plugin already watching: {0}")]
    AlreadyWatching(String),
}

/// Hot reload event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum HotReloadEvent {
    /// Plugin was modified and needs reload
    Modified { plugin_id: String },
    /// Plugin was added
    Added { plugin_id: String },
    /// Plugin was removed
    Removed { plugin_id: String },
    /// Error occurred
    Error { plugin_id: String, message: String },
}

/// Plugin hot reload manager
#[derive(Debug)]
pub struct HotReloadManager {
    /// Sender for hot reload events
    event_tx: tokio::sync::broadcast::Sender<HotReloadEvent>,
    /// Currently watched plugins
    watched_plugins: RwLock<HashMap<String, WatchedPlugin>>,
    /// Debounce duration
    debounce_duration: Duration,
    /// Poll interval
    poll_interval: Duration,
}

/// Watched plugin state
#[derive(Debug, Clone)]
struct WatchedPlugin {
    path: PathBuf,
    last_modified: SystemTime,
    last_event: SystemTime,
}

impl HotReloadManager {
    /// Create a new hot reload manager
    pub fn new() -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(100);
        Self {
            event_tx,
            watched_plugins: RwLock::new(HashMap::new()),
            debounce_duration: Duration::from_millis(500),
            poll_interval: Duration::from_millis(100),
        }
    }

    /// Subscribe to hot reload events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<HotReloadEvent> {
        self.event_tx.subscribe()
    }

    /// Start watching a plugin directory or file
    pub fn watch_plugin(
        &self,
        plugin_id: &str,
        path: PathBuf,
    ) -> Result<(), HotReloadError> {
        if self.watched_plugins.read().contains_key(plugin_id) {
            return Err(HotReloadError::AlreadyWatching(plugin_id.to_string()));
        }

        // Get initial modified time
        let last_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::UNIX_EPOCH);

        let watched = WatchedPlugin {
            path: path.clone(),
            last_modified,
            last_event: SystemTime::UNIX_EPOCH,
        };

        {
            let mut watched_plugins = self.watched_plugins.write();
            watched_plugins.insert(plugin_id.to_string(), watched);
        }

        tracing::info!("Watching plugin {} at {}", plugin_id, path.display());

        Ok(())
    }

    /// Stop watching a plugin
    pub fn unwatch_plugin(&self, plugin_id: &str) -> Result<(), HotReloadError> {
        let mut watched = self.watched_plugins.write();
        if let Some(_path) = watched.remove(plugin_id) {
            tracing::info!("Stopped watching plugin {}", plugin_id);
        }
        Ok(())
    }

    /// Check for file changes and trigger reloads
    pub fn check_changes(&self) -> Vec<HotReloadEvent> {
        let mut events = Vec::new();
        let now = SystemTime::now();

        let watched_plugins = self.watched_plugins.read();
        for (plugin_id, plugin) in watched_plugins.iter() {
            // Skip if recently triggered (debounce)
            if plugin.last_event.elapsed().map_or(false, |d| d < self.debounce_duration) {
                continue;
            }

            // Check if file exists
            if !plugin.path.exists() {
                events.push(HotReloadEvent::Removed {
                    plugin_id: plugin_id.clone(),
                });
                continue;
            }

            // Check modification time
            if let Ok(metadata) = std::fs::metadata(&plugin.path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > plugin.last_modified {
                        let mut watched = self.watched_plugins.write();
                        if let Some(wp) = watched.get_mut(plugin_id) {
                            wp.last_modified = modified;
                            wp.last_event = now;
                        }

                        events.push(HotReloadEvent::Modified {
                            plugin_id: plugin_id.clone(),
                        });
                    }
                }
            }
        }

        // Send events to subscribers
        for event in &events {
            let _ = self.event_tx.send(event.clone());
        }

        events
    }

    /// Get list of watched plugins
    pub fn watched_plugins(&self) -> Vec<String> {
        self.watched_plugins.read().keys().cloned().collect()
    }

    /// Set poll interval
    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
    }

    /// Get poll interval
    pub fn poll_interval(&self) -> Duration {
        self.poll_interval
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin reload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Enable hot reload
    pub enabled: bool,
    /// Directories to watch
    pub watch_dirs: Vec<PathBuf>,
    /// File patterns to watch
    pub patterns: Vec<String>,
    /// Debounce duration in milliseconds
    pub debounce_ms: u64,
    /// Poll interval in milliseconds
    pub poll_interval_ms: u64,
    /// Enable auto-reload on change
    pub auto_reload: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            watch_dirs: vec![PathBuf::from("plugins")],
            patterns: vec!["*.wasm".to_string(), "*.json".to_string()],
            debounce_ms: 500,
            poll_interval_ms: 100,
            auto_reload: true,
        }
    }
}

/// Create a shared hot reload manager
pub fn create_manager() -> Arc<RwLock<HotReloadManager>> {
    Arc::new(RwLock::new(HotReloadManager::new()))
}

/// Run hot reload polling loop
pub async fn run_hot_reload_loop(
    manager: Arc<RwLock<HotReloadManager>>,
    reload_callback: impl Fn(&str) -> Result<(), String> + Send + Sync + 'static,
) {
    let interval = manager.read().poll_interval();
    let mut tick = tokio::time::interval(interval);

    loop {
        tick.tick().await;

        let events = {
            let manager_guard = manager.read();
            manager_guard.check_changes()
        };

        for event in events {
            if let HotReloadEvent::Modified { plugin_id } = event {
                tracing::info!("Plugin {} modified, reloading...", plugin_id);

                if let Err(e) = reload_callback(&plugin_id) {
                    tracing::error!("Failed to reload plugin {}: {}", plugin_id, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hot_reload_manager() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("test.wasm");
        fs::write(&plugin_path, "test content").unwrap();

        let manager = HotReloadManager::new();
        manager.watch_plugin("test-plugin", plugin_path).unwrap();

        let watched = manager.watched_plugins();
        assert_eq!(watched, vec!["test-plugin"]);

        manager.unwatch_plugin("test-plugin").unwrap();

        let watched = manager.watched_plugins();
        assert!(watched.is_empty());
    }

    #[test]
    fn test_hot_reload_config_default() {
        let config = HotReloadConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.watch_dirs.len(), 1);
        assert_eq!(config.debounce_ms, 500);
        assert_eq!(config.poll_interval_ms, 100);
    }

    #[test]
    fn test_check_changes() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("test.wasm");
        fs::write(&plugin_path, "initial content").unwrap();

        let manager = HotReloadManager::new();
        manager.watch_plugin("test-plugin", plugin_path.clone()).unwrap();

        // First check - no changes
        let events = manager.check_changes();
        assert!(events.is_empty());

        // Modify the file
        fs::write(&plugin_path, "modified content").unwrap();

        // Need to wait for debounce
        std::thread::sleep(Duration::from_millis(600));

        // Second check - should detect change
        let events = manager.check_changes();
        assert_eq!(events.len(), 1);
        match &events[0] {
            HotReloadEvent::Modified { plugin_id } => {
                assert_eq!(plugin_id, "test-plugin");
            }
            _ => panic!("Expected Modified event"),
        }
    }

    #[test]
    fn test_event_subscription() {
        let manager = HotReloadManager::new();
        let mut rx = manager.subscribe();

        // Create a temp file and watch it
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("test.wasm");
        fs::write(&plugin_path, "test").unwrap();
        manager.watch_plugin("test-plugin", plugin_path).unwrap();

        // Send an event directly
        let event = HotReloadEvent::Modified {
            plugin_id: "test-plugin".to_string(),
        };
        let _ = manager.event_tx.send(event.clone());

        // Receive the event
        let received = rx.try_recv();
        assert!(received.is_ok());
        assert_eq!(received.unwrap(), event);
    }
}
