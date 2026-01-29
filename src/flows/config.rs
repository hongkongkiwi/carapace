//! Flow configuration types.
//!
//! Defines the configuration structures for flows, triggers, conditions, and actions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main flow configuration container.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowConfig {
    /// List of configured flows.
    pub flows: Vec<Flow>,
}

/// A single flow with trigger, conditions, and actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    /// Unique identifier for the flow.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this flow does.
    #[serde(default)]
    pub description: String,
    /// Whether the flow is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// What triggers this flow.
    pub trigger: FlowTrigger,
    /// Optional conditions that must be met.
    #[serde(default)]
    pub conditions: Vec<FlowCondition>,
    /// Actions to execute when triggered.
    pub actions: Vec<Action>,
    /// Priority (higher = executed first).
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    0
}

/// Types of triggers that can start a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowTrigger {
    /// Trigger on messages from a specific channel.
    ChannelMessage {
        /// Channel name to listen on.
        channel: String,
    },
    /// Trigger on a specific command.
    Command {
        /// Command name (without prefix).
        command: String,
    },
    /// Trigger on webhook requests.
    Webhook {
        /// Webhook path/ID.
        path: String,
    },
    /// Trigger on a schedule (cron expression).
    Schedule {
        /// Cron expression (e.g., "0 * * * *").
        cron: String,
    },
    /// Trigger on any event of a certain type.
    Event {
        /// Event type to listen for.
        event_type: String,
    },
    /// Trigger when user joins a channel.
    #[serde(rename = "user_join")]
    UserJoin {
        /// Channel to monitor.
        channel: String,
    },
    /// Trigger when user leaves a channel.
    #[serde(rename = "user_leave")]
    UserLeave {
        /// Channel to monitor.
        channel: String,
    },
    /// Manual trigger only (via API).
    Manual {
        /// Flow ID to trigger.
        flow_id: String,
    },
}

/// Condition for filtering when a flow executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "field_type", rename_all = "snake_case")]
pub enum FlowCondition {
    /// Check message text.
    Text {
        /// Matching operator.
        operator: ConditionOperator,
        /// Value to compare against.
        value: String,
    },
    /// Check message sender.
    Sender {
        /// Matching operator.
        operator: ConditionOperator,
        /// Username or pattern to match.
        value: String,
    },
    /// Check if sender is in a list.
    SenderIn {
        /// List of allowed usernames.
        users: Vec<String>,
    },
    /// Check if sender is in a role.
    SenderRole {
        /// Required role.
        role: String,
    },
    /// Check message has attachment.
    HasAttachment {
        /// Required attachment type (image, file, etc.).
        #[serde(default)]
        type_: Option<String>,
    },
    /// Check message word count.
    WordCount {
        /// Comparison operator.
        operator: ConditionOperator,
        /// Count to compare.
        value: u32,
    },
    /// Custom expression (advanced).
    Expression {
        /// Expression to evaluate.
        expression: String,
    },
}

/// Operators for condition matching.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Exact match.
    Equals,
    /// Contains substring.
    Contains,
    /// Starts with prefix.
    StartsWith,
    /// Ends with suffix.
    EndsWith,
    /// Regex match.
    Regex,
    /// Greater than (for numeric).
    GreaterThan,
    /// Less than (for numeric).
    LessThan,
    /// Greater than or equal.
    GreaterEqual,
    /// Less than or equal.
    LessEqual,
    /// Not equal.
    NotEquals,
    /// In list.
    In,
    /// Not in list.
    NotIn,
}

/// Actions that can be performed by a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Forward message to channel.
    Forward {
        /// Target channel name.
        to_channel: String,
    },
    /// Send a response.
    Respond {
        /// Response message template.
        message: String,
    },
    /// Send to multiple channels.
    Broadcast {
        /// Target channel names.
        channels: Vec<String>,
        /// Message template.
        message: String,
    },
    /// Trigger an agent.
    Agent {
        /// Agent name to run.
        agent: String,
        /// Whether to deliver response via channel.
        #[serde(default)]
        deliver_response: bool,
    },
    /// Transform message content.
    Transform {
        /// Template with placeholders.
        template: String,
    },
    /// Log the event.
    Log {
        /// Log level.
        #[serde(default = "default_log_level")]
        level: String,
        /// Message template.
        message: String,
    },
    /// Set a variable.
    SetVariable {
        /// Variable name.
        name: String,
        /// Value expression.
        value: String,
    },
    /// Wait/sleep.
    Wait {
        /// Duration in seconds.
        seconds: u64,
    },
    /// Conditional branch.
    Branch {
        /// Condition to check.
        condition: Box<FlowCondition>,
        /// Actions if true.
        then: Vec<Action>,
        /// Actions if false (optional).
        #[serde(default)]
        else_: Vec<Action>,
    },
    /// Stop flow execution.
    Stop {
        /// Stop with error.
        #[serde(default)]
        error: bool,
    },
    /// Call webhook.
    Webhook {
        /// URL to call.
        url: String,
        /// HTTP method.
        #[serde(default = "default_webhook_method")]
        method: String,
        /// Request body template.
        #[serde(default)]
        body: Option<String>,
    },
    /// Execute code.
    Execute {
        /// Code to execute.
        code: String,
    },
    /// Add reaction to message.
    React {
        /// Emoji to add.
        emoji: String,
    },
    /// Execute multiple actions in parallel.
    Parallel {
        /// Actions to execute in parallel.
        actions: Vec<Action>,
    },
    /// Call another flow as a subflow.
    Subflow {
        /// Flow ID to call.
        flow_id: String,
        /// Whether to wait for the subflow to complete.
        #[serde(default = "default_true")]
        wait: bool,
        /// Input data to pass to the subflow.
        #[serde(default)]
        input: HashMap<String, String>,
    },
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

/// Simplified trigger type for enum access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerType {
    #[serde(rename = "channel_message")]
    ChannelMessage { channel: String },
    #[serde(rename = "command")]
    Command { command: String },
    #[serde(rename = "webhook")]
    Webhook { path: String },
    #[serde(rename = "schedule")]
    Schedule { cron: String },
    #[serde(rename = "event")]
    Event { event_type: String },
    #[serde(rename = "user_join")]
    UserJoin { channel: String },
    #[serde(rename = "user_leave")]
    UserLeave { channel: String },
    #[serde(rename = "manual")]
    Manual { flow_id: String },
}

/// Simplified condition for enum access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowConditionType {
    #[serde(rename = "text")]
    Text { operator: String, value: String },
    #[serde(rename = "sender")]
    Sender { operator: String, value: String },
    #[serde(rename = "sender_in")]
    SenderIn { users: Vec<String> },
    #[serde(rename = "sender_role")]
    SenderRole { role: String },
    #[serde(rename = "has_attachment")]
    HasAttachment { type_: Option<String> },
    #[serde(rename = "word_count")]
    WordCount { operator: String, value: u32 },
    #[serde(rename = "expression")]
    Expression { expression: String },
}

/// Simplified action for enum access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    #[serde(rename = "forward")]
    Forward { to_channel: String },
    #[serde(rename = "respond")]
    Respond { message: String },
    #[serde(rename = "broadcast")]
    Broadcast { channels: Vec<String>, message: String },
    #[serde(rename = "agent")]
    Agent { agent: String, deliver_response: bool },
    #[serde(rename = "transform")]
    Transform { template: String },
    #[serde(rename = "log")]
    Log { level: String, message: String },
    #[serde(rename = "set_variable")]
    SetVariable { name: String, value: String },
    #[serde(rename = "wait")]
    Wait { seconds: u64 },
    #[serde(rename = "branch")]
    Branch { condition: Box<FlowConditionType>, then: Vec<ActionType>, else_: Vec<ActionType> },
    #[serde(rename = "stop")]
    Stop { error: bool },
    #[serde(rename = "webhook")]
    Webhook { url: String, method: String, body: Option<String> },
    #[serde(rename = "execute")]
    Execute { code: String },
    #[serde(rename = "react")]
    React { emoji: String },
    #[serde(rename = "parallel")]
    Parallel { actions: Vec<ActionType> },
    #[serde(rename = "subflow")]
    Subflow { flow_id: String, wait: bool, input: HashMap<String, String> },
}

/// Legacy/simple condition struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Field to check.
    pub field: String,
    /// Matching operator (contains, equals, etc.).
    pub operator: String,
    /// Value to match.
    pub value: String,
}

/// Legacy/simple action struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLegacy {
    /// Action type.
    #[serde(rename = "type")]
    pub action_type: String,
    /// Target channel (for forward).
    #[serde(default)]
    pub to_channel: String,
    /// Message template.
    #[serde(default)]
    pub message: String,
    /// Agent name.
    #[serde(default)]
    pub agent: String,
    /// Whether to deliver response.
    #[serde(default)]
    pub deliver_response: bool,
    /// Template for transform.
    #[serde(default)]
    pub template: String,
    /// Broadcast channels.
    #[serde(default)]
    pub channels: Vec<String>,
    /// Variables to set.
    #[serde(default)]
    pub variables: HashMap<String, String>,
}
