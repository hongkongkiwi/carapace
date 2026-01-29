//! Polling Module
//!
//! Provides poll/survey functionality for channels.
//! Supports single/multiple choice, rating, quiz, and open-ended polls.

pub mod config;
pub mod engine;

pub use config::{
    PollConfig, PollFilter, PollOption, PollResults, PollType, PollVisibility, PollVote,
};
pub use engine::{create_engine, PollEngine, PollEngineStats};
