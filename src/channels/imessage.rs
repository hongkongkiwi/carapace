//! iMessage Channel Implementation
//!
//! Provides messaging support via iMessage on macOS.
//! Uses AppleScript for sending and receiving messages.
//!
//! Note: Full iMessage support requires a Mac with Messages app configured.
//! Receiving messages requires polling or event hooks.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// iMessage channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImessageConfig {
    /// Enable iMessage channel
    pub enabled: bool,
    /// Default recipient phone number or email
    pub default_recipient: String,
    /// Send timeout in seconds
    pub send_timeout_secs: u64,
    /// Polling interval for incoming messages (if using polling)
    pub poll_interval_secs: u64,
    /// Max message length
    pub max_message_length: usize,
}

impl Default for ImessageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_recipient: String::new(),
            send_timeout_secs: 30,
            poll_interval_secs: 5,
            max_message_length: 500,
        }
    }
}

/// iMessage channel error
#[derive(Debug, thiserror::Error)]
pub enum ImessageError {
    #[error("send error: {0}")]
    Send(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("apple script error: {0}")]
    AppleScript(String),
    #[error("not supported: {0}")]
    NotSupported(String),
}

/// iMessage channel struct
#[derive(Debug)]
pub struct ImessageChannel {
    config: ImessageConfig,
    event_tx: mpsc::Sender<MessageContent>,
}

impl ImessageChannel {
    /// Create a new iMessage channel
    pub fn new(config: ImessageConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        Self { config, event_tx }
    }

    /// Check if iMessage is available on this system
    pub fn is_available() -> bool {
        // Check if we can run AppleScript
        let output = Command::new("osascript")
            .arg("-e")
            .arg("return 1")
            .output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Send a text message via iMessage
    pub async fn send_message(&self, recipient: &str, text: &str) -> Result<(), ImessageError> {
        let sanitized_text = self.sanitize_text(text);

        let script = format!(
            r#"
            tell application "Messages"
                activate
                set targetService to 1st service whose service type = iMessage
                set targetBuddy to buddy "{}" of targetService
                send "{}" to targetBuddy
            end tell
            "#,
            recipient, sanitized_text
        );

        let _ = self.run_applescript(&script).map_err(|e| ImessageError::Send(e.to_string()))?;
        Ok(())
    }

    /// Send a message to the default recipient
    pub async fn send_default(&self, text: &str) -> Result<(), ImessageError> {
        if self.config.default_recipient.is_empty() {
            return Err(ImessageError::Send("No default recipient configured".to_string()));
        }
        self.send_message(&self.config.default_recipient, text).await
    }

    /// Send an SMS message (to phone number)
    pub async fn send_sms(&self, phone: &str, text: &str) -> Result<(), ImessageError> {
        let sanitized_text = self.sanitize_text(text);

        let script = format!(
            r#"
            tell application "Messages"
                activate
                set targetService to 1st service whose service type = SMS
                set targetBuddy to buddy "{}" of targetService
                send "{}" to targetBuddy
            end tell
            "#,
            phone, sanitized_text
        );

        let _ = self.run_applescript(&script).map_err(|e| ImessageError::Send(e.to_string()))?;
        Ok(())
    }

    /// Get recent messages from chat database
    pub async fn get_recent_messages(
        &self,
        _limit: usize,
    ) -> Result<Vec<ImessageChat>, ImessageError> {
        // Query Messages database
        let script = r#"
            tell application "Messages"
                set chatList to every chat
                set resultList to {}
                repeat with aChat in chatList
                    set chatName to name of aChat
                    set messageCount to count of messages of aChat
                    set lastMessage to ""
                    try
                        set lastMessage to word -2 thru -1 of (get id of last message of aChat)
                    end try
                    set end of resultList to {chatName, messageCount, lastMessage}
                end repeat
                return resultList
            end tell
        "#;

        let output = self.run_applescript(script)?;
        self.parse_chat_list(&output)
    }

    /// Run an AppleScript command
    fn run_applescript(&self, script: &str) -> Result<String, ImessageError> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| ImessageError::AppleScript(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ImessageError::AppleScript(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    }

    /// Sanitize text for AppleScript
    fn sanitize_text(&self, text: &str) -> String {
        // Escape quotes and backslashes for AppleScript
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', " ")
    }

    /// Parse chat list from AppleScript output
    fn parse_chat_list(&self, output: &str) -> Result<Vec<ImessageChat>, ImessageError> {
        // Parse AppleScript list format: {chatName, messageCount, lastMessage}
        let mut chats = Vec::new();

        // Simple parsing - look for comma-separated values in braces
        let cleaned = output.trim_matches(|c| c == '{' || c == '}');

        for line in cleaned.lines() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let chat = ImessageChat {
                    id: parts[0].to_string(),
                    participant: parts[0].to_string(),
                    last_message: parts[2].to_string().trim_matches('"').to_string(),
                    timestamp: None,
                };
                chats.push(chat);
            }
        }

        Ok(chats)
    }

    /// Connect to iMessage service
    pub async fn connect(&mut self) -> Result<(), ImessageError> {
        if !Self::is_available() {
            return Err(ImessageError::NotSupported(
                "iMessage is not available on this system".to_string(),
            ));
        }

        info!("iMessage channel connected");
        Ok(())
    }

    /// Disconnect from iMessage service
    pub async fn disconnect(&mut self) -> Result<(), ImessageError> {
        info!("iMessage channel disconnected");
        Ok(())
    }
}

/// Represents an iMessage chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImessageChat {
    /// Chat ID
    pub id: String,
    /// Participant phone/email
    pub participant: String,
    /// Last message content
    pub last_message: String,
    /// Last message timestamp
    pub timestamp: Option<String>,
}

/// Represents an iMessage contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImessageContact {
    /// Contact email or phone
    pub id: String,
    /// Display name
    pub name: String,
    /// Whether this is an iMessage user
    pub is_imessage: bool,
}

/// Get list of iMessage contacts
pub async fn get_contacts() -> Result<Vec<ImessageContact>, ImessageError> {
    // This would query the Contacts app via AppleScript
    let _script = r#"
        tell application "Contacts"
            set contactList to {}
            repeat with aPerson in people
                set contactName to name of aPerson
                set contactEmails to {}
                repeat with anEmail in emails of aPerson
                    set end of contactEmails to value of anEmail
                end repeat
                set end of contactList to {contactName, contactEmails}
            end repeat
            return contactList
        end tell
    "#;

    // For now, return empty list
    Ok(Vec::new())
}

/// Export for module
pub use ImessageChannel as Channel;
pub use ImessageConfig as Config;
pub use ImessageError as Error;
