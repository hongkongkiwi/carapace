//! Cron handlers.
//!
//! This module implements the cron scheduler methods:
//! - cron.status: Get scheduler status
//! - cron.list: List all jobs
//! - cron.add: Add a new job
//! - cron.update: Update an existing job
//! - cron.remove: Remove a job
//! - cron.run: Manually run a job
//! - cron.runs: Get run history

use serde_json::{json, Value};

use super::super::*;

// Re-export types for use by other modules
pub use crate::cron::{
    CronError, CronEvent, CronEventAction, CronIsolation, CronJob, CronJobCreate, CronJobPatch,
    CronJobState, CronJobStatus, CronPayload, CronRemoveResult, CronRunLogEntry, CronRunMode,
    CronRunReason, CronRunResult, CronSchedule, CronScheduler, CronSessionTarget, CronStatus,
    CronStoreFile, CronWakeMode,
};

/// Get the cron scheduler status.
pub(super) fn handle_cron_status(state: &WsServerState) -> Result<Value, ErrorShape> {
    let status = state.cron_scheduler.status();
    Ok(json!(status))
}

/// List all cron jobs.
pub(super) fn handle_cron_list(state: &WsServerState, params: Option<&Value>) -> Result<Value, ErrorShape> {
    let include_disabled = params
        .and_then(|v| v.get("includeDisabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let jobs = state.cron_scheduler.list(include_disabled);
    Ok(json!({ "jobs": jobs }))
}

/// Add a new cron job.
pub(super) fn handle_cron_add(state: &WsServerState, params: Option<&Value>) -> Result<Value, ErrorShape> {
    let params =
        params.ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "params required", None))?;

    // Parse required fields
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "name is required", None))?;

    if name.trim().is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "name cannot be empty",
            None,
        ));
    }

    let schedule = parse_schedule(params.get("schedule"))?;
    let payload = parse_payload(params.get("payload"))?;

    let input = CronJobCreate {
        name: name.to_string(),
        agent_id: params.get("agentId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        description: params.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        enabled: params.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
        delete_after_run: params.get("deleteAfterRun").and_then(|v| v.as_bool()),
        schedule,
        session_target: params.get("sessionTarget")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "isolated" => CronSessionTarget::Isolated,
                _ => CronSessionTarget::Main,
            })
            .unwrap_or_default(),
        wake_mode: params.get("wakeMode")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "next-heartbeat" => CronWakeMode::NextHeartbeat,
                _ => CronWakeMode::Now,
            })
            .unwrap_or_default(),
        payload,
        isolation: None, // TODO: Parse isolation settings
    };

    let job = state.cron_scheduler.add(input);

    // Broadcast event to all connections
    broadcast_event(state, "cron", json!({
        "jobId": job.id,
        "action": "added",
        "nextRunAtMs": job.state.next_run_at_ms
    }));

    Ok(json!({
        "ok": true,
        "job": job
    }))
}

/// Update an existing cron job.
pub(super) fn handle_cron_update(state: &WsServerState, params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params
        .and_then(|v| v.get("jobId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "jobId is required", None))?;

    if job_id.trim().is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "jobId cannot be empty",
            None,
        ));
    }

    // Build patch from params
    let patch = CronJobPatch {
        name: params.and_then(|v| v.get("name")).and_then(|v| v.as_str()).map(|s| s.to_string()),
        agent_id: params.and_then(|v| v.get("agentId")).and_then(|v| v.as_str()).map(|s| s.to_string()),
        description: params.and_then(|v| v.get("description")).and_then(|v| v.as_str()).map(|s| s.to_string()),
        enabled: params.and_then(|v| v.get("enabled")).and_then(|v| v.as_bool()),
        delete_after_run: params.and_then(|v| v.get("deleteAfterRun")).and_then(|v| v.as_bool()),
        schedule: params.and_then(|v| v.get("schedule")).map(|s| serde_json::from_value(s.clone()).ok()).flatten(),
        session_target: params.and_then(|v| v.get("sessionTarget"))
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "isolated" => CronSessionTarget::Isolated,
                _ => CronSessionTarget::Main,
            }),
        wake_mode: params.and_then(|v| v.get("wakeMode"))
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "next-heartbeat" => CronWakeMode::NextHeartbeat,
                _ => CronWakeMode::Now,
            }),
        payload: params.and_then(|v| v.get("payload")).map(|p| serde_json::from_value(p.clone()).ok()).flatten(),
        isolation: None,
    };

    let job = state.cron_scheduler.update(job_id, patch)
        .map_err(|e| error_shape(ERROR_INVALID_REQUEST, &e.to_string(), None))?;

    // Broadcast event
    broadcast_event(state, "cron", json!({
        "jobId": job_id,
        "action": "updated",
        "nextRunAtMs": job.state.next_run_at_ms
    }));

    Ok(json!({
        "ok": true,
        "jobId": job_id,
        "updated": true,
        "job": job
    }))
}

/// Remove a cron job.
pub(super) fn handle_cron_remove(state: &WsServerState, params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params
        .and_then(|v| v.get("jobId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "jobId is required", None))?;

    if job_id.trim().is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "jobId cannot be empty",
            None,
        ));
    }

    let result = state.cron_scheduler.remove(job_id);

    if result.removed {
        // Broadcast event
        broadcast_event(state, "cron", json!({
            "jobId": job_id,
            "action": "removed"
        }));
    }

    Ok(json!(result))
}

/// Manually run a cron job.
pub(super) fn handle_cron_run(state: &WsServerState, params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params
        .and_then(|v| v.get("jobId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "jobId is required", None))?;

    if job_id.trim().is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "jobId cannot be empty",
            None,
        ));
    }

    let mode_str = params.and_then(|v| v.get("mode")).and_then(|v| v.as_str());

    // Validate and parse mode
    let mode = match mode_str {
        Some("force") => Some(CronRunMode::Force),
        Some("due") | None => Some(CronRunMode::Due),
        Some(_) => {
            return Err(error_shape(
                ERROR_INVALID_REQUEST,
                "mode must be 'due' or 'force'",
                None,
            ));
        }
    };

    let result = state.cron_scheduler.run(job_id, mode)
        .map_err(|e| error_shape(ERROR_INVALID_REQUEST, &e.to_string(), None))?;

    if result.ran {
        // Get the job to find next run time
        let job = state.cron_scheduler.get(job_id);

        // Broadcast event
        broadcast_event(state, "cron", json!({
            "jobId": job_id,
            "action": "finished",
            "status": "ok",
            "nextRunAtMs": job.as_ref().and_then(|j| j.state.next_run_at_ms)
        }));
    }

    Ok(json!(result))
}

/// Get run history for jobs.
pub(super) fn handle_cron_runs(state: &WsServerState, params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params.and_then(|v| v.get("jobId")).and_then(|v| v.as_str());
    let limit = params
        .and_then(|v| v.get("limit"))
        .map(|v| v.as_u64().unwrap_or(200) as usize);

    let runs = state.cron_scheduler.runs(job_id, limit);

    Ok(json!({
        "runs": runs,
        "jobId": job_id
    }))
}

/// Parse a schedule from JSON.
fn parse_schedule(value: Option<&Value>) -> Result<CronSchedule, ErrorShape> {
    let value =
        value.ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "schedule is required", None))?;

    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "schedule.kind is required", None))?;

    match kind {
        "at" => {
            let at_ms = value.get("atMs").and_then(|v| v.as_u64()).ok_or_else(|| {
                error_shape(
                    ERROR_INVALID_REQUEST,
                    "schedule.atMs is required for 'at' schedule",
                    None,
                )
            })?;
            Ok(CronSchedule::At { at_ms })
        }
        "every" => {
            let every_ms = value
                .get("everyMs")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    error_shape(
                        ERROR_INVALID_REQUEST,
                        "schedule.everyMs is required for 'every' schedule",
                        None,
                    )
                })?;
            // Validate everyMs >= 1 to prevent divide-by-zero in compute_next_run
            if every_ms < 1 {
                return Err(error_shape(
                    ERROR_INVALID_REQUEST,
                    "schedule.everyMs must be at least 1",
                    None,
                ));
            }
            let anchor_ms = value.get("anchorMs").and_then(|v| v.as_u64());
            Ok(CronSchedule::Every {
                every_ms,
                anchor_ms,
            })
        }
        "cron" => {
            let expr = value.get("expr").and_then(|v| v.as_str()).ok_or_else(|| {
                error_shape(
                    ERROR_INVALID_REQUEST,
                    "schedule.expr is required for 'cron' schedule",
                    None,
                )
            })?;
            let tz = value
                .get("tz")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(CronSchedule::Cron {
                expr: expr.to_string(),
                tz,
            })
        }
        _ => Err(error_shape(
            ERROR_INVALID_REQUEST,
            "schedule.kind must be 'at', 'every', or 'cron'",
            None,
        )),
    }
}

/// Parse a payload from JSON.
fn parse_payload(value: Option<&Value>) -> Result<CronPayload, ErrorShape> {
    let value =
        value.ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "payload is required", None))?;

    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "payload.kind is required", None))?;

    match kind {
        "systemEvent" => {
            let text = value.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                error_shape(
                    ERROR_INVALID_REQUEST,
                    "payload.text is required for 'systemEvent' payload",
                    None,
                )
            })?;
            Ok(CronPayload::SystemEvent {
                text: text.to_string(),
            })
        }
        "agentTurn" => {
            let message = value
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    error_shape(
                        ERROR_INVALID_REQUEST,
                        "payload.message is required for 'agentTurn' payload",
                        None,
                    )
                })?;
            Ok(CronPayload::AgentTurn {
                message: message.to_string(),
                model: value
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                thinking: value
                    .get("thinking")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                timeout_seconds: value
                    .get("timeoutSeconds")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
                allow_unsafe_external_content: value
                    .get("allowUnsafeExternalContent")
                    .and_then(|v| v.as_bool()),
                deliver: value.get("deliver").and_then(|v| v.as_bool()),
                channel: value
                    .get("channel")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                to: value
                    .get("to")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                best_effort_deliver: value.get("bestEffortDeliver").and_then(|v| v.as_bool()),
            })
        }
        _ => Err(error_shape(
            ERROR_INVALID_REQUEST,
            "payload.kind must be 'systemEvent' or 'agentTurn'",
            None,
        )),
    }
}

/// Parse isolation settings from JSON.
#[allow(dead_code)]
fn parse_isolation(value: &Value) -> Option<CronIsolation> {
    if !value.is_object() {
        return None;
    }

    Some(CronIsolation {
        post_to_main_prefix: value
            .get("postToMainPrefix")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        post_to_main_mode: value
            .get("postToMainMode")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        post_to_main_max_chars: value
            .get("postToMainMaxChars")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
    })
}

/// Compute the next run time for a schedule.
fn compute_next_run(schedule: &CronSchedule, now: u64) -> Option<u64> {
    match schedule {
        CronSchedule::At { at_ms } => {
            if *at_ms > now {
                Some(*at_ms)
            } else {
                None // Already passed
            }
        }
        CronSchedule::Every {
            every_ms,
            anchor_ms,
        } => {
            // Guard against divide-by-zero (should be validated at parse time)
            if *every_ms == 0 {
                return None;
            }
            let anchor = anchor_ms.unwrap_or(now);
            if now < anchor {
                Some(anchor)
            } else {
                let elapsed = now - anchor;
                let periods = elapsed / every_ms;
                Some(anchor + (periods + 1) * every_ms)
            }
        }
        CronSchedule::Cron { expr: _, tz: _ } => {
            // Cron expression parsing would require a cron library
            // For now, return a default next minute
            let next_minute = (now / 60_000 + 1) * 60_000;
            Some(next_minute)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Handler integration tests require a full WsServerState.
    // The cron module tests in src/cron/mod.rs cover the scheduler logic.
    // These tests focus on parsing and utility functions.

    #[test]
    fn test_parse_schedule_at() {
        let value = json!({ "kind": "at", "atMs": 1234567890 });
        let schedule = parse_schedule(Some(&value)).unwrap();
        match schedule {
            CronSchedule::At { at_ms } => assert_eq!(at_ms, 1234567890),
            _ => panic!("Expected At schedule"),
        }
    }

    #[test]
    fn test_parse_schedule_every() {
        let value = json!({ "kind": "every", "everyMs": 60000, "anchorMs": 1000 });
        let schedule = parse_schedule(Some(&value)).unwrap();
        match schedule {
            CronSchedule::Every {
                every_ms,
                anchor_ms,
            } => {
                assert_eq!(every_ms, 60000);
                assert_eq!(anchor_ms, Some(1000));
            }
            _ => panic!("Expected Every schedule"),
        }
    }

    #[test]
    fn test_parse_schedule_cron() {
        let value = json!({ "kind": "cron", "expr": "0 9 * * *", "tz": "America/New_York" });
        let schedule = parse_schedule(Some(&value)).unwrap();
        match schedule {
            CronSchedule::Cron { expr, tz } => {
                assert_eq!(expr, "0 9 * * *");
                assert_eq!(tz, Some("America/New_York".to_string()));
            }
            _ => panic!("Expected Cron schedule"),
        }
    }

    #[test]
    fn test_parse_schedule_invalid() {
        let value = json!({ "kind": "invalid" });
        let result = parse_schedule(Some(&value));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_schedule_every_rejects_zero() {
        // everyMs=0 would cause divide-by-zero in compute_next_run
        let value = json!({ "kind": "every", "everyMs": 0 });
        let result = parse_schedule(Some(&value));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.message, "schedule.everyMs must be at least 1");
    }

    #[test]
    fn test_compute_next_run_every_zero_returns_none() {
        // Defensive guard: even if everyMs=0 somehow gets through validation,
        // compute_next_run should return None instead of panicking
        let schedule = CronSchedule::Every {
            every_ms: 0,
            anchor_ms: None,
        };
        assert_eq!(compute_next_run(&schedule, 1000), None);
    }

    #[test]
    fn test_parse_payload_system_event() {
        let value = json!({ "kind": "systemEvent", "text": "Hello" });
        let payload = parse_payload(Some(&value)).unwrap();
        match payload {
            CronPayload::SystemEvent { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected SystemEvent payload"),
        }
    }

    #[test]
    fn test_parse_payload_agent_turn() {
        let value = json!({
            "kind": "agentTurn",
            "message": "Do something",
            "model": "claude-3-opus",
            "deliver": true
        });
        let payload = parse_payload(Some(&value)).unwrap();
        match payload {
            CronPayload::AgentTurn {
                message,
                model,
                deliver,
                ..
            } => {
                assert_eq!(message, "Do something");
                assert_eq!(model, Some("claude-3-opus".to_string()));
                assert_eq!(deliver, Some(true));
            }
            _ => panic!("Expected AgentTurn payload"),
        }
    }

    #[test]
    fn test_parse_isolation() {
        let value = json!({
            "postToMainPrefix": "[Cron]",
            "postToMainMode": "summary",
            "postToMainMaxChars": 5000
        });
        let isolation = parse_isolation(&value).unwrap();
        assert_eq!(isolation.post_to_main_prefix, Some("[Cron]".to_string()));
        assert_eq!(isolation.post_to_main_mode, Some("summary".to_string()));
        assert_eq!(isolation.post_to_main_max_chars, Some(5000));
    }

    #[test]
    fn test_compute_next_run_at() {
        let now = 1000;

        // Future time
        let schedule = CronSchedule::At { at_ms: 2000 };
        assert_eq!(compute_next_run(&schedule, now), Some(2000));

        // Past time
        let schedule = CronSchedule::At { at_ms: 500 };
        assert_eq!(compute_next_run(&schedule, now), None);
    }

    #[test]
    fn test_compute_next_run_every() {
        let now = 1000;

        // Simple interval
        let schedule = CronSchedule::Every {
            every_ms: 100,
            anchor_ms: None,
        };
        let next = compute_next_run(&schedule, now).unwrap();
        assert!(next > now);
        assert!(next <= now + 100);

        // With anchor
        let schedule = CronSchedule::Every {
            every_ms: 100,
            anchor_ms: Some(950),
        };
        let next = compute_next_run(&schedule, now).unwrap();
        assert_eq!(next, 1050); // 950 + 100
    }
}
