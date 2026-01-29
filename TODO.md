# TODO

## Status Legend

- [x] Complete — fully implemented and tested
- [~] Partial — handler exists with real logic, but some aspects stubbed
- [ ] Not started — no implementation

## Port Status Overview

| Category | Completion | Notes |
|----------|-----------|-------|
| Infrastructure (WS, HTTP, config, logging) | ~99% | Production-quality, TLS, mDNS, config reload, CLI, Tailscale, remote gateway |
| Security (auth, credentials, rate limiting, encryption) | ~99% | Real, reviewed, tool allowlists, OAuth profiles, encrypted secrets, audit logging, backup encryption |
| Data storage (sessions, cron, usage, nodes, devices) | ~99% | Real, tested, file-backed, retention cleanup |
| Core functionality (agent/LLM, channel delivery, cron execution) | ~99% | Multi-provider (Anthropic/OpenAI/Ollama/Gemini/Bedrock), built-in tools, channel tools, media analysis, link understanding |

## Infrastructure (Complete)

- [x] Config parsing — JSON5, `$include`, env substitution, caching
- [x] Auth — token, password, loopback, Tailscale whois, device identity, timing-safe
- [x] Credential storage — macOS Keychain, Linux Secret Service, Windows Credential Manager
- [x] Logging — structured tracing, ring buffer, JSON/plaintext, log tail streaming
- [x] Rate limiting — per-IP, per-endpoint, 429 responses
- [x] CSRF protection — token generation/validation
- [x] Security headers — CSP, HSTS, X-Content-Type-Options
- [x] HTTP server — static files, routing, health check, hooks, OpenAI-compatible chat endpoint (wired to LLM provider)
- [x] Config control HTTP endpoint — PATCH config with validation, optimistic concurrency, persistence
- [x] WebSocket server — JSON-RPC dispatch, auth, handshake, broadcast
- [x] Media pipeline — SSRF-protected fetch, temp storage, cleanup, image/audio analysis
- [x] Plugin runtime — wasmtime, capability enforcement, sandbox, real WASM export calls
- [x] Plugin loader — WASM metadata extraction, manifest derivation, kind detection from exports
- [x] Hooks — webhook handler, token auth, mappings
- [x] Server startup harness — `main.rs` with tokio runtime, config loading, HTTP+WS bind, graceful shutdown
- [x] TLS — self-signed cert auto-generation, configurable cert/key paths, SHA-256 fingerprint, `axum-server` rustls binding
- [x] mDNS discovery — `_moltbot._tcp.local.` Bonjour broadcast, off/minimal/full modes, graceful shutdown
- [x] Config defaults — 7-section defaults pipeline, deep-merge with user-wins semantics, partial config support
- [x] Config hot reload — file watcher (notify), SIGHUP handler, `config.reload` WS method, debounce, validation
- [x] CLI — `start`, `config`, `status`, `logs`, `version`, `backup`, `restore`, `reset`, `setup`, `pair`, `update` subcommands via clap
- [x] Network binding modes — loopback/lan/auto/tailnet/custom with interface detection
- [x] Link understanding — URL extraction, SSRF-safe fetching, HTML-to-text, LRU cache
- [x] Tailscale serve/funnel — auto-configure Tailscale serve (LAN proxy) or funnel (public internet), lifecycle management, teardown on shutdown

## Security Features

- [x] Encrypted config secrets — `src/config/secrets.rs` AES-256-GCM at-rest encryption with PBKDF2 key derivation, `enc:v1:` prefix format, seal/resolve config tree operations (41 tests)
- [x] Structured audit logging — `src/logging/audit.rs` append-only JSONL audit trail with 17 event types, file rotation at 50MB, `recent_audit_events()` tail reader (29 tests)
- [x] Secret masking in logs — `src/logging/redact.rs` regex-based redaction of API keys, bearer tokens, query params; JSON key name matching; `RedactedDisplay<T>` wrapper (22 tests)
- [x] Backup encryption — `src/cli/backup_crypto.rs` AES-256-GCM with PBKDF2-HMAC-SHA256 (600K iterations), `CRPC_ENC` format with magic/version/salt/nonce header (17 tests)
- [x] Prometheus metrics — `src/server/metrics.rs` counter, gauge, histogram types with atomic backing, `/metrics` text exposition endpoint, 10 standard metrics (31 tests)

## Core Functionality

- [x] Agent/LLM execution engine — `src/agent/` with Anthropic + OpenAI + Ollama + Gemini + Bedrock streaming, MultiProvider dispatch, tool dispatch, context building, cancellation token, per-chunk stream timeout
- [x] Built-in agent tools — 10 tools: current_time, web_fetch, memory_read/write/list, message_send, session_list/read, config_read, math_eval
- [x] Channel-specific agent tools — 15 tools gated by channel: Telegram (edit, delete, pin, reply_markup, send_photo), Discord (reaction, embed, thread, edit, delete), Slack (blocks, ephemeral, reaction, update, delete)
- [x] Agent tool allowlists — AllowAll/AllowList/DenyList policy with enforcement at definition filtering and dispatch gating
- [x] Media understanding — Anthropic + OpenAI image analysis, OpenAI Whisper audio transcription, result caching
- [x] Channel message delivery — `src/messages/delivery.rs` delivery loop spawned at startup, drains queue, invokes channel plugins
- [x] Cron background execution — `src/cron/tick.rs` tick loop (10s interval) spawned at startup, payload execution via `src/cron/executor.rs`
- [x] GDPR data portability — `sessions.export_user` exports all user sessions/histories, resilient to per-session failures with warnings
- [x] GDPR right to erasure — `sessions.purge_user` deletes all user data (best-effort), reports deleted/total counts
- [x] Session retention — automatic cleanup via background timer with configurable interval and retention days
- [x] Session scoping — per-sender/global/per-channel-peer session isolation with daily/idle/manual reset policies
- [x] Exec approvals persistence — `exec.approvals.get/set` with atomic file I/O and SHA256 optimistic concurrency

## WS Method Handlers

### Complete (real logic)

- [x] `sessions.*` — CRUD, history, archiving, compaction, archive protection, GDPR export/purge
- [x] `cron.*` — scheduler CRUD, events, run history, 500-job limit, background tick, payload execution
- [x] `exec.approvals.*` — file-backed store with atomic writes, SHA256 concurrency control, workflow, wait/resolve via oneshot channels
- [x] `usage.status` / `usage.cost` — token counting, cost calculation, model pricing
- [x] `last-heartbeat` / `set-heartbeats` — heartbeat tracking with real values
- [x] `voicewake.*` — config, multiple wake words, thresholds
- [x] `talk.mode` — state machine (off, push-to-talk, voice-activated, continuous)
- [x] `wizard.*` — onboarding state machine
- [x] `logs.tail` — real log source, ring buffer, streaming
- [x] `system-presence` / `system-event` — presence tracking, event broadcast
- [x] `wake` — wake trigger
- [x] `channels.status` / `channels.logout` — registry queries
- [x] `node.*` / `device.*` — pairing state machines, token management
- [x] `config.get` — config reading
- [x] `config.set` / `config.apply` / `config.patch` — config writing with validation, optimistic concurrency, JSON merge-patch
- [x] `config.reload` — hot/hybrid reload with validation, broadcasts `config.changed` event
- [x] `agent` / `agent.wait` — LLM execution with streaming, tool orchestration, cancellation via `CancellationToken`
- [x] `agent.identity.get` — reads from config `agents.list`, supports explicit `agentId` lookup
- [x] `chat.send` / `chat.abort` — queues messages, spawns agent run, cancellation
- [x] `models.list` — reads from config `agents.defaults.models` and `models.providers`
- [x] `agents.list` — reads from config `agents.list`
- [x] `send` — queues outbound message, delivery loop invokes channel plugins, delivery result fields wired through
- [x] `skills.status` / `skills.bins` — reads `skills.entries` from config, scans managed bins directory
- [x] `skills.install` / `skills.update` — WASM download, magic byte validation, atomic writes, manifest tracking

### Partial (handler exists, some aspects stubbed)

- [~] `tts.convert` / `tts.speak` — config, provider/voice selection, rate/pitch/volume all work; OpenAI TTS wired (returns null audio when no API key)
- [~] `update.run` / `update.check` — real GitHub Releases API check, state tracking, install stub

## Node Parity Gaps

- [x] `sessions.compact` — archiving with archived path
- [x] `last-heartbeat` — tracks heartbeat, returns real value
- [x] `send` — delivery result fields wired through from channel plugins
- [x] Node event routing — `node.event` → operator broadcast mapping
- [x] Node pairing payload parity — accept/record version, coreVersion, uiVersion, deviceFamily, modelIdentifier, caps, permissions, remoteIp, silent, isRepair
- [x] Node list/describe integration — merge paired metadata into output

## Remaining Code TODOs

- [ ] Gateway WS transport — actual `tokio-tungstenite` WebSocket client connection in `connect_to_gateway` (currently returns mock Connected state)

## Feature Gaps (clawdbot parity)

Priority order reflects what blocks real-world usage soonest.

### P0 — Required for LAN deployment

- [x] **TLS termination** — `src/tls/mod.rs`
- [x] **mDNS service discovery** — `src/discovery/mod.rs`
- [x] **Config defaults application** — `src/config/defaults.rs`

### P1 — Required for real agent usage

- [x] **Multiple LLM providers** — Anthropic, OpenAI, Ollama, Google Gemini, and AWS Bedrock implemented with `MultiProvider` dispatch
- [x] **Built-in agent tools** — 10 core tools in `src/agent/builtin_tools.rs`, wired into `ToolsRegistry`
- [x] **Media understanding pipeline** — `src/media/analysis.rs` with Anthropic + OpenAI image analysis, Whisper transcription, caching

### P2 — Required for multi-user / multi-channel deployments

- [x] **Session scoping and reset rules** — `src/sessions/scoping.rs` with per-sender/global/per-channel-peer, daily/idle/manual reset
- [x] **Config reload modes** — `src/config/watcher.rs` with hot/hybrid/off, file watcher, SIGHUP, WS method
- [x] **Channel-specific agent tools** — `src/agent/channel_tools.rs` with 15 tools: Telegram (edit, delete, pin, reply_markup, send_photo), Discord (reaction, embed, thread, edit, delete), Slack (blocks, ephemeral, reaction, update, delete), gated by `message_channel`

### P3 — Needed for production operations

- [x] **Remote gateway support** — `src/gateway/mod.rs` with GatewayRegistry, TOFU fingerprint verification, SSH tunnel transport, direct WebSocket, config integration, lifecycle management
- [x] **CLI subcommands** — `start`, `config`, `status`, `logs`, `version`, `backup`, `restore`, `reset`, `setup`, `pair`, `update` implemented via clap
- [x] **Auth profiles** — `src/auth/profiles.rs` with OAuth2 PKCE flow for Google/GitHub/Discord, ProfileStore, token refresh, config integration
- [x] **Network binding modes** — loopback/lan/auto/tailnet/custom in `src/server/bind.rs`

### P4 — Nice to have

- [x] **Link understanding pipeline** — `src/links/mod.rs` with URL extraction, SSRF-safe fetching, HTML-to-text, LRU cache
- [x] **Tailscale serve/funnel modes** — `src/tailscale/mod.rs` with serve/funnel/off modes, CLI wrapper, lifecycle management, teardown on shutdown
- [x] **Agent tool allowlists** — `src/agent/tool_policy.rs` with AllowAll/AllowList/DenyList, enforcement at definition filtering and dispatch
- [x] **Automatic session retention cleanup** — `src/sessions/retention.rs` with background timer, configurable interval/retention days

## Security Superiority Roadmap (beyond parity)

### Completed

- [x] **Encrypted config secrets** — AES-256-GCM at-rest encryption for API keys and tokens in config files
- [x] **Structured audit logging** — Append-only JSONL audit trail for all security-relevant operations
- [x] **Secret masking in logs** — Automatic redaction of secrets from all log output
- [x] **Backup encryption** — AES-256-GCM encrypted backup archives with PBKDF2 key derivation
- [x] **Prometheus metrics** — `/metrics` endpoint for monitoring and alerting

### Planned

- [ ] **Prompt guard** — Three-layer safety system for agent prompts:
  - Pre-flight analysis: Static checks on system prompts (injection patterns, privilege escalation vectors, data exfiltration markers)
  - Post-flight filtering: Output sanitization (PII leak detection, credential exposure, harmful instruction detection)
  - Config-time linting: Warn when `agents.list` entries contain risky patterns (unrestricted tool access, no output limits, overly broad instructions)
- [ ] **Gateway WS transport** — Real WebSocket client connection for remote gateway nodes via `tokio-tungstenite`
- [ ] **Content security policy for agent outputs** — Sandboxed HTML/markdown rendering with CSP
- [ ] **Agent execution sandboxing** — Resource limits (CPU, memory, network) per agent run
- [ ] **Cryptographic session integrity** — HMAC-signed session files to detect tampering
- [ ] **mTLS for gateway-to-gateway** — Mutual TLS authentication for multi-node deployments
- [ ] **Capability-based plugin permissions** — Fine-grained permissions model for WASM plugins
- [ ] **deny.toml** — `cargo-deny` configuration for dependency auditing

## Missing Non-Code Artifacts

- [x] Example config file — `config.example.json5`
- [x] Dockerfile — multi-stage build with health check
- [x] Release workflow — `.github/workflows/release.yml` (tag-triggered, cross-platform binary builds)
- [x] CONTRIBUTING.md — development guide, prerequisites, workflow, code style, testing, PR guidelines
- [x] CHANGELOG.md — Keep a Changelog format, version 0.1.0 (Unreleased)

## Tests

- [x] 1,849 tests passing (`cargo test --lib`)
- [x] Pre-commit hooks: `cargo fmt --check` + `cargo clippy -- -D warnings`
- [x] Pre-push hooks: full test suite via `cargo nextest run`
- [x] CI: fmt, clippy -D warnings, nextest, MSRV check (1.75.0), cargo-deny, cross-platform (macOS, Windows, Linux)
- [x] Golden trace tests — 38 insta snapshot tests covering all WS method categories in `src/server/ws/golden_tests.rs`
- [ ] Cross-platform CI tests (currently builds only on macOS/Windows, tests only on Linux)
