//! Cron handlers.

use serde_json::{json, Value};
use uuid::Uuid;

use super::super::*;

pub(super) fn handle_cron_status() -> Result<Value, ErrorShape> {
    Ok(json!({
        "enabled": true,
        "jobs": 0
    }))
}

pub(super) fn handle_cron_list() -> Result<Value, ErrorShape> {
    Ok(json!({
        "jobs": []
    }))
}

pub(super) fn handle_cron_add(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let schedule = params
        .and_then(|v| v.get("schedule"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "schedule is required", None))?;
    let command = params
        .and_then(|v| v.get("command"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "command is required", None))?;
    Ok(json!({
        "ok": true,
        "jobId": Uuid::new_v4().to_string(),
        "schedule": schedule,
        "command": command
    }))
}

pub(super) fn handle_cron_update(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params
        .and_then(|v| v.get("jobId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "jobId is required", None))?;
    Ok(json!({
        "ok": true,
        "jobId": job_id,
        "updated": true
    }))
}

pub(super) fn handle_cron_remove(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params
        .and_then(|v| v.get("jobId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "jobId is required", None))?;
    Ok(json!({
        "ok": true,
        "jobId": job_id,
        "removed": true
    }))
}

pub(super) fn handle_cron_run(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params
        .and_then(|v| v.get("jobId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "jobId is required", None))?;
    Ok(json!({
        "ok": true,
        "jobId": job_id,
        "runId": Uuid::new_v4().to_string()
    }))
}

pub(super) fn handle_cron_runs(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let job_id = params.and_then(|v| v.get("jobId")).and_then(|v| v.as_str());
    Ok(json!({
        "runs": [],
        "jobId": job_id
    }))
}
