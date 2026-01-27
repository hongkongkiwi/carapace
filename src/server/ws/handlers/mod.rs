//! WebSocket handlers

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use uuid::Uuid;

use super::*;

pub(super) fn handle_health() -> Value {
    json!({
        "ts": now_ms(),
        "status": "healthy"
    })
}

pub(super) fn handle_status(state: &WsServerState) -> Value {
    let sessions = state
        .session_store
        .list_sessions(sessions::SessionFilter::new())
        .unwrap_or_default();
    let recent_sessions = sessions
        .iter()
        .take(10)
        .map(|session| {
            json!({
                "sessionId": session.id,
                "key": session.session_key,
                "updatedAt": session.updated_at
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ts": now_ms(),
        "status": "ok",
        "uptimeMs": state.start_time.elapsed().as_millis() as u64,
        "version": env!("CARGO_PKG_VERSION"),
        "runtime": {
            "name": "rusty-clawd",
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH
        },
        "channels": {
            "total": state.channel_registry.len(),
            "connected": state
                .channel_registry
                .count_by_status(channels::ChannelStatus::Connected)
        },
        "sessions": {
            "count": sessions.len(),
            "recent": recent_sessions
        }
    })
}

#[derive(Debug, Serialize)]
struct ConfigIssue {
    path: String,
    message: String,
}

#[derive(Debug)]
struct ConfigSnapshot {
    path: String,
    exists: bool,
    raw: Option<String>,
    parsed: Value,
    valid: bool,
    config: Value,
    hash: Option<String>,
    issues: Vec<ConfigIssue>,
}

fn map_validation_issues(issues: Vec<config::ValidationIssue>) -> Vec<ConfigIssue> {
    issues
        .into_iter()
        .map(|issue| ConfigIssue {
            path: issue.path,
            message: issue.message,
        })
        .collect()
}

fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{:x}", digest)
}

fn read_config_snapshot() -> ConfigSnapshot {
    let path = config::get_config_path();
    let path_str = path.display().to_string();
    if !path.exists() {
        return ConfigSnapshot {
            path: path_str,
            exists: false,
            raw: None,
            parsed: Value::Object(serde_json::Map::new()),
            valid: true,
            config: Value::Object(serde_json::Map::new()),
            hash: None,
            issues: Vec::new(),
        };
    }

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) => {
            return ConfigSnapshot {
                path: path_str,
                exists: true,
                raw: None,
                parsed: Value::Object(serde_json::Map::new()),
                valid: false,
                config: Value::Object(serde_json::Map::new()),
                hash: None,
                issues: vec![ConfigIssue {
                    path: "".to_string(),
                    message: format!("read failed: {}", err),
                }],
            }
        }
    };

    let hash = Some(sha256_hex(&raw));
    let parsed = json5::from_str::<Value>(&raw).unwrap_or(Value::Null);

    let (config_value, mut issues, valid) = match config::load_config_uncached(&path) {
        Ok(cfg) => {
            let issues = map_validation_issues(config::validate_config(&cfg));
            let valid = issues.is_empty();
            (cfg, issues, valid)
        }
        Err(err) => {
            let mut issues = Vec::new();
            issues.push(ConfigIssue {
                path: "".to_string(),
                message: err.to_string(),
            });
            (parsed.clone(), issues, false)
        }
    };

    if !valid && issues.is_empty() {
        issues.push(ConfigIssue {
            path: "".to_string(),
            message: "invalid config".to_string(),
        });
    }

    ConfigSnapshot {
        path: path_str,
        exists: true,
        raw: Some(raw),
        parsed,
        valid,
        config: config_value,
        hash,
        issues,
    }
}

fn require_config_base_hash(
    params: Option<&Value>,
    snapshot: &ConfigSnapshot,
) -> Result<(), ErrorShape> {
    if !snapshot.exists {
        return Ok(());
    }
    let base_hash = params
        .and_then(|v| v.get("baseHash"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let expected = snapshot.hash.as_deref();
    if expected.is_none() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "config base hash unavailable; re-run config.get and retry",
            None,
        ));
    }
    let Some(base_hash) = base_hash else {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "config base hash required; re-run config.get and retry",
            None,
        ));
    };
    if Some(base_hash) != expected {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "config changed since last load; re-run config.get and retry",
            None,
        ));
    }
    Ok(())
}

fn write_config_file(path: &PathBuf, config_value: &Value) -> Result<(), ErrorShape> {
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return Err(error_shape(
                ERROR_UNAVAILABLE,
                &format!("failed to create config dir: {}", err),
                None,
            ));
        }
    }

    let content = serde_json::to_string_pretty(config_value)
        .map_err(|err| error_shape(ERROR_UNAVAILABLE, &err.to_string(), None))?;
    let tmp_path = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp_path).map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("failed to write config: {}", err),
                None,
            )
        })?;
        file.write_all(content.as_bytes()).map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("failed to write config: {}", err),
                None,
            )
        })?;
        file.write_all(b"\n").map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("failed to write config: {}", err),
                None,
            )
        })?;
    }
    if let Err(err) = fs::rename(&tmp_path, path) {
        return Err(error_shape(
            ERROR_UNAVAILABLE,
            &format!("failed to replace config: {}", err),
            None,
        ));
    }

    config::clear_cache();
    Ok(())
}

fn merge_patch(base: Value, patch: Value) -> Value {
    match (base, patch) {
        (_, Value::Null) => Value::Null,
        (Value::Object(mut base_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                if patch_value.is_null() {
                    base_map.remove(&key);
                } else {
                    let base_value = base_map.remove(&key).unwrap_or(Value::Null);
                    let merged = merge_patch(base_value, patch_value);
                    base_map.insert(key, merged);
                }
            }
            Value::Object(base_map)
        }
        (_, patch_value) => patch_value,
    }
}

/// Methods exclusively for the `node` role
///
/// These methods can ONLY be called by node connections.
/// Non-node roles are explicitly blocked from calling these.
/// This matches Node.js gateway behavior in src/gateway/server-methods.ts.
pub(super) const NODE_ONLY_METHODS: [&str; 3] = ["node.invoke.result", "node.event", "skills.bins"];

/// Methods that require operator.admin scope for operator role
///
/// Per Node.js gateway: config.*, wizard.*, update.*, skills.install/update,
/// channels.logout, sessions.*, and cron.* require operator.admin for operators.
const OPERATOR_ADMIN_REQUIRED_METHODS: [&str; 20] = [
    "config.get",
    "config.set",
    "config.apply",
    "config.patch",
    "config.schema",
    "sessions.patch",
    "sessions.reset",
    "sessions.delete",
    "sessions.compact",
    "wizard.start",
    "wizard.next",
    "wizard.cancel",
    "update.run",
    "skills.install",
    "skills.update",
    "cron.add",
    "cron.update",
    "cron.remove",
    "cron.run",
    "channels.logout",
];

/// Method authorization levels
///
/// Methods are categorized by the minimum role required to call them:
/// - read: health, status, list operations (any authenticated connection)
/// - write: session modifications, agent invocations
/// - admin: device pairing, exec approvals, sensitive operations
///
/// Note: For operators, additional scope checks are applied separately.
fn get_method_required_role(method: &str) -> &'static str {
    match method {
        // Read-only operations (any authenticated role)
        "health"
        | "status"
        | "last-heartbeat"
        | "config.get"
        | "config.schema"
        | "sessions.list"
        | "sessions.preview"
        | "channels.status"
        | "agent.identity.get"
        | "chat.history"
        | "tts.status"
        | "tts.providers"
        | "voicewake.get"
        | "wizard.status"
        | "models.list"
        | "agents.list"
        | "skills.status"
        | "cron.status"
        | "cron.list"
        | "cron.runs"
        | "node.list"
        | "node.describe"
        | "node.pair.list"
        | "device.pair.list"
        | "exec.approvals.get"
        | "exec.approvals.node.get"
        | "usage.status"
        | "usage.cost"
        | "logs.tail" => "read",

        // Write operations (requires write or admin role)
        "config.set" | "config.apply" | "config.patch" | "sessions.patch" | "sessions.reset"
        | "sessions.delete" | "sessions.compact" | "channels.logout" | "agent" | "agent.wait"
        | "chat.send" | "chat.abort" | "tts.enable" | "tts.disable" | "tts.convert"
        | "tts.setProvider" | "voicewake.set" | "wizard.start" | "wizard.next"
        | "wizard.cancel" | "talk.mode" | "skills.install" | "skills.update" | "update.run"
        | "cron.add" | "cron.update" | "cron.remove" | "cron.run" | "node.invoke"
        | "set-heartbeats" | "wake" | "send" | "system-presence" | "system-event" => "write",

        // Admin operations (requires admin role, or operator with specific scopes)
        "device.pair.approve"
        | "device.pair.reject"
        | "device.token.rotate"
        | "device.token.revoke"
        | "node.pair.request"
        | "node.pair.approve"
        | "node.pair.reject"
        | "node.pair.verify"
        | "node.rename"
        | "exec.approvals.set"
        | "exec.approvals.node.set"
        | "exec.approval.request"
        | "exec.approval.resolve" => "admin",

        // Unknown methods default to admin (fail secure)
        _ => "admin",
    }
}

/// Get the required scope for admin-level methods (for operator role)
///
/// These are methods that require a specific scope beyond operator.admin.
/// Operators can call these with the specific scope without needing full operator.admin.
pub(super) fn get_method_specific_scope(method: &str) -> Option<&'static str> {
    match method {
        // Pairing operations require operator.pairing scope
        "device.pair.approve"
        | "device.pair.reject"
        | "device.token.rotate"
        | "device.token.revoke"
        | "node.pair.request"
        | "node.pair.approve"
        | "node.pair.reject"
        | "node.pair.verify"
        | "node.rename" => Some("operator.pairing"),

        // Exec approval operations require operator.approvals scope
        "exec.approvals.set"
        | "exec.approvals.node.set"
        | "exec.approval.request"
        | "exec.approval.resolve" => Some("operator.approvals"),

        // All other methods don't have a specific scope override
        _ => None,
    }
}

/// Check if a role satisfies the required role level
///
/// Role hierarchy: admin > operator > write > read
pub(super) fn role_satisfies(has_role: &str, required_role: &str) -> bool {
    match required_role {
        "read" => true, // Any role satisfies read
        "write" => matches!(has_role, "write" | "admin" | "operator"),
        "admin" => has_role == "admin",
        _ => false,
    }
}

/// Check if scopes satisfy the required scope
pub(super) fn scope_satisfies(scopes: &[String], required_scope: &str) -> bool {
    for scope in scopes {
        // Exact match
        if scope == required_scope {
            return true;
        }

        // Wildcard: operator.* covers all operator scopes
        if scope == "operator.*" && required_scope.starts_with("operator.") {
            return true;
        }

        // operator.admin covers all operator scopes
        if scope == "operator.admin" && required_scope.starts_with("operator.") {
            return true;
        }

        // operator.write covers operator.read
        if scope == "operator.write" && required_scope == "operator.read" {
            return true;
        }
    }

    false
}

/// Check if the connection is authorized to call a method
///
/// Authorization flow (matching Node.js gateway):
/// 1. Block node-only methods for non-node roles
/// 2. Node role: only allow node-only methods
/// 3. Admin role: full access
/// 4. Operator role: check scopes per method requirements
/// 5. Other roles: check role hierarchy
pub(super) fn check_method_authorization(
    method: &str,
    conn: &ConnectionContext,
) -> Result<(), ErrorShape> {
    // Block node-only methods for non-node roles
    if NODE_ONLY_METHODS.contains(&method) && conn.role != "node" {
        return Err(error_shape(
            ERROR_FORBIDDEN,
            &format!("method '{}' is only allowed for node role", method),
            Some(json!({
                "method": method,
                "connection_role": conn.role,
                "required_role": "node"
            })),
        ));
    }

    // Node role: only allow node-only methods
    if conn.role == "node" {
        if NODE_ONLY_METHODS.contains(&method) {
            return Ok(());
        }
        return Err(error_shape(
            ERROR_FORBIDDEN,
            &format!(
                "method '{}' not allowed for node role (allowed: {:?})",
                method, NODE_ONLY_METHODS
            ),
            Some(json!({
                "method": method,
                "connection_role": "node",
                "allowed_methods": NODE_ONLY_METHODS
            })),
        ));
    }

    // Admin role: full access
    if conn.role == "admin" {
        return Ok(());
    }

    let required_role = get_method_required_role(method);

    // Operator role: check scopes per Node.js gateway model
    if conn.role == "operator" {
        return check_operator_authorization(method, required_role, conn);
    }

    // Other roles: check role hierarchy
    if !role_satisfies(&conn.role, required_role) {
        return Err(error_shape(
            ERROR_FORBIDDEN,
            &format!(
                "method '{}' requires role '{}', connection has role '{}'",
                method, required_role, conn.role
            ),
            Some(json!({
                "method": method,
                "required_role": required_role,
                "connection_role": conn.role
            })),
        ));
    }

    Ok(())
}

/// Check operator authorization with scope-based access control
///
/// Per Node.js gateway:
/// - operator.admin required for: config.*, wizard.*, update.*, skills.install/update, channels.logout
/// - operator.pairing allows: device pairing methods (without needing operator.admin)
/// - operator.approvals allows: exec approval methods (without needing operator.admin)
/// - operator.write required for write-level methods
/// - operator.read required for read-level methods
fn check_operator_authorization(
    method: &str,
    required_role: &str,
    conn: &ConnectionContext,
) -> Result<(), ErrorShape> {
    // Check if method requires operator.admin (config.*, wizard.*, etc.)
    if OPERATOR_ADMIN_REQUIRED_METHODS.contains(&method) {
        if !scope_satisfies(&conn.scopes, "operator.admin") {
            return Err(error_shape(
                ERROR_FORBIDDEN,
                &format!("method '{}' requires 'operator.admin' scope", method),
                Some(json!({
                    "method": method,
                    "required_scope": "operator.admin",
                    "connection_scopes": conn.scopes
                })),
            ));
        }
        return Ok(());
    }

    // Check if method has a specific scope that can bypass operator.admin
    // E.g., operator.pairing allows device.pair.* without full admin
    if let Some(specific_scope) = get_method_specific_scope(method) {
        if scope_satisfies(&conn.scopes, specific_scope) {
            return Ok(());
        }
        // Also allow if they have operator.admin
        if scope_satisfies(&conn.scopes, "operator.admin") {
            return Ok(());
        }
        return Err(error_shape(
            ERROR_FORBIDDEN,
            &format!(
                "method '{}' requires '{}' or 'operator.admin' scope",
                method, specific_scope
            ),
            Some(json!({
                "method": method,
                "required_scope": specific_scope,
                "connection_scopes": conn.scopes
            })),
        ));
    }

    // Check scope based on required role level
    match required_role {
        "write" => {
            if !scope_satisfies(&conn.scopes, "operator.write") {
                return Err(error_shape(
                    ERROR_FORBIDDEN,
                    &format!("method '{}' requires 'operator.write' scope", method),
                    Some(json!({
                        "method": method,
                        "required_scope": "operator.write",
                        "connection_scopes": conn.scopes
                    })),
                ));
            }
        }
        "read" => {
            if !scope_satisfies(&conn.scopes, "operator.read") {
                return Err(error_shape(
                    ERROR_FORBIDDEN,
                    &format!("method '{}' requires 'operator.read' scope", method),
                    Some(json!({
                        "method": method,
                        "required_scope": "operator.read",
                        "connection_scopes": conn.scopes
                    })),
                ));
            }
        }
        "admin" => {
            // Admin methods that don't have specific scopes require operator.admin
            if !scope_satisfies(&conn.scopes, "operator.admin") {
                return Err(error_shape(
                    ERROR_FORBIDDEN,
                    &format!("method '{}' requires 'operator.admin' scope", method),
                    Some(json!({
                        "method": method,
                        "required_scope": "operator.admin",
                        "connection_scopes": conn.scopes
                    })),
                ));
            }
        }
        _ => {
            // Unknown role level, fail secure
            return Err(error_shape(
                ERROR_FORBIDDEN,
                &format!(
                    "method '{}' has unknown required role '{}'",
                    method, required_role
                ),
                Some(json!({
                    "method": method,
                    "required_role": required_role
                })),
            ));
        }
    }

    Ok(())
}

pub(super) async fn dispatch_method(
    method: &str,
    params: Option<&Value>,
    state: &WsServerState,
    conn: &ConnectionContext,
) -> Result<Value, ErrorShape> {
    // Check authorization before dispatching
    check_method_authorization(method, conn)?;

    match method {
        // Health/status
        "health" => Ok(handle_health()),
        "status" => Ok(handle_status(state)),

        // Config
        "config.get" => handle_config_get(params),
        "config.set" => handle_config_set(params),
        "config.apply" => handle_config_apply(params),
        "config.patch" => handle_config_patch(params),
        "config.schema" => handle_config_schema(),

        // Sessions
        "sessions.list" => handle_sessions_list(state, params),
        "sessions.preview" => handle_sessions_preview(state, params),
        "sessions.patch" => handle_sessions_patch(state, params),
        "sessions.reset" => handle_sessions_reset(state, params),
        "sessions.delete" => handle_sessions_delete(state, params),
        "sessions.compact" => handle_sessions_compact(state, params),

        // Channels
        "channels.status" => handle_channels_status(state),
        "channels.logout" => handle_channels_logout(params, state),

        // Agent
        "agent" => handle_agent(params, state, conn),
        "agent.identity.get" => handle_agent_identity_get(state),
        "agent.wait" => handle_agent_wait(params),

        // Chat
        "chat.history" => handle_chat_history(state, params),
        "chat.send" => handle_chat_send(state, params, conn),
        "chat.abort" => handle_chat_abort(state, params),

        // TTS
        "tts.status" => handle_tts_status(),
        "tts.providers" => handle_tts_providers(),
        "tts.enable" => handle_tts_enable(),
        "tts.disable" => handle_tts_disable(),
        "tts.convert" => handle_tts_convert(params),
        "tts.setProvider" => handle_tts_set_provider(params),

        // Voice wake
        "voicewake.get" => handle_voicewake_get(),
        "voicewake.set" => handle_voicewake_set(params),

        // Wizard
        "wizard.start" => handle_wizard_start(params),
        "wizard.next" => handle_wizard_next(params),
        "wizard.cancel" => handle_wizard_cancel(),
        "wizard.status" => handle_wizard_status(),

        // Talk mode
        "talk.mode" => handle_talk_mode(params),

        // Models/agents/skills
        "models.list" => handle_models_list(),
        "agents.list" => handle_agents_list(),
        "skills.status" => handle_skills_status(),
        "skills.bins" => handle_skills_bins(),
        "skills.install" => handle_skills_install(params),
        "skills.update" => handle_skills_update(params),
        "update.run" => handle_update_run(),

        // Cron
        "cron.status" => handle_cron_status(),
        "cron.list" => handle_cron_list(),
        "cron.add" => handle_cron_add(params),
        "cron.update" => handle_cron_update(params),
        "cron.remove" => handle_cron_remove(params),
        "cron.run" => handle_cron_run(params),
        "cron.runs" => handle_cron_runs(params),

        // Node pairing
        "node.pair.request" => handle_node_pair_request(params, state),
        "node.pair.list" => handle_node_pair_list(state),
        "node.pair.approve" => handle_node_pair_approve(params, state),
        "node.pair.reject" => handle_node_pair_reject(params, state),
        "node.pair.verify" => handle_node_pair_verify(params, state),
        "node.rename" => handle_node_rename(params, state),
        "node.list" => handle_node_list(state),
        "node.describe" => handle_node_describe(params, state),
        "node.invoke" => handle_node_invoke(params, state).await,
        "node.invoke.result" => handle_node_invoke_result(params, state, conn),
        "node.event" => handle_node_event(params, state, conn),

        // Device pairing
        "device.pair.list" => handle_device_pair_list(state),
        "device.pair.approve" => handle_device_pair_approve(params, state),
        "device.pair.reject" => handle_device_pair_reject(params, state),
        "device.token.rotate" => handle_device_token_rotate(params, state),
        "device.token.revoke" => handle_device_token_revoke(params, state),

        // Exec approvals
        "exec.approvals.get" => handle_exec_approvals_get(),
        "exec.approvals.set" => handle_exec_approvals_set(params),
        "exec.approvals.node.get" => handle_exec_approvals_node_get(params),
        "exec.approvals.node.set" => handle_exec_approvals_node_set(params),
        "exec.approval.request" => handle_exec_approval_request(params),
        "exec.approval.resolve" => handle_exec_approval_resolve(params),

        // Usage
        "usage.status" => handle_usage_status(),
        "usage.cost" => handle_usage_cost(params),

        // Logs
        "logs.tail" => handle_logs_tail(params),

        // Misc
        "last-heartbeat" => handle_last_heartbeat(),
        "set-heartbeats" => handle_set_heartbeats(params),
        "wake" => handle_wake(params),
        "send" => handle_send(state, params, conn),
        "system-presence" => handle_system_presence(params),
        "system-event" => handle_system_event(params),

        _ => Err(error_shape(
            ERROR_UNAVAILABLE,
            "method unavailable",
            Some(json!({ "method": method })),
        )),
    }
}

pub(super) fn handle_config_get(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let snapshot = read_config_snapshot();
    let key = params
        .and_then(|v| v.get("key"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    if let Some(key) = key {
        let value = get_value_at_path(&snapshot.config, key).unwrap_or(Value::Null);
        return Ok(json!({
            "key": key,
            "value": value
        }));
    }

    Ok(json!({
        "path": snapshot.path,
        "exists": snapshot.exists,
        "raw": snapshot.raw,
        "parsed": snapshot.parsed,
        "valid": snapshot.valid,
        "config": snapshot.config,
        "hash": snapshot.hash,
        "issues": snapshot.issues,
        "warnings": [],
        "legacyIssues": []
    }))
}

pub(super) fn handle_config_set(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let snapshot = read_config_snapshot();
    require_config_base_hash(params, &snapshot)?;

    let raw = params
        .and_then(|v| v.get("raw"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "raw is required", None))?;
    let parsed = json5::from_str::<Value>(raw)
        .map_err(|err| error_shape(ERROR_INVALID_REQUEST, &err.to_string(), None))?;
    if !parsed.is_object() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "config.set raw must be an object",
            None,
        ));
    }
    let issues = map_validation_issues(config::validate_config(&parsed));
    if !issues.is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "invalid config",
            Some(json!({ "issues": issues })),
        ));
    }
    write_config_file(&config::get_config_path(), &parsed)?;
    Ok(json!({
        "ok": true,
        "path": config::get_config_path().display().to_string(),
        "config": parsed
    }))
}

pub(super) fn handle_config_apply(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let snapshot = read_config_snapshot();
    require_config_base_hash(params, &snapshot)?;

    let raw = params
        .and_then(|v| v.get("raw"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "raw is required", None))?;
    let parsed = json5::from_str::<Value>(raw)
        .map_err(|err| error_shape(ERROR_INVALID_REQUEST, &err.to_string(), None))?;
    if !parsed.is_object() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "config.apply raw must be an object",
            None,
        ));
    }
    let issues = map_validation_issues(config::validate_config(&parsed));
    if !issues.is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "invalid config",
            Some(json!({ "issues": issues })),
        ));
    }
    write_config_file(&config::get_config_path(), &parsed)?;
    Ok(json!({
        "ok": true,
        "path": config::get_config_path().display().to_string(),
        "config": parsed
    }))
}

pub(super) fn handle_config_patch(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let snapshot = read_config_snapshot();
    require_config_base_hash(params, &snapshot)?;

    let raw = params
        .and_then(|v| v.get("raw"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "raw is required", None))?;
    let patch_value = json5::from_str::<Value>(raw)
        .map_err(|err| error_shape(ERROR_INVALID_REQUEST, &err.to_string(), None))?;
    if !patch_value.is_object() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "config.patch raw must be an object",
            None,
        ));
    }

    let merged = merge_patch(snapshot.config.clone(), patch_value);
    let issues = map_validation_issues(config::validate_config(&merged));
    if !issues.is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "invalid config",
            Some(json!({ "issues": issues })),
        ));
    }

    write_config_file(&config::get_config_path(), &merged)?;
    Ok(json!({
        "ok": true,
        "path": config::get_config_path().display().to_string(),
        "config": merged
    }))
}

pub(super) fn handle_config_schema() -> Result<Value, ErrorShape> {
    // Return JSON schema for config
    Ok(json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {
            "gateway": { "type": "object" },
            "agent": { "type": "object" },
            "channels": { "type": "object" }
        }
    }))
}

pub(super) fn handle_sessions_list(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let mut filter = sessions::SessionFilter::new();
    if let Some(limit) = params.and_then(|v| v.get("limit")).and_then(|v| v.as_i64()) {
        if limit > 0 {
            filter = filter.with_limit(limit as usize);
        }
    }
    if let Some(offset) = params
        .and_then(|v| v.get("offset"))
        .and_then(|v| v.as_i64())
    {
        if offset >= 0 {
            filter = filter.with_offset(offset as usize);
        }
    }
    if let Some(agent_id) = params
        .and_then(|v| v.get("agentId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        filter = filter.with_agent_id(agent_id);
    }
    if let Some(channel) = params
        .and_then(|v| v.get("channel"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        filter = filter.with_channel(channel);
    }
    if let Some(user_id) = params
        .and_then(|v| v.get("userId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        filter.user_id = Some(user_id.to_string());
    }
    if let Some(status) = params
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str())
        .and_then(parse_session_status)
    {
        filter = filter.with_status(status);
    }
    let active_minutes = params
        .and_then(|v| v.get("activeMinutes"))
        .and_then(|v| v.as_i64())
        .map(|v| v.max(1) as i64);
    let label_filter = params
        .and_then(|v| v.get("label"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let search_filter = params
        .and_then(|v| v.get("search"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let sessions = state.session_store.list_sessions(filter).map_err(|err| {
        error_shape(
            ERROR_UNAVAILABLE,
            &format!("session list failed: {}", err),
            None,
        )
    })?;

    let include_global = params
        .and_then(|v| v.get("includeGlobal"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_unknown = params
        .and_then(|v| v.get("includeUnknown"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let agent_filter = params
        .and_then(|v| v.get("agentId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let include_last_message = params
        .and_then(|v| v.get("includeLastMessage"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_derived_titles = params
        .and_then(|v| v.get("includeDerivedTitles"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let now = now_ms() as i64;
    let rows = sessions
        .iter()
        .filter(|session| {
            if !include_global && session.session_key == "global" {
                return false;
            }
            if !include_unknown && session.session_key == "unknown" {
                return false;
            }
            if let Some(ref agent_id) = agent_filter {
                if let Some(meta_id) = session.metadata.agent_id.as_deref() {
                    if meta_id != agent_id.as_str() {
                        return false;
                    }
                } else if let Some(parsed) = parse_agent_session_key(&session.session_key) {
                    if parsed != agent_id.as_str() {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if let Some(minutes) = active_minutes {
                let cutoff = now - minutes * 60_000;
                if session.updated_at < cutoff {
                    return false;
                }
            }
            if let Some(ref label) = label_filter {
                if session.metadata.name.as_deref() != Some(label.as_str()) {
                    return false;
                }
            }
            if let Some(ref search) = search_filter {
                let mut haystack = session.session_key.clone();
                if let Some(name) = session.metadata.name.as_ref() {
                    haystack.push(' ');
                    haystack.push_str(name);
                }
                if !haystack.to_lowercase().contains(&search.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .map(|session| {
            let mut row = session_row(session);
            if include_last_message {
                if let Ok(messages) = state.session_store.get_history(&session.id, Some(1), None) {
                    if let Some(last) = messages.last() {
                        if let Some(obj) = row.as_object_mut() {
                            obj.insert(
                                "lastMessagePreview".to_string(),
                                Value::String(truncate_preview(&last.content, 200)),
                            );
                        }
                    }
                }
            }
            if include_derived_titles {
                if let Ok(messages) = state.session_store.get_history(&session.id, None, None) {
                    let title = messages
                        .iter()
                        .find(|msg| matches!(msg.role, sessions::MessageRole::User))
                        .map(|msg| truncate_preview(&msg.content, 60));
                    if let Some(title) = title {
                        if let Some(obj) = row.as_object_mut() {
                            obj.insert("derivedTitle".to_string(), Value::String(title));
                        }
                    }
                }
            }
            row
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "ts": now_ms(),
        "path": state.session_store.base_path().display().to_string(),
        "count": rows.len(),
        "defaults": {
            "modelProvider": null,
            "model": null,
            "contextTokens": null
        },
        "sessions": rows
    }))
}

pub(super) fn handle_sessions_preview(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let keys = params
        .and_then(|v| v.get("keys"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .take(64)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let limit = params
        .and_then(|v| v.get("limit"))
        .and_then(|v| v.as_i64())
        .map(|v| v.max(1) as usize)
        .unwrap_or(12);
    let max_chars = params
        .and_then(|v| v.get("maxChars"))
        .and_then(|v| v.as_i64())
        .map(|v| v.max(20) as usize)
        .unwrap_or(240);

    let previews = keys
        .into_iter()
        .map(|key| match state.session_store.get_session_by_key(&key) {
            Ok(session) => match state
                .session_store
                .get_history(&session.id, Some(limit), None)
            {
                Ok(messages) => {
                    if messages.is_empty() {
                        json!({ "key": key, "status": "empty", "items": [] })
                    } else {
                        let items = messages
                            .into_iter()
                            .map(|msg| {
                                let mut text = msg.content;
                                if text.len() > max_chars {
                                    text.truncate(max_chars);
                                }
                                json!({
                                    "role": role_to_string(msg.role),
                                    "text": text
                                })
                            })
                            .collect::<Vec<_>>();
                        json!({ "key": key, "status": "ok", "items": items })
                    }
                }
                Err(_) => json!({ "key": key, "status": "error", "items": [] }),
            },
            Err(sessions::SessionStoreError::NotFound(_)) => {
                json!({ "key": key, "status": "missing", "items": [] })
            }
            Err(_) => json!({ "key": key, "status": "error", "items": [] }),
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "ts": now_ms(),
        "previews": previews
    }))
}

/// Extract session key from params (supports both "key" and "sessionKey" fields)
fn extract_session_key(params: Option<&Value>) -> Option<String> {
    params
        .and_then(|v| v.get("key").or_else(|| v.get("sessionKey")))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn read_string_param(params: Option<&Value>, key: &str) -> Option<String> {
    params
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn parse_session_status(raw: &str) -> Option<sessions::SessionStatus> {
    match raw {
        "active" => Some(sessions::SessionStatus::Active),
        "paused" => Some(sessions::SessionStatus::Paused),
        "archived" => Some(sessions::SessionStatus::Archived),
        "compacting" => Some(sessions::SessionStatus::Compacting),
        _ => None,
    }
}

#[derive(Debug)]
struct LogSlice {
    cursor: u64,
    size: u64,
    lines: Vec<String>,
    truncated: bool,
    reset: bool,
}

fn resolve_log_file_path() -> PathBuf {
    if let Ok(path) = env::var("CLAWDBOT_LOG_FILE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    resolve_state_dir().join("logs").join("clawdbot.log")
}

fn resolve_log_file(path: &PathBuf) -> PathBuf {
    if path.exists() {
        return path.clone();
    }
    static ROLLING_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^clawdbot-\d{4}-\d{2}-\d{2}\.log$").unwrap());
    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    if !ROLLING_RE.is_match(file_name) {
        return path.clone();
    }
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return path.clone(),
    };
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let candidate = entry.path();
        let candidate_name = candidate.file_name().and_then(|v| v.to_str()).unwrap_or("");
        if !ROLLING_RE.is_match(candidate_name) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                let is_newer = newest
                    .as_ref()
                    .map(|(_, ts)| modified > *ts)
                    .unwrap_or(true);
                if is_newer {
                    newest = Some((candidate.clone(), modified));
                }
            }
        }
    }
    newest.map(|(path, _)| path).unwrap_or_else(|| path.clone())
}

fn read_log_slice(
    file: &PathBuf,
    cursor: Option<u64>,
    limit: usize,
    max_bytes: usize,
) -> Result<LogSlice, ErrorShape> {
    let meta = match fs::metadata(file) {
        Ok(meta) => meta,
        Err(_) => {
            return Ok(LogSlice {
                cursor: 0,
                size: 0,
                lines: Vec::new(),
                truncated: false,
                reset: false,
            })
        }
    };
    let size = meta.len();
    let mut reset = false;
    let mut truncated = false;
    let mut start: u64;

    if let Some(cursor) = cursor {
        if cursor > size {
            reset = true;
            start = size.saturating_sub(max_bytes as u64);
            truncated = start > 0;
        } else {
            start = cursor;
            if size.saturating_sub(start) > max_bytes as u64 {
                reset = true;
                truncated = true;
                start = size.saturating_sub(max_bytes as u64);
            }
        }
    } else {
        start = size.saturating_sub(max_bytes as u64);
        truncated = start > 0;
    }

    if size == 0 || size <= start {
        return Ok(LogSlice {
            cursor: size,
            size,
            lines: Vec::new(),
            truncated,
            reset,
        });
    }

    let mut file_handle = fs::File::open(file).map_err(|err| {
        error_shape(
            ERROR_UNAVAILABLE,
            &format!("log read failed: {}", err),
            None,
        )
    })?;
    let mut prefix = String::new();
    if start > 0 {
        file_handle.seek(SeekFrom::Start(start - 1)).ok();
        let mut buf = [0u8; 1];
        if let Ok(read) = file_handle.read(&mut buf) {
            if read > 0 {
                prefix = String::from_utf8_lossy(&buf[..read]).to_string();
            }
        }
    }
    file_handle.seek(SeekFrom::Start(start)).ok();
    let mut buffer = vec![0u8; (size - start) as usize];
    let read = file_handle.read(&mut buffer).unwrap_or(0);
    buffer.truncate(read);
    let text = String::from_utf8_lossy(&buffer).to_string();
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    if start > 0 && prefix != "\n" && !lines.is_empty() {
        lines.remove(0);
    }
    if lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }

    Ok(LogSlice {
        cursor: size,
        size,
        lines,
        truncated,
        reset,
    })
}

fn ensure_object(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(serde_json::Map::new());
    }
    value.as_object_mut().expect("value is object")
}

fn role_to_string(role: sessions::MessageRole) -> &'static str {
    match role {
        sessions::MessageRole::User => "user",
        sessions::MessageRole::Assistant => "assistant",
        sessions::MessageRole::System => "system",
        sessions::MessageRole::Tool => "tool",
    }
}

fn resolve_workspace_dir(cfg: &Value) -> PathBuf {
    if let Ok(dir) = env::var("CLAWDBOT_WORKSPACE_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Some(workspace) = cfg
        .get("agents")
        .and_then(|v| v.get("defaults"))
        .and_then(|v| v.get("workspace"))
        .and_then(|v| v.as_str())
    {
        if !workspace.trim().is_empty() {
            return PathBuf::from(workspace);
        }
    }
    if let Some(list) = cfg
        .get("agents")
        .and_then(|v| v.get("list"))
        .and_then(|v| v.as_array())
    {
        for entry in list {
            if entry
                .get("default")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                if let Some(workspace) = entry.get("workspace").and_then(|v| v.as_str()) {
                    if !workspace.trim().is_empty() {
                        return PathBuf::from(workspace);
                    }
                }
            }
        }
        if let Some(first) = list.first() {
            if let Some(workspace) = first.get("workspace").and_then(|v| v.as_str()) {
                if !workspace.trim().is_empty() {
                    return PathBuf::from(workspace);
                }
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn session_row(session: &sessions::Session) -> Value {
    json!({
        "key": session.session_key,
        "kind": classify_session_key(&session.session_key, &session.metadata),
        "label": session.metadata.name,
        "displayName": session.metadata.description,
        "channel": session.metadata.channel,
        "chatId": session.metadata.chat_id,
        "userId": session.metadata.user_id,
        "updatedAt": session.updated_at,
        "sessionId": session.id,
        "messageCount": session.message_count,
        "thinkingLevel": session.metadata.thinking_level,
        "model": session.metadata.model
    })
}

fn session_entry(session: &sessions::Session) -> Value {
    json!({
        "sessionId": session.id,
        "updatedAt": session.updated_at,
        "label": session.metadata.name,
        "thinkingLevel": session.metadata.thinking_level,
        "model": session.metadata.model,
        "channel": session.metadata.channel,
        "chatId": session.metadata.chat_id,
        "agentId": session.metadata.agent_id,
        "userId": session.metadata.user_id,
        "status": session.status.to_string()
    })
}

fn classify_session_key(key: &str, metadata: &sessions::SessionMetadata) -> &'static str {
    if key == "global" {
        return "global";
    }
    if key == "unknown" {
        return "unknown";
    }
    if key.contains(":group:") || key.contains(":channel:") {
        return "group";
    }
    if let Some(chat_id) = metadata.chat_id.as_deref() {
        if chat_id.contains(":group:") || chat_id.contains(":channel:") {
            return "group";
        }
    }
    "direct"
}

fn parse_agent_session_key(key: &str) -> Option<&str> {
    if key == "global" || key == "unknown" {
        return None;
    }
    let mut parts = key.splitn(3, ':');
    let agent = parts.next()?;
    let channel = parts.next()?;
    let chat = parts.next()?;
    if agent.is_empty() || channel.is_empty() || chat.is_empty() {
        return None;
    }
    Some(agent)
}

fn truncate_preview(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut out = text[..max_len].to_string();
    out.push('…');
    out
}

fn build_session_metadata(params: Option<&Value>) -> sessions::SessionMetadata {
    let mut meta = sessions::SessionMetadata::default();
    if let Some(label) = read_string_param(params, "label") {
        meta.name = Some(label);
    }
    if let Some(description) = read_string_param(params, "description") {
        meta.description = Some(description);
    }
    if let Some(agent_id) = read_string_param(params, "agentId") {
        meta.agent_id = Some(agent_id);
    }
    if let Some(channel) = read_string_param(params, "channel") {
        meta.channel = Some(channel);
    }
    if let Some(user_id) = read_string_param(params, "userId") {
        meta.user_id = Some(user_id);
    }
    if let Some(model) = read_string_param(params, "model") {
        meta.model = Some(model);
    }
    if let Some(thinking_level) = read_string_param(params, "thinkingLevel") {
        meta.thinking_level = Some(thinking_level);
    }
    if let Some(tags) = params
        .and_then(|v| v.get("tags"))
        .and_then(|v| v.as_array())
    {
        meta.tags = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(extra) = params.and_then(|v| v.get("extra")) {
        if !extra.is_null() {
            meta.extra = Some(extra.clone());
        }
    }
    meta
}

fn has_metadata_updates(meta: &sessions::SessionMetadata) -> bool {
    meta.name.is_some()
        || meta.description.is_some()
        || meta.agent_id.is_some()
        || meta.channel.is_some()
        || meta.user_id.is_some()
        || meta.model.is_some()
        || meta.thinking_level.is_some()
        || !meta.tags.is_empty()
        || meta.extra.is_some()
}

pub(super) fn handle_sessions_patch(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let key = extract_session_key(params)
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "key is required", None))?;
    let updates = build_session_metadata(params);
    let has_updates = has_metadata_updates(&updates);

    let session = match state.session_store.get_session_by_key(&key) {
        Ok(existing) => {
            if has_updates {
                state
                    .session_store
                    .patch_session(&existing.id, updates)
                    .map_err(|err| {
                        error_shape(
                            ERROR_UNAVAILABLE,
                            &format!("session patch failed: {}", err),
                            None,
                        )
                    })?
            } else {
                existing
            }
        }
        Err(sessions::SessionStoreError::NotFound(_)) => {
            let metadata = if has_updates {
                updates
            } else {
                sessions::SessionMetadata::default()
            };
            state
                .session_store
                .get_or_create_session(&key, metadata)
                .map_err(|err| {
                    error_shape(
                        ERROR_UNAVAILABLE,
                        &format!("session create failed: {}", err),
                        None,
                    )
                })?
        }
        Err(err) => {
            return Err(error_shape(
                ERROR_UNAVAILABLE,
                &format!("session load failed: {}", err),
                None,
            ))
        }
    };

    Ok(json!({
        "ok": true,
        "path": state.session_store.base_path().display().to_string(),
        "key": session.session_key,
        "entry": session_entry(&session)
    }))
}

pub(super) fn handle_sessions_reset(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let key = extract_session_key(params)
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "key is required", None))?;
    let session = state
        .session_store
        .get_or_create_session(&key, sessions::SessionMetadata::default())
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session create failed: {}", err),
                None,
            )
        })?;
    let reset = state
        .session_store
        .reset_session(&session.id)
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session reset failed: {}", err),
                None,
            )
        })?;
    Ok(json!({ "ok": true, "key": reset.session_key, "entry": session_entry(&reset) }))
}

pub(super) fn handle_sessions_delete(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let key = extract_session_key(params)
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "key is required", None))?;
    let session = match state.session_store.get_session_by_key(&key) {
        Ok(session) => Some(session),
        Err(sessions::SessionStoreError::NotFound(_)) => None,
        Err(err) => {
            return Err(error_shape(
                ERROR_UNAVAILABLE,
                &format!("session load failed: {}", err),
                None,
            ))
        }
    };

    let deleted = if let Some(session) = session {
        state
            .session_store
            .delete_session(&session.id)
            .map_err(|err| {
                error_shape(
                    ERROR_UNAVAILABLE,
                    &format!("session delete failed: {}", err),
                    None,
                )
            })?;
        true
    } else {
        false
    };

    Ok(json!({ "ok": true, "key": key, "deleted": deleted }))
}

pub(super) fn handle_sessions_compact(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let key = extract_session_key(params)
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "key is required", None))?;
    let keep_recent = params
        .and_then(|v| v.get("maxLines"))
        .and_then(|v| v.as_i64())
        .map(|v| v.max(1) as usize)
        .unwrap_or(400);

    let session = match state.session_store.get_session_by_key(&key) {
        Ok(session) => session,
        Err(sessions::SessionStoreError::NotFound(_)) => {
            return Ok(json!({
                "ok": true,
                "key": key,
                "compacted": false,
                "reason": "not_found"
            }))
        }
        Err(err) => {
            return Err(error_shape(
                ERROR_UNAVAILABLE,
                &format!("session load failed: {}", err),
                None,
            ))
        }
    };

    let history_len = state
        .session_store
        .get_history(&session.id, None, None)
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session history failed: {}", err),
                None,
            )
        })?
        .len();

    if history_len <= keep_recent {
        return Ok(json!({
            "ok": true,
            "key": key,
            "compacted": false,
            "kept": history_len
        }));
    }

    let compacted = state
        .session_store
        .compact_session(
            &session.id,
            keep_recent,
            |messages: &[sessions::ChatMessage]| format!("Compacted {} messages.", messages.len()),
        )
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session compact failed: {}", err),
                None,
            )
        })?;

    Ok(json!({
        "ok": true,
        "key": session.session_key,
        "compacted": compacted.messages_compacted > 0,
        "kept": keep_recent
    }))
}

pub(super) fn handle_channels_status(state: &WsServerState) -> Result<Value, ErrorShape> {
    let snapshot = state.channel_registry.snapshot();
    let connected = snapshot
        .channels
        .iter()
        .filter(|c| c.status == channels::ChannelStatus::Connected)
        .count();
    Ok(json!({
        "total": snapshot.channels.len(),
        "connected": connected,
        "channels": snapshot.channels,
        "ts": snapshot.timestamp
    }))
}

pub(super) fn handle_channels_logout(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let channel = params
        .and_then(|v| v.get("channel").or_else(|| v.get("channelId")))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "channel is required", None))?;
    if !state.channel_registry.logout(channel) {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "unknown channel",
            Some(json!({ "channel": channel })),
        ));
    }
    Ok(json!({
        "ok": true,
        "channel": channel
    }))
}

pub(super) fn handle_agent(
    params: Option<&Value>,
    state: &WsServerState,
    _conn: &ConnectionContext,
) -> Result<Value, ErrorShape> {
    let message = params
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "message is required", None))?;
    if message.trim().is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "message is required",
            None,
        ));
    }
    let idempotency_key = params
        .and_then(|v| v.get("idempotencyKey"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "idempotencyKey is required", None))?;
    let session_key = params
        .and_then(|v| v.get("sessionKey"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("default");

    let metadata = build_session_metadata(params);
    let session = state
        .session_store
        .get_or_create_session(session_key, metadata)
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session create failed: {}", err),
                None,
            )
        })?;
    state
        .session_store
        .append_message(sessions::ChatMessage::user(session.id.clone(), message))
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session write failed: {}", err),
                None,
            )
        })?;

    // In full implementation, this would queue an agent run
    Ok(json!({
        "runId": idempotency_key,
        "status": "started",
        "message": message,
        "sessionKey": session.session_key
    }))
}

pub(super) fn handle_agent_identity_get(_state: &WsServerState) -> Result<Value, ErrorShape> {
    // Return agent identity (would read from config)
    Ok(json!({
        "agentId": "default",
        "name": "Clawdbot"
    }))
}

pub(super) fn handle_agent_wait(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let run_id = params
        .and_then(|v| v.get("runId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "runId is required", None))?;
    // In full implementation, this would wait for an agent run to complete
    Ok(json!({
        "runId": run_id,
        "status": "completed",
        "result": null
    }))
}

pub(super) fn handle_chat_history(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let session_id = params
        .and_then(|v| v.get("sessionId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let session_key = extract_session_key(params);
    let limit = params
        .and_then(|v| v.get("limit"))
        .and_then(|v| v.as_i64())
        .map(|v| v.max(1) as usize)
        .unwrap_or(200)
        .min(1000);

    let session = if let Some(session_id) = session_id {
        state
            .session_store
            .get_session(session_id)
            .map_err(|err| error_shape(ERROR_INVALID_REQUEST, &err.to_string(), None))?
    } else {
        let key = session_key
            .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "sessionKey is required", None))?;
        state
            .session_store
            .get_session_by_key(&key)
            .map_err(|err| error_shape(ERROR_INVALID_REQUEST, &err.to_string(), None))?
    };

    let messages = state
        .session_store
        .get_history(&session.id, Some(limit), None)
        .map_err(|err| error_shape(ERROR_UNAVAILABLE, &err.to_string(), None))?
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id,
                "role": role_to_string(m.role),
                "content": m.content,
                "ts": m.created_at
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "sessionKey": session.session_key,
        "sessionId": session.id,
        "messages": messages,
        "thinkingLevel": session
            .metadata
            .thinking_level
            .clone()
            .unwrap_or_else(|| "off".to_string())
    }))
}

pub(super) fn handle_chat_send(
    state: &WsServerState,
    params: Option<&Value>,
    _conn: &ConnectionContext,
) -> Result<Value, ErrorShape> {
    let session_id = params
        .and_then(|v| v.get("sessionId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let session_key = extract_session_key(params);
    let message = params
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "message is required", None))?;
    if message.trim().is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "message is required",
            None,
        ));
    }
    let idempotency_key = params
        .and_then(|v| v.get("idempotencyKey"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "idempotencyKey is required", None))?;

    let session = if let Some(session_id) = session_id {
        state
            .session_store
            .get_session(session_id)
            .map_err(|err| error_shape(ERROR_INVALID_REQUEST, &err.to_string(), None))?
    } else {
        let key = session_key
            .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "sessionKey is required", None))?;
        state
            .session_store
            .get_or_create_session(&key, sessions::SessionMetadata::default())
            .map_err(|err| {
                error_shape(
                    ERROR_UNAVAILABLE,
                    &format!("session create failed: {}", err),
                    None,
                )
            })?
    };

    state
        .session_store
        .append_message(sessions::ChatMessage::user(session.id.clone(), message))
        .map_err(|err| {
            error_shape(
                ERROR_UNAVAILABLE,
                &format!("session write failed: {}", err),
                None,
            )
        })?;

    Ok(json!({
        "runId": idempotency_key,
        "status": "queued",
        "sessionKey": session.session_key
    }))
}

pub(super) fn handle_chat_abort(
    state: &WsServerState,
    params: Option<&Value>,
) -> Result<Value, ErrorShape> {
    let session_id = params
        .and_then(|v| v.get("sessionId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let session_key = extract_session_key(params);
    let run_id = params
        .and_then(|v| v.get("runId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let session = if let Some(session_id) = session_id {
        state.session_store.get_session(session_id).ok()
    } else if let Some(key) = session_key.as_deref() {
        state.session_store.get_session_by_key(key).ok()
    } else {
        None
    };
    Ok(json!({
        "ok": true,
        "aborted": false,
        "sessionKey": session
            .as_ref()
            .map(|s| s.session_key.clone())
            .or(session_key),
        "runIds": run_id.map(|id| vec![id]).unwrap_or_default()
    }))
}

pub(super) fn handle_tts_status() -> Result<Value, ErrorShape> {
    Ok(json!({
        "enabled": false,
        "provider": null
    }))
}

pub(super) fn handle_tts_providers() -> Result<Value, ErrorShape> {
    Ok(json!({
        "providers": ["system", "elevenlabs", "openai"]
    }))
}

pub(super) fn handle_tts_enable() -> Result<Value, ErrorShape> {
    Ok(json!({ "ok": true, "enabled": true }))
}

pub(super) fn handle_tts_disable() -> Result<Value, ErrorShape> {
    Ok(json!({ "ok": true, "enabled": false }))
}

pub(super) fn handle_tts_convert(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let text = params
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "text is required", None))?;
    // In full implementation, would convert text to speech
    Ok(json!({
        "ok": true,
        "text": text,
        "audio": null
    }))
}

pub(super) fn handle_tts_set_provider(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let provider = params
        .and_then(|v| v.get("provider"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "provider is required", None))?;
    Ok(json!({
        "ok": true,
        "provider": provider
    }))
}

pub(super) fn handle_voicewake_get() -> Result<Value, ErrorShape> {
    Ok(json!({
        "enabled": false,
        "keyword": null
    }))
}

pub(super) fn handle_voicewake_set(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let enabled = params
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Ok(json!({
        "ok": true,
        "enabled": enabled
    }))
}

pub(super) fn handle_wizard_start(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let wizard_type = params
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("setup");
    Ok(json!({
        "ok": true,
        "wizardId": Uuid::new_v4().to_string(),
        "type": wizard_type,
        "step": 0
    }))
}

pub(super) fn handle_wizard_next(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let wizard_id = params
        .and_then(|v| v.get("wizardId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "wizardId is required", None))?;
    Ok(json!({
        "ok": true,
        "wizardId": wizard_id,
        "step": 1,
        "complete": false
    }))
}

pub(super) fn handle_wizard_cancel() -> Result<Value, ErrorShape> {
    Ok(json!({ "ok": true, "cancelled": true }))
}

pub(super) fn handle_wizard_status() -> Result<Value, ErrorShape> {
    Ok(json!({
        "active": false,
        "wizardId": null
    }))
}

pub(super) fn handle_talk_mode(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let mode = params
        .and_then(|v| v.get("mode"))
        .and_then(|v| v.as_str())
        .unwrap_or("off");
    Ok(json!({
        "ok": true,
        "mode": mode
    }))
}

pub(super) fn handle_models_list() -> Result<Value, ErrorShape> {
    let cfg = config::load_config().unwrap_or(Value::Object(serde_json::Map::new()));
    let mut models = Vec::new();
    if let Some(map) = cfg
        .get("agents")
        .and_then(|v| v.get("defaults"))
        .and_then(|v| v.get("models"))
        .and_then(|v| v.as_object())
    {
        for (id, entry) in map {
            let alias = entry
                .get("alias")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let label = entry
                .get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            models.push(json!({
                "id": id,
                "alias": alias,
                "label": label
            }));
        }
    }
    Ok(json!({ "models": models }))
}

pub(super) fn handle_agents_list() -> Result<Value, ErrorShape> {
    let cfg = config::load_config().unwrap_or(Value::Object(serde_json::Map::new()));
    let mut agents = Vec::new();
    let mut default_id: Option<String> = None;
    let mut main_key: Option<String> = None;
    let mut scope: Option<String> = None;
    if let Some(session_obj) = cfg.get("session").and_then(|v| v.as_object()) {
        if let Some(main_key_value) = session_obj.get("mainKey").and_then(|v| v.as_str()) {
            if !main_key_value.trim().is_empty() {
                main_key = Some(main_key_value.trim().to_string());
            }
        }
        if let Some(scope_value) = session_obj.get("scope").and_then(|v| v.as_str()) {
            if !scope_value.trim().is_empty() {
                scope = Some(scope_value.trim().to_string());
            }
        }
    }
    if let Some(list) = cfg
        .get("agents")
        .and_then(|v| v.get("list"))
        .and_then(|v| v.as_array())
    {
        for entry in list {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                if entry
                    .get("default")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    default_id = Some(id.to_string());
                }
                let name = entry
                    .get("identity")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                agents.push(json!({
                    "id": id,
                    "name": name,
                    "identity": entry.get("identity").cloned().unwrap_or(Value::Null)
                }));
            }
        }
    }
    if agents.is_empty() {
        agents.push(json!({
            "id": "default",
            "name": "Clawdbot"
        }));
    }
    if default_id.is_none() {
        if let Some(first) = agents.first() {
            if let Some(id) = first.get("id").and_then(|v| v.as_str()) {
                default_id = Some(id.to_string());
            }
        }
    }
    Ok(json!({
        "defaultId": default_id.unwrap_or_else(|| "default".to_string()),
        "mainKey": main_key.unwrap_or_else(|| "main".to_string()),
        "scope": scope.unwrap_or_else(|| "per-sender".to_string()),
        "agents": agents
    }))
}

pub(super) fn handle_skills_status() -> Result<Value, ErrorShape> {
    let cfg = config::load_config().unwrap_or(Value::Object(serde_json::Map::new()));
    let workspace_dir = resolve_workspace_dir(&cfg);
    let managed_skills_dir = workspace_dir.join("skills");
    Ok(json!({
        "workspaceDir": workspace_dir.to_string_lossy(),
        "managedSkillsDir": managed_skills_dir.to_string_lossy(),
        "skills": []
    }))
}

pub(super) fn handle_skills_bins() -> Result<Value, ErrorShape> {
    Ok(json!({ "bins": [] }))
}

pub(super) fn handle_skills_install(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let name = params
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "name is required", None))?;
    let install_id = params
        .and_then(|v| v.get("installId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "installId is required", None))?;
    let timeout_ms = params
        .and_then(|v| v.get("timeoutMs"))
        .and_then(|v| v.as_i64())
        .map(|v| v.max(1000) as u64);

    let mut config_value = read_config_snapshot().config;
    let root = ensure_object(&mut config_value);
    let skills = root.entry("skills").or_insert_with(|| json!({}));
    let skills_obj = ensure_object(skills);
    let entries = skills_obj.entry("entries").or_insert_with(|| json!({}));
    let entries_obj = ensure_object(entries);
    let entry = entries_obj
        .entry(name.to_string())
        .or_insert_with(|| json!({}));
    let entry_obj = ensure_object(entry);
    entry_obj.insert("enabled".to_string(), Value::Bool(true));
    entry_obj.insert("name".to_string(), Value::String(name.to_string()));
    entry_obj.insert(
        "installId".to_string(),
        Value::String(install_id.to_string()),
    );
    entry_obj.insert("requestedAt".to_string(), Value::Number(now_ms().into()));

    let issues = map_validation_issues(config::validate_config(&config_value));
    if !issues.is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "invalid config",
            Some(json!({ "issues": issues })),
        ));
    }
    write_config_file(&config::get_config_path(), &config_value)?;

    Ok(json!({
        "ok": true,
        "name": name,
        "installId": install_id,
        "timeoutMs": timeout_ms,
        "queued": true
    }))
}

pub(super) fn handle_skills_update(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let skill_key = params
        .and_then(|v| v.get("skillKey"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "skillKey is required", None))?;
    let enabled = params
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool());
    let api_key = params
        .and_then(|v| v.get("apiKey"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());
    let env_map = params
        .and_then(|v| v.get("env"))
        .and_then(|v| v.as_object())
        .cloned();

    let mut config_value = read_config_snapshot().config;
    let root = ensure_object(&mut config_value);
    let skills = root.entry("skills").or_insert_with(|| json!({}));
    let skills_obj = ensure_object(skills);
    let entries = skills_obj.entry("entries").or_insert_with(|| json!({}));
    let entries_obj = ensure_object(entries);
    let entry = entries_obj
        .entry(skill_key.to_string())
        .or_insert_with(|| json!({}));
    let entry_obj = ensure_object(entry);

    if let Some(enabled) = enabled {
        entry_obj.insert("enabled".to_string(), Value::Bool(enabled));
    }
    if let Some(api_key) = api_key {
        if api_key.trim().is_empty() {
            entry_obj.remove("apiKey");
        } else {
            entry_obj.insert("apiKey".to_string(), Value::String(api_key));
        }
    }
    if let Some(env_map) = env_map {
        let env_value = entry_obj
            .entry("env".to_string())
            .or_insert_with(|| json!({}));
        let env_obj = ensure_object(env_value);
        for (key, value) in env_map {
            let k = key.trim().to_string();
            if k.is_empty() {
                continue;
            }
            if let Some(v) = value.as_str() {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    env_obj.remove(&k);
                } else {
                    env_obj.insert(k, Value::String(trimmed.to_string()));
                }
            }
        }
    }

    let issues = map_validation_issues(config::validate_config(&config_value));
    if !issues.is_empty() {
        return Err(error_shape(
            ERROR_INVALID_REQUEST,
            "invalid config",
            Some(json!({ "issues": issues })),
        ));
    }
    write_config_file(&config::get_config_path(), &config_value)?;

    Ok(json!({
        "ok": true,
        "skillKey": skill_key,
        "updated": true
    }))
}

pub(super) fn handle_update_run() -> Result<Value, ErrorShape> {
    Ok(json!({
        "ok": true,
        "updateAvailable": false
    }))
}

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

pub(super) fn handle_node_pair_request(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    let public_key = params
        .and_then(|v| v.get("publicKey"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let commands = params
        .and_then(|v| v.get("commands"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let display_name = params
        .and_then(|v| v.get("displayName"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let platform = params
        .and_then(|v| v.get("platform"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let outcome = state
        .node_pairing
        .request_pairing_with_status(
            node_id.to_string(),
            public_key,
            commands,
            display_name,
            platform,
        )
        .map_err(|e| match e {
            nodes::NodePairingError::NodeAlreadyPaired => {
                error_shape(ERROR_INVALID_REQUEST, "node already paired", None)
            }
            nodes::NodePairingError::TooManyPendingRequests => {
                error_shape(ERROR_UNAVAILABLE, "too many pending pairing requests", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    let request = outcome.request;
    let request_value = json!({
        "requestId": request.request_id,
        "nodeId": request.node_id,
        "displayName": request.display_name,
        "platform": request.platform,
        "commands": request.commands,
        "ts": request.created_at_ms
    });

    if outcome.created {
        broadcast_event(state, "node.pair.requested", request_value.clone());
    }

    Ok(json!({
        "status": "pending",
        "request": request_value,
        "created": outcome.created
    }))
}

pub(super) fn handle_node_pair_list(state: &WsServerState) -> Result<Value, ErrorShape> {
    let paired_nodes = state.node_pairing.list_paired_nodes();
    let (pending_requests, _resolved) = state.node_pairing.list_requests();

    let paired: Vec<Value> = paired_nodes
        .iter()
        .map(|n| {
            json!({
                "nodeId": n.node_id,
                "token": null,
                "displayName": n.display_name,
                "platform": n.platform,
                "commands": n.commands,
                "createdAtMs": n.paired_at_ms,
                "approvedAtMs": n.paired_at_ms,
                "lastConnectedAtMs": n.last_seen_ms
            })
        })
        .collect();

    let pending: Vec<Value> = pending_requests
        .iter()
        .map(|r| {
            json!({
                "requestId": r.request_id,
                "nodeId": r.node_id,
                "displayName": r.display_name,
                "platform": r.platform,
                "commands": r.commands,
                "ts": r.created_at_ms
            })
        })
        .collect();

    Ok(json!({
        "pending": pending,
        "paired": paired
    }))
}

pub(super) fn handle_node_pair_approve(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let request_id = params
        .and_then(|v| v.get("requestId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "requestId is required", None))?;

    let (node, token) = state
        .node_pairing
        .approve_request(request_id)
        .map_err(|e| match e {
            nodes::NodePairingError::RequestNotFound => {
                error_shape(ERROR_INVALID_REQUEST, "request not found", None)
            }
            nodes::NodePairingError::RequestAlreadyResolved => {
                error_shape(ERROR_INVALID_REQUEST, "request already resolved", None)
            }
            nodes::NodePairingError::RequestExpired => {
                error_shape(ERROR_INVALID_REQUEST, "request expired", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    broadcast_event(
        state,
        "node.pair.resolved",
        json!({
            "requestId": request_id,
            "nodeId": node.node_id,
            "decision": "approved",
            "ts": now_ms()
        }),
    );

    Ok(json!({
        "requestId": request_id,
        "node": {
            "nodeId": node.node_id,
            "token": token,
            "displayName": node.display_name,
            "platform": node.platform,
            "commands": node.commands,
            "createdAtMs": node.paired_at_ms,
            "approvedAtMs": node.paired_at_ms,
            "lastConnectedAtMs": node.last_seen_ms
        }
    }))
}

pub(super) fn handle_node_pair_reject(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let request_id = params
        .and_then(|v| v.get("requestId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "requestId is required", None))?;
    let reason = params
        .and_then(|v| v.get("reason"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let request = state
        .node_pairing
        .reject_request(request_id, reason)
        .map_err(|e| match e {
            nodes::NodePairingError::RequestNotFound => {
                error_shape(ERROR_INVALID_REQUEST, "request not found", None)
            }
            nodes::NodePairingError::RequestAlreadyResolved => {
                error_shape(ERROR_INVALID_REQUEST, "request already resolved", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    broadcast_event(
        state,
        "node.pair.resolved",
        json!({
            "requestId": request_id,
            "nodeId": request.node_id,
            "decision": "rejected",
            "ts": now_ms()
        }),
    );

    Ok(json!({
        "requestId": request_id,
        "nodeId": request.node_id
    }))
}

pub(super) fn handle_node_pair_verify(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    let token = params
        .and_then(|v| v.get("token"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "token is required", None))?;

    let verified = state.node_pairing.verify_token(node_id, token);
    let ok = match verified {
        Ok(()) => {
            state.node_pairing.touch_node(node_id);
            true
        }
        Err(
            nodes::NodePairingError::NodeNotPaired
            | nodes::NodePairingError::TokenInvalid
            | nodes::NodePairingError::TokenExpired
            | nodes::NodePairingError::TokenRevoked,
        ) => false,
        Err(err) => {
            return Err(error_shape(ERROR_UNAVAILABLE, &err.to_string(), None));
        }
    };

    let node_value = if ok {
        state.node_pairing.get_paired_node(node_id).map(|node| {
            json!({
                "nodeId": node.node_id,
                "token": null,
                "displayName": node.display_name,
                "platform": node.platform,
                "commands": node.commands,
                "createdAtMs": node.paired_at_ms,
                "approvedAtMs": node.paired_at_ms,
                "lastConnectedAtMs": node.last_seen_ms
            })
        })
    } else {
        None
    };

    Ok(json!({
        "ok": ok,
        "node": node_value
    }))
}

pub(super) fn handle_node_rename(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    let name = params
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "name is required", None))?;

    state
        .node_pairing
        .rename_node(node_id, name.to_string())
        .map_err(|e| match e {
            nodes::NodePairingError::NodeNotPaired => {
                error_shape(ERROR_NOT_PAIRED, "node not paired", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    Ok(json!({
        "ok": true,
        "nodeId": node_id,
        "name": name
    }))
}

pub(super) fn handle_node_list(state: &WsServerState) -> Result<Value, ErrorShape> {
    let paired_nodes = state.node_pairing.list_paired_nodes();
    let connected = state.node_registry.lock().list_connected();
    let paired_by_id: HashMap<String, nodes::PairedNode> = paired_nodes
        .into_iter()
        .map(|node| (node.node_id.clone(), node))
        .collect();
    let connected_by_id: HashMap<String, NodeSession> = connected
        .into_iter()
        .map(|node| (node.node_id.clone(), node))
        .collect();
    let mut node_ids = HashSet::new();
    node_ids.extend(paired_by_id.keys().cloned());
    node_ids.extend(connected_by_id.keys().cloned());

    let mut entries: Vec<(bool, String, String, Value)> = Vec::new();
    for node_id in node_ids {
        let paired = paired_by_id.get(&node_id);
        let live = connected_by_id.get(&node_id);
        let display_name = live
            .and_then(|n| n.display_name.clone())
            .or_else(|| paired.and_then(|n| n.display_name.clone()));
        let platform = live
            .and_then(|n| n.platform.clone())
            .or_else(|| paired.and_then(|n| n.platform.clone()));
        let version = live.and_then(|n| n.version.clone());
        let device_family = live.and_then(|n| n.device_family.clone());
        let model_identifier = live.and_then(|n| n.model_identifier.clone());
        let remote_ip = live.and_then(|n| n.remote_ip.clone());
        let caps = live.map(|n| n.caps.clone()).unwrap_or_default();
        let mut commands = HashSet::new();
        if let Some(live) = live {
            commands.extend(live.commands.iter().cloned());
        }
        if let Some(paired) = paired {
            commands.extend(paired.commands.iter().cloned());
        }
        let mut commands: Vec<String> = commands.into_iter().collect();
        commands.sort();

        let connected = live.is_some();
        let paired = paired.is_some();
        let name_key = display_name
            .clone()
            .unwrap_or_else(|| node_id.clone())
            .to_lowercase();

        let value = json!({
            "nodeId": node_id,
            "displayName": display_name,
            "platform": platform,
            "version": version,
            "coreVersion": null,
            "uiVersion": null,
            "deviceFamily": device_family,
            "modelIdentifier": model_identifier,
            "remoteIp": remote_ip,
            "caps": caps,
            "commands": commands,
            "pathEnv": live.and_then(|n| n.path_env.clone()),
            "permissions": live.and_then(|n| n.permissions.clone()),
            "connectedAtMs": live.map(|n| n.connected_at_ms),
            "paired": paired,
            "connected": connected
        });

        entries.push((connected, name_key, node_id, value));
    }

    entries.sort_by(|a, b| {
        if a.0 != b.0 {
            return b.0.cmp(&a.0);
        }
        let name_cmp = a.1.cmp(&b.1);
        if name_cmp != std::cmp::Ordering::Equal {
            return name_cmp;
        }
        a.2.cmp(&b.2)
    });

    let nodes: Vec<Value> = entries.into_iter().map(|(_, _, _, value)| value).collect();
    Ok(json!({ "ts": now_ms(), "nodes": nodes }))
}

pub(super) fn handle_node_describe(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;

    let paired = state.node_pairing.get_paired_node(node_id);
    let live = state.node_registry.lock().get(node_id).cloned();

    if paired.is_none() && live.is_none() {
        return Err(error_shape(ERROR_INVALID_REQUEST, "unknown nodeId", None));
    }

    let display_name = live
        .as_ref()
        .and_then(|n| n.display_name.clone())
        .or_else(|| paired.as_ref().and_then(|n| n.display_name.clone()));
    let platform = live
        .as_ref()
        .and_then(|n| n.platform.clone())
        .or_else(|| paired.as_ref().and_then(|n| n.platform.clone()));
    let version = live.as_ref().and_then(|n| n.version.clone());
    let device_family = live.as_ref().and_then(|n| n.device_family.clone());
    let model_identifier = live.as_ref().and_then(|n| n.model_identifier.clone());
    let remote_ip = live.as_ref().and_then(|n| n.remote_ip.clone());
    let caps = live.as_ref().map(|n| n.caps.clone()).unwrap_or_default();
    let mut commands = HashSet::new();
    if let Some(live) = live.as_ref() {
        commands.extend(live.commands.iter().cloned());
    }
    if let Some(paired) = paired.as_ref() {
        commands.extend(paired.commands.iter().cloned());
    }
    let mut commands: Vec<String> = commands.into_iter().collect();
    commands.sort();

    Ok(json!({
        "ts": now_ms(),
        "nodeId": node_id,
        "displayName": display_name,
        "platform": platform,
        "version": version,
        "coreVersion": null,
        "uiVersion": null,
        "deviceFamily": device_family,
        "modelIdentifier": model_identifier,
        "remoteIp": remote_ip,
        "caps": caps,
        "commands": commands,
        "pathEnv": live.as_ref().and_then(|n| n.path_env.clone()),
        "permissions": live.as_ref().and_then(|n| n.permissions.clone()),
        "connectedAtMs": live.as_ref().map(|n| n.connected_at_ms),
        "paired": paired.is_some(),
        "connected": live.is_some()
    }))
}

pub(super) async fn handle_node_invoke(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let params =
        params.ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "params required", None))?;
    let node_id = params
        .get("nodeId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    let command = params
        .get("command")
        .or_else(|| params.get("method"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "command is required", None))?;
    let idempotency_key = params
        .get("idempotencyKey")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "idempotencyKey is required", None))?;
    let timeout_ms = params
        .get("timeoutMs")
        .and_then(|v| v.as_i64())
        .filter(|v| *v >= 0)
        .map(|v| v as u64);
    let params_value = params.get("params").cloned();
    let params_json = match params_value {
        Some(value) => Some(
            serde_json::to_string(&value)
                .map_err(|_| error_shape(ERROR_INVALID_REQUEST, "params not serializable", None))?,
        ),
        None => None,
    };

    let (conn_id, commands) = {
        let registry = state.node_registry.lock();
        let node = registry.get(node_id).ok_or_else(|| {
            error_shape(
                ERROR_UNAVAILABLE,
                "node not connected",
                Some(json!({
                    "details": { "nodeId": node_id, "nodeError": { "code": "NOT_CONNECTED" } }
                })),
            )
        })?;
        (node.conn_id.clone(), node.commands.clone())
    };

    if commands.is_empty() {
        return Err(error_shape(
            ERROR_FORBIDDEN,
            "node did not declare commands",
            Some(json!({ "nodeId": node_id })),
        ));
    }
    if !commands.contains(command) {
        return Err(error_shape(
            ERROR_FORBIDDEN,
            "command not allowlisted",
            Some(json!({ "nodeId": node_id, "command": command })),
        ));
    }

    let invoke_id = Uuid::new_v4().to_string();
    let (responder, receiver) = oneshot::channel();
    {
        let mut registry = state.node_registry.lock();
        registry.insert_pending_invoke(
            invoke_id.clone(),
            PendingInvoke {
                node_id: node_id.to_string(),
                command: command.to_string(),
                responder,
            },
        );
    }

    let mut payload = serde_json::Map::new();
    payload.insert("id".to_string(), json!(invoke_id));
    payload.insert("nodeId".to_string(), json!(node_id));
    payload.insert("command".to_string(), json!(command));
    payload.insert("idempotencyKey".to_string(), json!(idempotency_key));
    if let Some(params_json) = params_json {
        payload.insert("paramsJSON".to_string(), json!(params_json));
    }
    if let Some(timeout_ms) = timeout_ms {
        payload.insert("timeoutMs".to_string(), json!(timeout_ms));
    }

    if !send_event_to_connection(
        state,
        &conn_id,
        "node.invoke.request",
        Value::Object(payload),
    ) {
        state.node_registry.lock().remove_pending_invoke(&invoke_id);
        return Err(error_shape(
            ERROR_UNAVAILABLE,
            "failed to send invoke to node",
            Some(json!({
                "details": {
                    "nodeId": node_id,
                    "command": command,
                    "nodeError": { "code": "UNAVAILABLE" }
                }
            })),
        ));
    }

    let timeout_ms = timeout_ms.unwrap_or(30_000);
    let result = match tokio::time::timeout(Duration::from_millis(timeout_ms), receiver).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => NodeInvokeResult {
            ok: false,
            payload: None,
            payload_json: None,
            error: Some(NodeInvokeError {
                code: Some("UNAVAILABLE".to_string()),
                message: Some("node invoke failed".to_string()),
            }),
        },
        Err(_) => {
            state.node_registry.lock().remove_pending_invoke(&invoke_id);
            return Err(error_shape(
                ERROR_UNAVAILABLE,
                "node invoke timed out",
                Some(json!({
                    "details": { "code": "TIMEOUT", "nodeId": node_id, "command": command }
                })),
            ));
        }
    };

    if !result.ok {
        let error = result.error.unwrap_or(NodeInvokeError {
            code: None,
            message: None,
        });
        return Err(error_shape(
            ERROR_UNAVAILABLE,
            error.message.as_deref().unwrap_or("node invoke failed"),
            Some(json!({
                "details": {
                    "nodeId": node_id,
                    "command": command,
                    "nodeError": {
                        "code": error.code,
                        "message": error.message
                    }
                }
            })),
        ));
    }

    let payload = if let Some(payload_json) = result.payload_json.clone() {
        serde_json::from_str(&payload_json).unwrap_or(Value::Null)
    } else {
        result.payload.unwrap_or(Value::Null)
    };

    Ok(json!({
        "ok": true,
        "nodeId": node_id,
        "command": command,
        "payload": payload,
        "payloadJSON": result.payload_json
    }))
}

pub(super) fn handle_node_invoke_result(
    params: Option<&Value>,
    state: &WsServerState,
    conn: &ConnectionContext,
) -> Result<Value, ErrorShape> {
    // This method is called by nodes to report results of invocations
    // Verify the node is paired and the connection is authorized
    if conn.role != "node" {
        return Err(error_shape(
            ERROR_FORBIDDEN,
            "only node connections can send invoke results",
            None,
        ));
    }

    let invoke_id = params
        .and_then(|v| v.get("id").or_else(|| v.get("invokeId")))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "id is required", None))?;
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    let ok = params
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let payload = params.and_then(|v| v.get("payload")).cloned();
    let payload_json = params
        .and_then(|v| v.get("payloadJSON"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let error = params
        .and_then(|v| v.get("error"))
        .and_then(|v| v.as_object());

    let caller_node_id = conn
        .device_id
        .as_ref()
        .or_else(|| Some(&conn.client.id))
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "node identity required", None))?;
    if caller_node_id != node_id {
        return Err(error_shape(ERROR_INVALID_REQUEST, "nodeId mismatch", None));
    }

    // Verify the node is paired
    if !state.node_pairing.is_paired(node_id) {
        return Err(error_shape(ERROR_NOT_PAIRED, "node not paired", None));
    }

    // Update last seen time
    state.node_pairing.touch_node(node_id);

    let result = NodeInvokeResult {
        ok,
        payload,
        payload_json,
        error: error.map(|err| NodeInvokeError {
            code: err
                .get("code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            message: err
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }),
    };

    let resolved = state
        .node_registry
        .lock()
        .resolve_invoke(invoke_id, node_id, result);
    if !resolved {
        return Ok(json!({ "ok": true, "ignored": true }));
    }

    Ok(json!({ "ok": true }))
}

pub(super) fn handle_node_event(
    params: Option<&Value>,
    state: &WsServerState,
    conn: &ConnectionContext,
) -> Result<Value, ErrorShape> {
    // This method is called by nodes to emit events
    if conn.role != "node" {
        return Err(error_shape(
            ERROR_FORBIDDEN,
            "only node connections can send events",
            None,
        ));
    }

    let caller_node_id = conn
        .device_id
        .as_ref()
        .or_else(|| Some(&conn.client.id))
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "node identity required", None))?;
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .unwrap_or(caller_node_id);
    if node_id != caller_node_id {
        return Err(error_shape(ERROR_INVALID_REQUEST, "nodeId mismatch", None));
    }
    let event = params
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "event is required", None))?;
    let payload = params.and_then(|v| v.get("payload")).cloned();
    let payload_json = params
        .and_then(|v| v.get("payloadJSON"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Verify the node is paired
    if !state.node_pairing.is_paired(node_id) {
        return Err(error_shape(ERROR_NOT_PAIRED, "node not paired", None));
    }

    // Update last seen time
    state.node_pairing.touch_node(node_id);

    // In a full implementation, this would broadcast the event to subscribed
    // operator connections. For now, we acknowledge receipt.
    Ok(json!({
        "ok": true,
        "nodeId": node_id,
        "event": event,
        "hasPayload": payload.is_some() || payload_json.is_some()
    }))
}

pub(super) fn handle_device_pair_list(state: &WsServerState) -> Result<Value, ErrorShape> {
    let (pending_requests, _resolved) = state.device_registry.list_requests();
    let paired_devices = state.device_registry.list_paired_devices();

    let pending = pending_requests
        .iter()
        .map(|req| {
            json!({
                "requestId": req.request_id,
                "deviceId": req.device_id,
                "publicKey": req.public_key,
                "displayName": req.display_name,
                "platform": req.platform,
                "clientId": req.client_id,
                "clientMode": req.client_mode,
                "role": req.role,
                "roles": req.requested_roles,
                "scopes": req.requested_scopes,
                "remoteIp": req.remote_ip,
                "silent": req.silent,
                "isRepair": req.is_repair,
                "ts": req.created_at_ms
            })
        })
        .collect::<Vec<_>>();

    let paired = paired_devices
        .iter()
        .map(|device| {
            json!({
                "deviceId": device.device_id,
                "publicKey": device.public_key,
                "displayName": device.display_name,
                "platform": device.platform,
                "clientId": device.client_id,
                "clientMode": device.client_mode,
                "remoteIp": device.remote_ip,
                "roles": device.roles,
                "scopes": device.scopes,
                "createdAtMs": device.paired_at_ms,
                "approvedAtMs": device.paired_at_ms,
                "lastSeenAtMs": device.last_seen_ms
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({ "pending": pending, "paired": paired }))
}

pub(super) fn handle_device_pair_approve(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let request_id = params
        .and_then(|v| v.get("requestId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "requestId is required", None))?;

    let request = state
        .device_registry
        .get_request(request_id)
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "request not found", None))?;

    let (device, _token) = state
        .device_registry
        .approve_request(
            request_id,
            request.requested_roles,
            request.requested_scopes,
        )
        .map_err(|e| match e {
            devices::DevicePairingError::RequestNotFound => {
                error_shape(ERROR_INVALID_REQUEST, "request not found", None)
            }
            devices::DevicePairingError::RequestAlreadyResolved => {
                error_shape(ERROR_INVALID_REQUEST, "request already resolved", None)
            }
            devices::DevicePairingError::RequestExpired => {
                error_shape(ERROR_INVALID_REQUEST, "request expired", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    broadcast_event(
        state,
        "device.pair.resolved",
        json!({
            "requestId": request_id,
            "deviceId": device.device_id,
            "decision": "approved",
            "ts": now_ms()
        }),
    );

    Ok(json!({
        "requestId": request_id,
        "device": {
            "deviceId": device.device_id,
            "publicKey": device.public_key,
            "displayName": device.display_name,
            "platform": device.platform,
            "clientId": device.client_id,
            "roles": device.roles,
            "scopes": device.scopes,
            "createdAtMs": device.paired_at_ms,
            "approvedAtMs": device.paired_at_ms,
            "lastSeenAtMs": device.last_seen_ms
        }
    }))
}

pub(super) fn handle_device_pair_reject(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let request_id = params
        .and_then(|v| v.get("requestId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "requestId is required", None))?;
    let request = state
        .device_registry
        .reject_request(request_id, None)
        .map_err(|e| match e {
            devices::DevicePairingError::RequestNotFound => {
                error_shape(ERROR_INVALID_REQUEST, "request not found", None)
            }
            devices::DevicePairingError::RequestAlreadyResolved => {
                error_shape(ERROR_INVALID_REQUEST, "request already resolved", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    broadcast_event(
        state,
        "device.pair.resolved",
        json!({
            "requestId": request_id,
            "deviceId": request.device_id,
            "decision": "rejected",
            "ts": now_ms()
        }),
    );

    Ok(json!({
        "requestId": request_id,
        "deviceId": request.device_id
    }))
}

pub(super) fn handle_device_token_rotate(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let device_id = params
        .and_then(|v| v.get("deviceId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "deviceId is required", None))?;
    let role = params
        .and_then(|v| v.get("role"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "role is required", None))?;
    let scopes = params
        .and_then(|v| v.get("scopes"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        });
    let scopes = match scopes {
        Some(scopes) => scopes,
        None => state
            .device_registry
            .latest_token_scopes(device_id, role)
            .or_else(|| {
                state
                    .device_registry
                    .get_paired_device(device_id)
                    .map(|device| device.scopes)
            })
            .unwrap_or_default(),
    };

    let meta = state
        .device_registry
        .rotate_token(device_id, role.to_string(), scopes)
        .map_err(|e| match e {
            devices::DevicePairingError::DeviceNotPaired => {
                error_shape(ERROR_INVALID_REQUEST, "unknown deviceId/role", None)
            }
            devices::DevicePairingError::RoleNotAllowed => {
                error_shape(ERROR_FORBIDDEN, "role not allowed", None)
            }
            devices::DevicePairingError::ScopeNotAllowed => {
                error_shape(ERROR_FORBIDDEN, "scope not allowed", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    Ok(json!({
        "deviceId": device_id,
        "role": role,
        "token": meta.token,
        "scopes": meta.scopes,
        "rotatedAtMs": meta.issued_at_ms
    }))
}

pub(super) fn handle_device_token_revoke(
    params: Option<&Value>,
    state: &WsServerState,
) -> Result<Value, ErrorShape> {
    let device_id = params
        .and_then(|v| v.get("deviceId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "deviceId is required", None))?;
    let role = params
        .and_then(|v| v.get("role"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "role is required", None))?;

    let revoked_at_ms = state
        .device_registry
        .revoke_token(device_id, role)
        .map_err(|e| match e {
            devices::DevicePairingError::DeviceNotPaired => {
                error_shape(ERROR_INVALID_REQUEST, "unknown deviceId/role", None)
            }
            devices::DevicePairingError::TokenInvalid => {
                error_shape(ERROR_INVALID_REQUEST, "unknown deviceId/role", None)
            }
            _ => error_shape(ERROR_UNAVAILABLE, &e.to_string(), None),
        })?;

    Ok(json!({
        "deviceId": device_id,
        "role": role,
        "revokedAtMs": revoked_at_ms
    }))
}

pub(super) fn handle_exec_approvals_get() -> Result<Value, ErrorShape> {
    Ok(json!({
        "approvals": []
    }))
}

pub(super) fn handle_exec_approvals_set(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let approvals = params
        .and_then(|v| v.get("approvals"))
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "approvals is required", None))?;
    Ok(json!({
        "ok": true,
        "approvals": approvals.clone()
    }))
}

pub(super) fn handle_exec_approvals_node_get(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    Ok(json!({
        "nodeId": node_id,
        "approvals": []
    }))
}

pub(super) fn handle_exec_approvals_node_set(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let node_id = params
        .and_then(|v| v.get("nodeId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "nodeId is required", None))?;
    let approvals = params
        .and_then(|v| v.get("approvals"))
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "approvals is required", None))?;
    Ok(json!({
        "ok": true,
        "nodeId": node_id,
        "approvals": approvals.clone()
    }))
}

pub(super) fn handle_exec_approval_request(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let command = params
        .and_then(|v| v.get("command"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "command is required", None))?;
    Ok(json!({
        "requestId": Uuid::new_v4().to_string(),
        "command": command,
        "status": "pending"
    }))
}

pub(super) fn handle_exec_approval_resolve(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let request_id = params
        .and_then(|v| v.get("requestId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "requestId is required", None))?;
    let approved = params
        .and_then(|v| v.get("approved"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Ok(json!({
        "ok": true,
        "requestId": request_id,
        "approved": approved
    }))
}

pub(super) fn handle_usage_status() -> Result<Value, ErrorShape> {
    let cfg = config::load_config().unwrap_or(Value::Object(serde_json::Map::new()));
    let tracking = cfg
        .get("usage")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    Ok(json!({
        "enabled": true,
        "tracking": tracking
    }))
}

pub(super) fn handle_usage_cost(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let session_key = params
        .and_then(|v| v.get("sessionKey"))
        .and_then(|v| v.as_str());
    let days = params
        .and_then(|v| v.get("days"))
        .and_then(|v| v.as_i64())
        .unwrap_or(30)
        .max(1);
    Ok(json!({
        "days": days,
        "sessionKey": session_key,
        "inputTokens": 0,
        "outputTokens": 0,
        "totalCost": 0.0
    }))
}

pub(super) fn handle_logs_tail(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let limit = params
        .and_then(|v| v.get("limit"))
        .and_then(|v| v.as_i64())
        .map(|v| clamp_i64(v, 1, LOGS_MAX_LIMIT as i64) as usize)
        .unwrap_or(LOGS_DEFAULT_LIMIT);
    let max_bytes = params
        .and_then(|v| v.get("maxBytes"))
        .and_then(|v| v.as_i64())
        .map(|v| clamp_i64(v, 1, LOGS_MAX_BYTES as i64) as usize)
        .unwrap_or(LOGS_DEFAULT_MAX_BYTES);
    let cursor = params
        .and_then(|v| v.get("cursor"))
        .and_then(|v| v.as_i64())
        .filter(|v| *v >= 0)
        .map(|v| v as u64);

    let configured = resolve_log_file_path();
    let file = resolve_log_file(&configured);
    let result = read_log_slice(&file, cursor, limit, max_bytes)?;

    Ok(json!({
        "file": file.to_string_lossy(),
        "cursor": result.cursor,
        "size": result.size,
        "lines": result.lines,
        "truncated": result.truncated,
        "reset": result.reset
    }))
}

pub(super) fn handle_last_heartbeat() -> Result<Value, ErrorShape> {
    Ok(json!({
        "ts": now_ms(),
        "lastHeartbeat": null
    }))
}

pub(super) fn handle_set_heartbeats(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let enabled = params
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    Ok(json!({
        "ok": true,
        "enabled": enabled
    }))
}

pub(super) fn handle_wake(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let target = params
        .and_then(|v| v.get("target"))
        .and_then(|v| v.as_str());
    Ok(json!({
        "ok": true,
        "target": target
    }))
}

pub(super) fn handle_send(
    state: &WsServerState,
    params: Option<&Value>,
    _conn: &ConnectionContext,
) -> Result<Value, ErrorShape> {
    let to = params
        .and_then(|v| v.get("to"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "to is required", None))?;
    let message = params
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "message is required", None))?;
    let idempotency_key = params
        .and_then(|v| v.get("idempotencyKey"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "idempotencyKey is required", None))?;
    let channel = params
        .and_then(|v| v.get("channel"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("default");

    let mut metadata = messages::outbound::MessageMetadata::default();
    metadata.recipient_id = Some(to.to_string());

    let outbound = messages::outbound::OutboundMessage::new(
        channel,
        messages::outbound::MessageContent::text(message),
    )
    .with_metadata(metadata);
    let ctx = messages::outbound::OutboundContext::new().with_trace_id(idempotency_key);

    let queued = state
        .message_pipeline
        .queue(outbound.clone(), ctx)
        .map_err(|e| error_shape(ERROR_UNAVAILABLE, &format!("queue failed: {}", e), None))?;

    Ok(json!({
        "ok": true,
        "runId": idempotency_key,
        "messageId": outbound.id.0,
        "channel": outbound.channel_id,
        "queuePosition": queued.queue_position,
        "to": to,
        "message": message
    }))
}

pub(super) fn handle_system_presence(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let presence = params
        .and_then(|v| v.get("presence"))
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "presence is required", None))?;
    Ok(json!({
        "ok": true,
        "presence": presence.clone()
    }))
}

pub(super) fn handle_system_event(params: Option<&Value>) -> Result<Value, ErrorShape> {
    let event = params
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| error_shape(ERROR_INVALID_REQUEST, "event is required", None))?;
    Ok(json!({
        "ok": true,
        "event": event
    }))
}
