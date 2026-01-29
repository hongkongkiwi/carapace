# Gap Analysis: moltbot vs carapace

## Overview

| Aspect | moltbot | carapace | Gap |
|--------|---------|----------|-----|
| Language | Python | Rust | Rewrite in progress |
| Architecture | Monolithic | Modular | Complete redesign |
| Plugin System | Limited | WASM-based | Significant improvement |
| Storage | SQLite | JSONL + Files | Better durability |

## Feature Comparison

### Core Messaging

| Feature | moltbot | carapace | Status |
|---------|---------|----------|--------|
| Telegram | Yes | Yes | Feature complete |
| Discord | Yes | Yes | Feature complete |
| Slack | Yes | Yes | Feature complete |
| WhatsApp | Partial | Yes | Implemented |
| iMessage | No | Yes | Implemented |
| Matrix | No | Yes | Implemented |
| Google Chat | No | Yes | Implemented |
| LINE | No | Yes | Implemented |
| WebChat | No | Yes | Implemented |
| Zalo | No | Yes | Implemented |
| Voice Calls | No | Yes | Implemented |
| Signal | No | Placeholder | Needs API integration |
| Skype | No | Placeholder | Needs API integration |
| Microsoft Teams | No | Placeholder | Needs API integration |

### Automation & Flows

| Feature | moltbot | carapace | Status |
|---------|---------|----------|--------|
| Basic Triggers | Yes | Yes | Feature complete |
| Conditions | Yes | Yes | 14+ operators |
| Command Handling | Yes | Yes | Feature complete |
| Scheduled Tasks | Yes | Yes | Cron-based |
| Webhook Triggers | Yes | Yes | Just added |
| Parallel Actions | No | Yes | Just added |
| Sub-flows | No | Yes | Stub implemented |
| Template Engine | Basic | Yes | Variable substitution |

### Agent System

| Feature | moltbot | carapace | Status |
|---------|---------|----------|--------|
| Agent Definition | Yes | Partial | Skills module exists |
| Tool Registry | Yes | Yes | Plugin-based |
| Context Management | Basic | Yes | Session-based |
| Streaming Responses | No | Yes | OpenAI compat |
| Tool Execution | Yes | Yes | Approval workflow |

### Storage & State

| Feature | moltbot | carapace | Status |
|---------|---------|----------|--------|
| Session History | SQLite | JSONL | Better durability |
| Compaction | Manual | Automatic | Auto-archiving |
| Credential Storage | SQLite | OS Keychain | More secure |
| Configuration | JSON | JSON5 | Env support, includes |

### Security

| Feature | moltbot | carapace | Status |
|---------|---------|----------|--------|
| Token Auth | Yes | Yes | SHA-256 hashing |
| Rate Limiting | Basic | Yes | Per-channel |
| SSRF Protection | Limited | Yes | Media pipeline |
| Credential Encryption | Basic | OS-level | Keychain/SS/Windows |
| Plugin Sandboxing | None | WASM | wasmtime isolation |

### API & Access

| Feature | moltbot | carapace | Status |
|---------|---------|----------|--------|
| WebSocket API | Yes | Yes | JSON-RPC |
| HTTP Endpoints | Yes | Yes | REST + OpenAI |
| OpenAI Compatibility | No | Yes | /v1/chat/completions |
| Control UI | Basic | Yes | Leptos-based |
| Mobile Access | Via API | Via API | Same |

## Key Architecture Differences

### moltbot (Legacy)
- Python monolithic application
- SQLite for all persistence
- Blocking I/O model
- Basic plugin system (Python modules)
- Tightly coupled components

### carapace (New)
- Rust async application (tokio)
- JSONL + OS keychain for persistence
- Non-blocking I/O throughout
- WASM-based plugin runtime (wasmtime)
- Loosely coupled via message passing
- Thread-safe with parking_lot locks

## Implementation Gaps

### High Priority

1. **Signal/Skype/Teams Integration**
   - Current: Placeholder implementations
   - Need: Actual API integration
   - Effort: Medium (external APIs)

2. **Agent Executor**
   - Current: Skills module exists, agent executor stub
   - Need: Full LLM orchestration
   - Effort: High

3. **Message Delivery Worker**
   - Current: Outbound pipeline exists
   - Need: Background delivery loop
   - Effort: Medium

### Medium Priority

4. **Plugin Ecosystem**
   - Current: WASM runtime exists
   - Need: Plugin SDK, sample plugins
   - Effort: High

5. **Voice Wake Word**
   - Current: Module exists
   - Need: Integration with audio pipeline
   - Effort: Medium

6. **Control UI**
   - Current: Leptos setup, disabled due to router API
   - Need: leptos_router 0.8 migration
   - Effort: Low

### Low Priority

7. **More Channels**
   - WhatsApp Business API
   - WeChat
   - Viber
   - Facebook Messenger

8. **Advanced Flow Features**
   - Flow debugging/step-through
   - Flow analytics
   - Conditional branching UI

## Recommendations

### Immediate (This Sprint)

1. **Complete Skills Module** - Agent executor integration
2. **Fix Control UI** - leptos_router migration
3. **API Integration** - Pick one of Signal/Skype/Teams to fully implement

### Short Term (1-2 Months)

1. **Message Delivery Worker** - Background processing
2. **Plugin SDK** - Developer experience
3. **Control UI Features** - Dashboard, channel management

### Long Term (3-6 Months)

1. **Complete Channel Coverage** - Signal, Skype, Teams
2. **Voice Integration** - Wake word, TTS pipeline
3. **Plugin Marketplace** - Community plugins

## Conclusion

carapace is a significant improvement over moltbot in terms of:
- Security (WASM sandbox, OS keychain)
- Performance (Rust async)
- Modularity (clear component boundaries)
- Extensibility (WASM plugins)

Key gaps to address:
1. Remaining channel integrations (Signal, Skype, Teams)
2. Agent/flow system completion
3. Control UI enablement
4. Plugin ecosystem development

The rewrite is well-structured and maintainable. Once the remaining high-priority items are completed, carapace will be a drop-in replacement for moltbot with additional capabilities.
