//! Flow System Module
//!
//! A simple event-based flow system for routing messages between channels,
//! triggering agents, and automating workflows. Inspired by n8n's concept
//! but simplified for carapace's use case.
//!
//! # Example Flow Configuration
//!
//! ```yaml
//! flows:
//!   - name: "Telegram to Discord Bridge"
//!     trigger:
//!       type: "channel_message"
//!       channel: "telegram"
//!     conditions:
//!       - field: "text"
//!         contains: "!bridge"
//!     actions:
//!       - type: "forward"
//!         to_channel: "discord"
//!
//!   - name: "Weather Agent Handler"
//!     trigger:
//!       type: "command"
//!       command: "weather"
//!     actions:
//!       - type: "agent"
//!         agent: "weather-agent"
//!         deliver_response: true
//!
//!   - name: "Auto-summarize"
//!     trigger:
//!       type: "channel_message"
//!       channel: "telegram"
//!     conditions:
//!       - field: "text"
//!         starts_with: "!summarize"
//!     actions:
//!       - type: "transform"
//!         template: "Summarizing: {{text}}"
//!       - type: "agent"
//!         agent: "summary-agent"
//! ```

pub mod config;
pub mod engine;
pub mod integrate;

pub use config::{Action, Condition, Flow, FlowConfig, FlowCondition, FlowTrigger, TriggerType};
pub use engine::{create_engine, FlowEngine, FlowResult};
pub use integrate::{create_flow_integration, FlowIntegration, FlowIntegrationError, FlowStats};
