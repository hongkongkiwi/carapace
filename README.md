# carapace

> **This project is under active development and is not yet production-ready.**
> Core functionality works and 1,900+ tests pass, but hardening work remains
> before this should be used outside of development/testing.

A secure, stable Rust alternative to moltbot - for when your molt needs a hard shell.

## What Works Today

- **Multi-provider LLM engine** — Anthropic, OpenAI, Ollama, Google Gemini, AWS Bedrock with streaming, tool dispatch, and cancellation
- **25 agent tools** — 10 built-in (web fetch, memory, sessions, math, etc.) + 15 channel-specific (Telegram, Discord, Slack)
- **WASM plugin runtime** — wasmtime-based with capability enforcement and sandboxing
- **Security hardening** — encrypted config secrets, SSRF protection with DNS rebinding defense, exfiltration-sensitive tool policy, audit logging, secret masking, backup encryption
- **Infrastructure** — TLS, mDNS discovery, config hot-reload, Tailscale integration, Prometheus metrics
- **Full CI pipeline** — clippy, fmt, nextest (cross-platform), cargo-deny, gitleaks, trivy, hadolint, cargo-geiger

## What's Still Needed for Production

- **Resource safety** — Connection limits, WASM CPU budgets, JSON depth limits (in progress)
- **Operational tooling** — Deep health checks, config schema validation, per-user rate limiting
- **Data integrity** — File locking for concurrent access, atomic writes audit
- **Gateway transport** — Real WebSocket client for remote gateway connections (currently stubbed)
- **Prompt injection defense** — Multi-layer prompt guard system

## Requirements

- Rust 1.87+ (2021 edition, MSRV enforced in CI)
- For WASM plugins: wasmtime 18+

### Recommended Tools

```bash
# Task runner (like make, but better)
cargo install just

# Faster test runner with better output
cargo install cargo-nextest

# File watcher for development (optional)
cargo install cargo-watch

# Code coverage (optional)
cargo install cargo-tarpaulin
```

## Development

This project uses [just](https://github.com/casey/just) as a task runner. Run `just` to see available commands:

```bash
just          # Show all available recipes
just build    # Build the project
just test     # Run tests with nextest
just lint     # Run clippy
just check    # Run lint + fmt-check + test
just watch    # Watch for changes and run tests
```

## Building

```bash
cargo build
# or
just build
```

## Testing

Using [cargo-nextest](https://nexte.st/) (recommended - faster, better output):
```bash
cargo nextest run
# or
just test
```

Using standard cargo test:
```bash
cargo test
# or
just test-cargo
```

Run specific tests:
```bash
just test-one test_name
```

With coverage:
```bash
just test-coverage
# or
cargo tarpaulin --out Html
```

## Linting

```bash
cargo clippy
cargo fmt --check
# or
just lint
just fmt-check
```

## Project Structure

```
src/
├── agent/          # LLM execution engine (Anthropic streaming, tool dispatch, context)
├── auth/           # Authentication (tokens, passwords, Tailscale whois, loopback)
├── channels/       # Channel registry
├── config/         # JSON5 config parsing, $include, env substitution
├── credentials/    # Credential storage (macOS Keychain, Linux Secret Service, Windows)
├── cron/           # Cron scheduler, background tick loop, payload execution
├── devices/        # Device pairing state machine
├── exec/           # Exec approval workflow (request, wait, resolve)
├── hooks/          # Webhook mappings
├── logging/        # Structured logging (ring buffer, JSON/plaintext)
├── media/          # SSRF-protected media fetch/store
├── messages/       # Outbound message pipeline and delivery loop
├── nodes/          # Node pairing state machine
├── plugins/        # WASM plugin runtime (wasmtime, capability enforcement)
├── server/         # HTTP + WebSocket server, handlers, rate limiting
├── sessions/       # Session storage (JSONL history, compaction, archiving)
└── usage/          # Token counting, cost calculation, model pricing

docs/
├── architecture.md # Component diagrams
├── security.md     # Threat model
├── protocol/       # Protocol specifications
└── refactor/       # Migration planning (historical)

tests/
├── golden/         # Golden test traces
└── *.rs            # Integration tests
```

## Documentation

See [docs/README.md](docs/README.md) for full documentation index.

## License

MIT - see [LICENSE](LICENSE)
