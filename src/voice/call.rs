//! Call State Management
//!
//! Manages active voice calls and their state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Call direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallDirection {
    /// Incoming call
    Inbound,
    /// Outgoing call
    Outbound,
}

/// Call status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallStatus {
    /// Call is queued
    Queued,
    /// Call is ringing
    Ringing,
    /// Call is in progress
    InProgress,
    /// Call is completed
    Completed,
    /// Call failed
    Failed,
    /// Call was busy
    Busy,
    /// No answer
    NoAnswer,
    /// Call was cancelled
    Cancelled,
}

/// Voice call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCall {
    /// Unique call ID
    pub id: String,
    /// Twilio call SID
    pub twilio_sid: Option<String>,
    /// Call direction
    pub direction: CallDirection,
    /// Call status
    pub status: CallStatus,
    /// Caller phone number
    pub from: String,
    /// Recipient phone number
    pub to: String,
    /// When the call started
    pub started_at: DateTime<Utc>,
    /// When the call ended (if completed)
    pub ended_at: Option<DateTime<Utc>>,
    /// Call duration in seconds
    pub duration: Option<u32>,
    /// Recording URL (if recorded)
    pub recording_url: Option<String>,
    /// Current conversation transcript
    pub transcript: Vec<TranscriptEntry>,
    /// Whether barge-in is enabled
    pub barge_in_enabled: bool,
    /// Custom parameters
    pub parameters: HashMap<String, String>,
}

/// Transcript entry for a call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    /// Who spoke (caller, callee, or system)
    pub speaker: String,
    /// What was said
    pub text: String,
    /// When it was said
    pub timestamp: DateTime<Utc>,
}

/// Call manager for tracking active calls
#[derive(Debug, Clone)]
pub struct CallManager {
    calls: Arc<RwLock<HashMap<String, VoiceCall>>>,
}

impl CallManager {
    /// Create a new call manager
    pub fn new() -> Self {
        Self {
            calls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new call
    pub async fn create_call(
        &self,
        direction: CallDirection,
        from: String,
        to: String,
        barge_in_enabled: bool,
    ) -> VoiceCall {
        let call = VoiceCall {
            id: Uuid::new_v4().to_string(),
            twilio_sid: None,
            direction,
            status: CallStatus::Queued,
            from,
            to,
            started_at: Utc::now(),
            ended_at: None,
            duration: None,
            recording_url: None,
            transcript: Vec::new(),
            barge_in_enabled,
            parameters: HashMap::new(),
        };

        let mut calls = self.calls.write().await;
        calls.insert(call.id.clone(), call.clone());

        call
    }

    /// Get a call by ID
    pub async fn get_call(&self, id: &str) -> Option<VoiceCall> {
        let calls = self.calls.read().await;
        calls.get(id).cloned()
    }

    /// Get a call by Twilio SID
    pub async fn get_call_by_twilio_sid(&self, sid: &str) -> Option<VoiceCall> {
        let calls = self.calls.read().await;
        calls
            .values()
            .find(|c| c.twilio_sid.as_deref() == Some(sid))
            .cloned()
    }

    /// Update call status
    pub async fn update_status(&self, id: &str, status: CallStatus) -> Option<()> {
        let mut calls = self.calls.write().await;
        if let Some(call) = calls.get_mut(id) {
            call.status = status;
            if status == CallStatus::Completed {
                call.ended_at = Some(Utc::now());
                if let Some(started) = call.started_at.timestamp().checked_sub(0) {
                    let ended = call.ended_at.unwrap().timestamp();
                    call.duration = Some((ended - started) as u32);
                }
            }
            Some(())
        } else {
            None
        }
    }

    /// Set Twilio SID for a call
    pub async fn set_twilio_sid(&self, id: &str, sid: String) -> Option<()> {
        let mut calls = self.calls.write().await;
        if let Some(call) = calls.get_mut(id) {
            call.twilio_sid = Some(sid);
            Some(())
        } else {
            None
        }
    }

    /// Add transcript entry
    pub async fn add_transcript_entry(
        &self,
        id: &str,
        speaker: String,
        text: String,
    ) -> Option<()> {
        let mut calls = self.calls.write().await;
        if let Some(call) = calls.get_mut(id) {
            call.transcript.push(TranscriptEntry {
                speaker,
                text,
                timestamp: Utc::now(),
            });
            Some(())
        } else {
            None
        }
    }

    /// Set recording URL
    pub async fn set_recording(&self, id: &str, url: String) -> Option<()> {
        let mut calls = self.calls.write().await;
        if let Some(call) = calls.get_mut(id) {
            call.recording_url = Some(url);
            Some(())
        } else {
            None
        }
    }

    /// List all active calls
    pub async fn list_active_calls(&self) -> Vec<VoiceCall> {
        let calls = self.calls.read().await;
        calls
            .values()
            .filter(|c| {
                matches!(
                    c.status,
                    CallStatus::Queued | CallStatus::Ringing | CallStatus::InProgress
                )
            })
            .cloned()
            .collect()
    }

    /// List all calls
    pub async fn list_all_calls(&self) -> Vec<VoiceCall> {
        let calls = self.calls.read().await;
        calls.values().cloned().collect()
    }

    /// End a call
    pub async fn end_call(&self, id: &str) -> Option<()> {
        self.update_status(id, CallStatus::Completed).await
    }

    /// Delete a call from tracking
    pub async fn delete_call(&self, id: &str) -> Option<VoiceCall> {
        let mut calls = self.calls.write().await;
        calls.remove(id)
    }
}

impl Default for CallManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_manager_create_call() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert_eq!(call.direction, CallDirection::Outbound);
        assert_eq!(call.from, "+1234567890");
        assert_eq!(call.to, "+0987654321");
        assert_eq!(call.status, CallStatus::Queued);
        assert!(call.barge_in_enabled);
        assert!(call.twilio_sid.is_none());
    }

    #[tokio::test]
    async fn test_call_manager_get_call() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Inbound,
            "+1111111111".to_string(),
            "+2222222222".to_string(),
            false,
        ).await;

        let retrieved = manager.get_call(&call.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, call.id);

        assert!(manager.get_call("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_call_manager_update_status() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert!(manager.update_status(&call.id, CallStatus::Ringing).await.is_some());

        let updated = manager.get_call(&call.id).await.unwrap();
        assert_eq!(updated.status, CallStatus::Ringing);

        assert!(manager.update_status("nonexistent", CallStatus::Completed).await.is_none());
    }

    #[tokio::test]
    async fn test_call_manager_set_twilio_sid() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert!(manager.set_twilio_sid(&call.id, "CA1234567890".to_string()).await.is_some());

        let updated = manager.get_call(&call.id).await.unwrap();
        assert_eq!(updated.twilio_sid, Some("CA1234567890".to_string()));
    }

    #[tokio::test]
    async fn test_call_manager_add_transcript_entry() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert!(manager.add_transcript_entry(&call.id, "caller".to_string(), "Hello".to_string()).await.is_some());
        assert!(manager.add_transcript_entry(&call.id, "system".to_string(), "Hi there".to_string()).await.is_some());

        let updated = manager.get_call(&call.id).await.unwrap();
        assert_eq!(updated.transcript.len(), 2);
        assert_eq!(updated.transcript[0].speaker, "caller");
        assert_eq!(updated.transcript[0].text, "Hello");
    }

    #[tokio::test]
    async fn test_call_manager_set_recording() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert!(manager.set_recording(&call.id, "https://example.com/recording.mp3".to_string()).await.is_some());

        let updated = manager.get_call(&call.id).await.unwrap();
        assert_eq!(updated.recording_url, Some("https://example.com/recording.mp3".to_string()));
    }

    #[tokio::test]
    async fn test_call_manager_list_active_calls() {
        let manager = CallManager::new();

        let call1 = manager.create_call(CallDirection::Outbound, "+1".to_string(), "+2".to_string(), true).await;
        let call2 = manager.create_call(CallDirection::Inbound, "+3".to_string(), "+4".to_string(), false).await;
        let _call3 = manager.create_call(CallDirection::Outbound, "+5".to_string(), "+6".to_string(), true).await;

        manager.update_status(&call1.id, CallStatus::InProgress).await;
        manager.update_status(&call2.id, CallStatus::Completed).await;

        let active = manager.list_active_calls().await;
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_call_manager_list_all_calls() {
        let manager = CallManager::new();

        let _call1 = manager.create_call(CallDirection::Outbound, "+1".to_string(), "+2".to_string(), true).await;
        let _call2 = manager.create_call(CallDirection::Inbound, "+3".to_string(), "+4".to_string(), false).await;

        let all = manager.list_all_calls().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_call_manager_end_call() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert!(manager.end_call(&call.id).await.is_some());

        let ended = manager.get_call(&call.id).await.unwrap();
        assert_eq!(ended.status, CallStatus::Completed);
        assert!(ended.ended_at.is_some());
        assert!(ended.duration.is_some());
    }

    #[tokio::test]
    async fn test_call_manager_delete_call() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        assert!(manager.delete_call(&call.id).await.is_some());
        assert!(manager.get_call(&call.id).await.is_none());
        assert!(manager.delete_call(&call.id).await.is_none());
    }

    #[tokio::test]
    async fn test_call_manager_get_call_by_twilio_sid() {
        let manager = CallManager::new();
        let call = manager.create_call(
            CallDirection::Outbound,
            "+1234567890".to_string(),
            "+0987654321".to_string(),
            true,
        ).await;

        manager.set_twilio_sid(&call.id, "CA12345".to_string()).await;

        let found = manager.get_call_by_twilio_sid("CA12345").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, call.id);

        assert!(manager.get_call_by_twilio_sid("nonexistent").await.is_none());
    }
}
