//! Session, agent, and chat handlers.

use serde_json::{json, Value};

use super::super::*;

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

fn parse_session_status(raw: &str) -> Option<sessions::SessionStatus> {
    match raw {
        "active" => Some(sessions::SessionStatus::Active),
        "paused" => Some(sessions::SessionStatus::Paused),
        "archived" => Some(sessions::SessionStatus::Archived),
        "compacting" => Some(sessions::SessionStatus::Compacting),
        _ => None,
    }
}

fn role_to_string(role: sessions::MessageRole) -> &'static str {
    match role {
        sessions::MessageRole::User => "user",
        sessions::MessageRole::Assistant => "assistant",
        sessions::MessageRole::System => "system",
        sessions::MessageRole::Tool => "tool",
    }
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
