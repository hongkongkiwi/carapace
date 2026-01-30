# carapace Documentation

A security-focused, open-source personal AI assistant — hardened alternative to
openclaw / moltbot / clawdbot.

## Architecture & Security

- [Architecture](architecture.md) — Component diagrams, request flows, design decisions
- [Security](security.md) — Threat model, trust boundaries, implementation checklist

## Protocol Reference

- [WebSocket Protocol](protocol/websocket.md) — JSON-RPC over WebSocket, methods, events
- [HTTP API](protocol/http.md) — REST endpoints, hooks, OpenAI compatibility
- [Pairing](protocol/pairing.md) — Node and device pairing flows
- [Configuration](protocol/config.md) — Config file format, environment variables
- [Credentials](protocol/credentials.md) — Secret storage and migration

## Quick Links

| Topic | File | Description |
|-------|------|-------------|
| Trust boundaries | [security.md](security.md#security-layers) | 4-layer defense model |
| WS handshake | [websocket.md](protocol/websocket.md#connection-lifecycle) | Connect flow, auth methods |
| HTTP hooks | [http.md](protocol/http.md#hooks) | Webhook configuration |
| Token security | [pairing.md](protocol/pairing.md#token-security) | SHA-256 hashing, constant-time comparison |
| Rate limiting | [security.md](security.md#rate-limiting) | Default limits per endpoint |
| Session storage | [architecture.md](architecture.md#key-files) | JSONL history, compaction |
