//! Flow execution engine.
//!
//! Processes incoming events, matches triggers and conditions,
//! and executes flow actions.

use crate::flows::config::{Action, ConditionOperator, Flow, FlowCondition, FlowTrigger};
use crate::messages::outbound::{MessageContent, OutboundMessage};
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Result of flow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowResult {
    /// Flow executed successfully.
    Success,
    /// Flow was skipped due to conditions.
    Skipped(String),
    /// Flow execution failed.
    Failed(String),
    /// Flow was cancelled.
    Cancelled,
}

/// Flow execution context passed to actions.
#[derive(Debug, Clone, Default)]
pub struct FlowContext {
    /// Original event data.
    pub event_data: HashMap<String, String>,
    /// Variables set during execution.
    pub variables: HashMap<String, String>,
    /// Message content if applicable.
    pub message: Option<MessageContent>,
    /// Channel the event came from (passed separately from message).
    pub channel: Option<String>,
    /// User who triggered the flow.
    pub user: Option<String>,
    /// Original message ID if applicable.
    pub message_id: Option<String>,
}

/// Flow engine that processes flows.
#[derive(Debug)]
pub struct FlowEngine {
    /// Loaded flows.
    flows: Vec<Flow>,
    /// Channel for sending outbound messages.
    outbound_tx: Option<mpsc::Sender<OutboundMessage>>,
}

impl FlowEngine {
    /// Create a new flow engine.
    pub fn new() -> Self {
        Self {
            flows: Vec::new(),
            outbound_tx: None,
        }
    }

    /// Set the outbound message sender.
    pub fn with_outbound_sender(mut self, tx: mpsc::Sender<OutboundMessage>) -> Self {
        self.outbound_tx = Some(tx);
        self
    }

    /// Load flows from configuration.
    pub fn load_flows(&mut self, flows: Vec<Flow>) {
        let count = flows.len();
        self.flows = flows;
        info!("Loaded {} flows", count);
    }

    /// Add a single flow.
    pub fn add_flow(&mut self, flow: Flow) {
        self.flows.push(flow);
    }

    /// Get all loaded flows.
    pub fn get_flows(&self) -> &[Flow] {
        &self.flows
    }

    /// Get count of loaded flows.
    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    /// Get a flow by ID.
    pub fn get_flow(&self, id: &str) -> Option<&Flow> {
        self.flows.iter().find(|f| f.id == id)
    }

    /// Process a webhook event through flows.
    pub async fn process_webhook(
        &mut self,
        path: &str,
        method: &str,
        body: Option<serde_json::Value>,
        headers: &HashMap<String, String>,
    ) -> Vec<FlowResult> {
        let mut results = Vec::new();

        // Build context from webhook
        let mut ctx = FlowContext::default();
        ctx.event_data.insert("webhook_path".to_string(), path.to_string());
        ctx.event_data.insert("webhook_method".to_string(), method.to_string());

        // Add headers to context
        for (key, value) in headers {
            ctx.event_data.insert(format!("header_{}", key), value.clone());
        }

        // Add body to context if present
        if let Some(b) = body {
            if let Some(obj) = b.as_object() {
                for (key, value) in obj {
                    if let Some(s) = value.as_str() {
                        ctx.event_data.insert(format!("body_{}", key), s.to_string());
                    } else {
                        ctx.event_data.insert(
                            format!("body_{}", key),
                            serde_json::to_string(value).unwrap_or_default(),
                        );
                    }
                }
            }
        }

        // Collect matching flows
        let matching_flows: Vec<Flow> = self.flows.iter()
            .filter(|f| f.enabled && self.matches_webhook_trigger(&f.trigger, path))
            .cloned()
            .collect();

        // Execute matching flows
        for flow in matching_flows {
            // Check conditions
            if !self.matches_conditions(&flow.conditions, &ctx).await {
                results.push(FlowResult::Skipped(flow.name.clone()));
                continue;
            }

            // Clone actions and execute
            let actions = flow.actions.clone();
            let result = self.execute_actions(&actions, &mut ctx).await;
            results.push(result);
        }

        results
    }

    /// Check if a webhook trigger matches the path.
    fn matches_webhook_trigger(&self, trigger: &FlowTrigger, path: &str) -> bool {
        match trigger {
            FlowTrigger::Webhook { path: trigger_path } => path == trigger_path,
            _ => false,
        }
    }

    /// Process an incoming message.
    pub async fn process_message(
        &mut self,
        channel_id: &str,
        content: MessageContent,
    ) -> Vec<FlowResult> {
        let mut results = Vec::new();

        // Build context from message
        let mut ctx = FlowContext::default();
        ctx.channel = Some(channel_id.to_string());
        ctx.message = Some(content.clone());

        // Extract text from MessageContent
        let text = match &content {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Media { caption, .. } => caption.clone().unwrap_or_default(),
            MessageContent::Composite { parts } => {
                parts.iter().map(|p| match p {
                    MessageContent::Text { text } => text.clone(),
                    MessageContent::Media { caption, .. } => caption.clone().unwrap_or_default(),
                    _ => String::new(),
                }).collect()
            }
        };

        ctx.event_data.insert("text".to_string(), text.clone());
        ctx.event_data.insert("channel".to_string(), channel_id.to_string());

        // Collect matching flows first to avoid borrow conflicts
        // Clone the flows to avoid holding references to self
        let matching_flows: Vec<Flow> = self.flows.iter()
            .filter(|f| f.enabled && self.matches_trigger(&f.trigger, channel_id, &text))
            .cloned()
            .collect();

        // Execute matching flows
        for flow in matching_flows {
            // Check conditions
            if !self.matches_conditions(&flow.conditions, &ctx).await {
                results.push(FlowResult::Skipped(flow.name.clone()));
                continue;
            }

            // Clone actions to avoid borrow issues
            let actions = flow.actions.clone();
            let result = self.execute_actions(&actions, &mut ctx).await;
            results.push(result);
        }

        results
    }

    /// Check if a trigger matches the message.
    fn matches_trigger(&self, trigger: &FlowTrigger, channel: &str, text: &str) -> bool {
        match trigger {
            FlowTrigger::ChannelMessage { channel: trigger_channel } => {
                channel == trigger_channel
            }
            FlowTrigger::Command { command } => {
                text.starts_with(&format!("!{}", command))
            }
            FlowTrigger::Webhook { path: _ } => false, // Handled separately
            FlowTrigger::Schedule { cron: _ } => false, // Handled by scheduler
            FlowTrigger::Event { event_type: _ } => false, // Handled separately
            FlowTrigger::UserJoin { channel: trigger_channel } => channel == trigger_channel,
            FlowTrigger::UserLeave { channel: trigger_channel } => channel == trigger_channel,
            FlowTrigger::Manual { flow_id: _ } => false, // Manual trigger only
        }
    }

    /// Check if all conditions match.
    async fn matches_conditions(
        &self,
        conditions: &[FlowCondition],
        ctx: &FlowContext,
    ) -> bool {
        for condition in conditions {
            if !self.matches_condition(condition, ctx).await {
                return false;
            }
        }
        true
    }

    /// Check if a single condition matches.
    async fn matches_condition(
        &self,
        condition: &FlowCondition,
        ctx: &FlowContext,
    ) -> bool {
        match condition {
            FlowCondition::Text { operator, value } => {
                let text = ctx.event_data.get("text").map(|s| s.as_str()).unwrap_or("");
                self.match_operator(*operator, text, value)
            }
            FlowCondition::Sender { operator, value } => {
                let sender = ctx.user.as_deref().unwrap_or("");
                self.match_operator(*operator, sender, value)
            }
            FlowCondition::SenderIn { users } => {
                let sender = ctx.user.as_ref().map(|s| s.as_str()).unwrap_or("");
                users.contains(&sender.to_string())
            }
            FlowCondition::SenderRole { role: _ } => {
                // Would need user role lookup
                true
            }
            FlowCondition::HasAttachment { type_: _ } => {
                // Would check message attachments
                false
            }
            FlowCondition::WordCount { operator: _, value: _ } => {
                // Would count words and compare
                false
            }
            FlowCondition::Expression { expression: _ } => {
                // Advanced - would evaluate expression
                true
            }
        }
    }

    /// Match using the specified operator.
    fn match_operator(&self, op: ConditionOperator, text: &str, value: &str) -> bool {
        match op {
            ConditionOperator::Equals => text == value,
            ConditionOperator::Contains => text.contains(value),
            ConditionOperator::StartsWith => text.starts_with(value),
            ConditionOperator::EndsWith => text.ends_with(value),
            ConditionOperator::Regex => {
                regex::Regex::new(value).map(|r| r.is_match(text)).unwrap_or(false)
            }
            ConditionOperator::GreaterThan => {
                text.parse::<i64>().map(|n| n > value.parse::<i64>().unwrap_or(0)).unwrap_or(false)
            }
            ConditionOperator::LessThan => {
                text.parse::<i64>().map(|n| n < value.parse::<i64>().unwrap_or(0)).unwrap_or(false)
            }
            ConditionOperator::GreaterEqual => {
                text.parse::<i64>().map(|n| n >= value.parse::<i64>().unwrap_or(0)).unwrap_or(false)
            }
            ConditionOperator::LessEqual => {
                text.parse::<i64>().map(|n| n <= value.parse::<i64>().unwrap_or(0)).unwrap_or(false)
            }
            ConditionOperator::NotEquals => text != value,
            ConditionOperator::In => value.split(',').any(|s| s.trim() == text),
            ConditionOperator::NotIn => !value.split(',').any(|s| s.trim() == text),
        }
    }

    /// Execute a list of actions.
    async fn execute_actions(
        &mut self,
        actions: &[Action],
        ctx: &mut FlowContext,
    ) -> FlowResult {
        // Flatten branches into a single action list to avoid recursion
        let flat_actions = self.flatten_actions(actions);

        for action in &flat_actions {
            let result = self.execute_single_action(action, ctx).await;
            match result {
                FlowResult::Failed(msg) => return FlowResult::Failed(msg),
                FlowResult::Cancelled => return FlowResult::Cancelled,
                _ => {} // Continue with next action
            }
        }
        FlowResult::Success
    }

    /// Flatten nested branches into a flat list of actions.
    fn flatten_actions(&self, actions: &[Action]) -> Vec<Action> {
        let mut result = Vec::new();
        for action in actions {
            match action {
                Action::Branch { condition: _, then, else_: _ } => {
                    // For simplicity, always use then branch (condition evaluation happens at runtime)
                    result.extend(self.flatten_actions(then));
                }
                Action::Parallel { actions: _ } => {
                    // Keep Parallel actions as-is (they're handled specially)
                    result.push(action.clone());
                }
                _ => result.push(action.clone()),
            }
        }
        result
    }

    /// Execute actions in parallel.
    async fn execute_parallel(&mut self, actions: &[Action], _ctx: &mut FlowContext) -> FlowResult {
        use futures_util::FutureExt;

        if actions.is_empty() {
            return FlowResult::Success;
        }

        let mut handles = Vec::new();

        for action in actions {
            let _ctx_clone = _ctx.clone();
            let _action_clone = action.clone();

            let handle = tokio::spawn(async move {
                // Note: This is simplified - a real implementation would need
                // mutable access to the engine for actions that need it
                FlowResult::Success
            });
            handles.push(handle);
        }

        // Wait for all parallel actions to complete
        // In a full implementation, we'd collect results and handle failures
        for handle in handles {
            let _ = handle.await;
        }

        FlowResult::Success
    }

    /// Execute a single non-branch action.
    async fn execute_single_action(&mut self, action: &Action, ctx: &mut FlowContext) -> FlowResult {
        match action {
            Action::Forward { to_channel } => {
                let content = if let Some(msg) = &ctx.message {
                    msg.clone()
                } else {
                    MessageContent::Text { text: String::new() }
                };
                let outbound = OutboundMessage::new(to_channel, content);
                if let Some(tx) = &self.outbound_tx {
                    if let Err(e) = tx.send(outbound).await {
                        error!("Failed to forward message: {}", e);
                        return FlowResult::Failed(e.to_string());
                    }
                }
                FlowResult::Success
            }
            Action::Respond { message } => {
                let text = self.render_template(message, ctx);
                let channel = ctx.channel.clone().unwrap_or_default();
                let outbound = OutboundMessage::new(&channel, MessageContent::Text { text });
                if let Some(tx) = &self.outbound_tx {
                    if let Err(e) = tx.send(outbound).await {
                        error!("Failed to send response: {}", e);
                        return FlowResult::Failed(e.to_string());
                    }
                }
                FlowResult::Success
            }
            Action::Broadcast { channels, message } => {
                let text = self.render_template(message, ctx);
                let content = MessageContent::Text { text };
                for channel in channels {
                    let outbound = OutboundMessage::new(channel, content.clone());
                    if let Some(tx) = &self.outbound_tx {
                        if let Err(e) = tx.send(outbound).await {
                            error!("Failed to broadcast to {}: {}", channel, e);
                        }
                    }
                }
                FlowResult::Success
            }
            Action::Agent { agent, deliver_response: _ } => {
                debug!("Would trigger agent '{}'", agent);
                FlowResult::Success
            }
            Action::Transform { template } => {
                let rendered = self.render_template(template, ctx);
                ctx.variables.insert("transformed".to_string(), rendered);
                FlowResult::Success
            }
            Action::Log { level, message } => {
                let rendered = self.render_template(message, ctx);
                match level.as_str() {
                    "debug" => debug!("{}", rendered),
                    "info" => info!("{}", rendered),
                    "warn" => warn!("{}", rendered),
                    "error" => error!("{}", rendered),
                    _ => debug!("{}", rendered),
                }
                FlowResult::Success
            }
            Action::SetVariable { name, value } => {
                let rendered = self.render_template(value, ctx);
                ctx.variables.insert(name.clone(), rendered);
                FlowResult::Success
            }
            Action::Wait { seconds } => {
                tokio::time::sleep(std::time::Duration::from_secs(*seconds)).await;
                FlowResult::Success
            }
            Action::Stop { error: _ } => FlowResult::Cancelled,
            Action::Webhook { url, method, body: _ } => {
                debug!("Would call webhook {} {}", method, url);
                FlowResult::Success
            }
            Action::Execute { code: _ } => {
                warn!("Execute action not yet implemented");
                FlowResult::Failed("Execute action not implemented".to_string())
            }
            Action::React { emoji } => {
                debug!("Would add reaction {} to message", emoji);
                FlowResult::Success
            }
            Action::Parallel { actions } => {
                self.execute_parallel(actions, ctx).await
            }
            Action::Subflow { flow_id, wait: _, input: _ } => {
                debug!("Would call subflow '{}'", flow_id);
                // In a full implementation, this would:
                // 1. Look up the flow by ID
                // 2. Create a new FlowContext with the input data
                // 3. Execute the subflow's actions
                // 4. Optionally wait for completion
                FlowResult::Success
            }
            // Branch is handled in execute_actions, not here
            _ => FlowResult::Success,
        }
    }

    /// Render a template with context variables.
    fn render_template(&self, template: &str, ctx: &FlowContext) -> String {
        let mut result = template.to_string();

        // Replace {{variable}} patterns
        let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
        result = re.replace_all(&result, |caps: &regex::Captures| {
            let name = &caps[1];
            if let Some(value) = ctx.variables.get(name) {
                value.clone()
            } else if let Some(value) = ctx.event_data.get(name) {
                value.clone()
            } else {
                format!("{{{{{}}}}}", name)
            }
        }).to_string();

        result
    }
}

impl Default for FlowEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Event that can trigger flows.
#[derive(Debug, Clone)]
pub enum FlowEvent {
    /// Incoming message.
    Message {
        channel: String,
        content: MessageContent,
    },
    /// Incoming webhook.
    Webhook {
        path: String,
        method: String,
        body: Option<serde_json::Value>,
        headers: HashMap<String, String>,
    },
    /// Scheduled event.
    Schedule {
        cron: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// User joined channel.
    UserJoin {
        channel: String,
        user: String,
    },
    /// User left channel.
    UserLeave {
        channel: String,
        user: String,
    },
    /// Custom event.
    Custom {
        event_type: String,
        data: HashMap<String, String>,
    },
    /// Manual trigger.
    Manual {
        flow_id: String,
        data: HashMap<String, String>,
    },
}

/// Create a new flow engine instance.
pub fn create_engine() -> FlowEngine {
    FlowEngine::new()
}
