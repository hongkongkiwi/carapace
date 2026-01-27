# rusty-clawd

Rust implementation of the Clawdbot gateway.

## Status

Work in progress. See [docs/refactor/implementation-plan.md](docs/refactor/implementation-plan.md) for current status.

## Requirements

- Rust 1.75+ (2021 edition)
- For WASM plugins: wasmtime 18+

## Building

```bash
cargo build
```

## Testing

```bash
cargo test
```

With coverage:
```bash
cargo tarpaulin --out Html
```

## Linting

```bash
cargo clippy
cargo fmt --check
```

## Project Structure

```
src/
├── auth/           # Authentication (tokens, passwords, loopback)
├── channels/       # Channel registry
├── credentials/    # Credential storage
├── devices/        # Device pairing
├── hooks/          # Webhook mappings
├── logging/        # Structured logging
├── media/          # Media fetch/store
├── messages/       # Outbound messages
├── nodes/          # Node pairing
├── plugins/        # WASM plugin runtime
├── server/         # HTTP + WebSocket server
└── sessions/       # Session storage

docs/
├── architecture.md # Component diagrams
├── security.md     # Threat model
└── protocol/       # Protocol specifications

tests/
├── golden/         # Golden test traces
└── *.rs            # Integration tests
```

## Documentation

See [docs/README.md](docs/README.md) for full documentation index.

## License

MIT - see [LICENSE](LICENSE)
