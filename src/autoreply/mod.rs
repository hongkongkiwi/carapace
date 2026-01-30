//! Auto-reply Module
//!
//! Provides automated responses based on message triggers.
//! Supports multiple trigger types, template variables, cooldowns, and actions.

pub mod config;
pub mod engine;

pub use config::{ActionType, AutoReplyConfig, AutoReplyRule, ResponseType, TriggerType};
pub use engine::{create_engine, AutoReplyEngine, MatchContext, MatchResult, RuleStats};
