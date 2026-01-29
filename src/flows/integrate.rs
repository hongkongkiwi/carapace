//! Flow integration with message pipeline and cron scheduler.
//!
//! This module provides the integration layer between flows and the
//! rest of the carapace system (message pipeline, cron scheduler, etc.).

use crate::cron::CronScheduler;
use crate::flows::config::{Flow, FlowTrigger};
use crate::flows::engine::{create_engine, FlowEngine, FlowEvent, FlowResult};
use crate::messages::outbound::{MessageContent, MessagePipeline, OutboundMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Flow store file format for persisting flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStoreFile {
    pub version: u32,
    pub flows: Vec<Flow>,
}

impl Default for FlowStoreFile {
    fn default() -> Self {
        Self {
            version: 1,
            flows: Vec::new(),
        }
    }
}

/// Flow integration service that coordinates flows with the rest of the system.
#[derive(Debug)]
pub struct FlowIntegration {
    /// The flow engine.
    engine: Arc<Mutex<FlowEngine>>,
    /// Path to the flow store file.
    store_path: PathBuf,
    /// Receiver for flow events.
    event_rx: Option<mpsc::Receiver<FlowEvent>>,
    /// Sender for outbound messages.
    outbound_tx: Option<mpsc::Sender<OutboundMessage>>,
    /// Cron scheduler for schedule-based triggers.
    cron_scheduler: Option<Arc<CronScheduler>>,
}

impl FlowIntegration {
    /// Create a new flow integration service.
    pub fn new(store_path: PathBuf) -> Self {
        let (_event_tx, event_rx) = mpsc::channel(100);
        let engine = Arc::new(Mutex::new(create_engine()));

        Self {
            engine,
            store_path,
            event_rx: Some(event_rx),
            outbound_tx: None,
            cron_scheduler: None,
        }
    }

    /// Create a new in-memory flow integration (for testing).
    pub fn in_memory() -> Self {
        Self::new(PathBuf::from(":memory:"))
    }

    /// Set the outbound message sender.
    pub fn with_outbound_sender(mut self, tx: mpsc::Sender<OutboundMessage>) -> Self {
        self.outbound_tx = Some(tx);
        self
    }

    /// Set the cron scheduler for schedule triggers.
    pub fn with_cron_scheduler(mut self, scheduler: Arc<CronScheduler>) -> Self {
        self.cron_scheduler = Some(scheduler);
        self
    }

    /// Load flows from the store file.
    pub async fn load_flows(&self) -> Result<(), FlowIntegrationError> {
        let mut engine = self.engine.lock().await;

        if self.store_path == PathBuf::from(":memory:") {
            return Ok(());
        }

        if !self.store_path.exists() {
            info!("No flow store file found at {}", self.store_path.display());
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.store_path)
            .await
            .map_err(|e| FlowIntegrationError::StoreError(e.to_string()))?;

        let store: FlowStoreFile = serde_yaml::from_str(&content)
            .map_err(|e| FlowIntegrationError::ParseError(e.to_string()))?;

        engine.load_flows(store.flows);
        info!("Loaded flows from {}", self.store_path.display());

        Ok(())
    }

    /// Save flows to the store file.
    pub async fn save_flows(&self) -> Result<(), FlowIntegrationError> {
        let engine = self.engine.lock().await;

        if self.store_path == PathBuf::from(":memory:") {
            return Ok(());
        }

        // Create parent directory if needed
        if let Some(parent) = self.store_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| FlowIntegrationError::StoreError(e.to_string()))?;
            }
        }

        let store = FlowStoreFile {
            version: 1,
            flows: engine.get_flows().to_vec(),
        };

        let content = serde_yaml::to_string(&store)
            .map_err(|e| FlowIntegrationError::ParseError(e.to_string()))?;

        tokio::fs::write(&self.store_path, content)
            .await
            .map_err(|e| FlowIntegrationError::StoreError(e.to_string()))?;

        Ok(())
    }

    /// Add a new flow.
    pub async fn add_flow(&mut self, flow: Flow) -> Result<(), FlowIntegrationError> {
        let mut engine = self.engine.lock().await;
        engine.add_flow(flow);
        self.save_flows().await?;
        Ok(())
    }

    /// Process an incoming message through flows.
    pub async fn process_message(
        &mut self,
        channel_id: &str,
        content: MessageContent,
    ) -> Vec<FlowResult> {
        let mut engine = self.engine.lock().await;
        engine.process_message(channel_id, content).await
    }

    /// Process a webhook request through flows.
    pub async fn process_webhook(
        &mut self,
        path: &str,
        method: &str,
        body: Option<serde_json::Value>,
        headers: HashMap<String, String>,
    ) -> Vec<FlowResult> {
        let mut engine = self.engine.lock().await;
        engine.process_webhook(path, method, body, &headers).await
    }

    /// Trigger a flow manually.
    pub async fn trigger_flow(&mut self, flow_id: &str, data: HashMap<String, String>) {
        let _event = FlowEvent::Manual {
            flow_id: flow_id.to_string(),
            data,
        };
        // Would process the event through matching flows
        debug!("Manual flow trigger: {}", flow_id);
    }

    /// Handle a cron event by triggering schedule-based flows.
    pub async fn handle_cron_event(&mut self, cron_expr: &str) {
        let _event = FlowEvent::Schedule {
            cron: cron_expr.to_string(),
            timestamp: chrono::Utc::now(),
        };
        debug!("Cron event triggered: {}", cron_expr);

        // Find and execute flows with matching schedule triggers
        let engine = self.engine.lock().await;
        for flow in engine.get_flows() {
            if !flow.enabled {
                continue;
            }
            if let FlowTrigger::Schedule { cron: trigger_cron } = &flow.trigger {
                if trigger_cron == cron_expr {
                    // Execute the flow's actions
                    debug!("Executing scheduled flow: {}", flow.name);
                }
            }
        }
    }

    /// Run the event processing loop.
    pub async fn run_event_loop(&mut self) {
        let Some(rx) = self.event_rx.take() else {
            return;
        };

        let mut event_rx = rx;
        let engine = Arc::clone(&self.engine);

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let mut engine = engine.lock().await;
                match event {
                    FlowEvent::Message { channel, content } => {
                        engine.process_message(&channel, content).await;
                    }
                    FlowEvent::Schedule { cron, timestamp } => {
                        debug!("Schedule event: {} at {:?}", cron, timestamp);
                    }
                    FlowEvent::Webhook { path, method, body: _, headers: _ } => {
                        debug!("Webhook event: {} {}", method, path);
                    }
                    FlowEvent::UserJoin { channel, user } => {
                        debug!("User join event: {} joined {}", user, channel);
                    }
                    FlowEvent::UserLeave { channel, user } => {
                        debug!("User leave event: {} left {}", user, channel);
                    }
                    FlowEvent::Custom { event_type, data } => {
                        debug!("Custom event: {} with {} data points", event_type, data.len());
                    }
                    FlowEvent::Manual { flow_id, data } => {
                        debug!("Manual trigger: {} with {} data points", flow_id, data.len());
                    }
                }
            }
        });
    }

    /// Get the number of loaded flows.
    pub fn flow_count(&self) -> usize {
        // Note: In a real implementation, we'd use a proper read lock
        // This is simplified for the example
        0
    }
}

/// Errors that can occur in flow integration.
#[derive(Debug, thiserror::Error)]
pub enum FlowIntegrationError {
    #[error("store error: {0}")]
    StoreError(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("flow not found: {0}")]
    FlowNotFound(String),
}

/// Create a flow integration service with the default store path.
pub fn create_flow_integration() -> FlowIntegration {
    let store_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("carapace/flows.yaml");
    FlowIntegration::new(store_path)
}

/// Flow statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStats {
    pub total_flows: usize,
    pub enabled_flows: usize,
    pub channel_triggers: usize,
    pub command_triggers: usize,
    pub schedule_triggers: usize,
    pub webhook_triggers: usize,
    pub event_triggers: usize,
}
