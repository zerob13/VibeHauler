# VibeHauler

A local-first cleaner for AI agent clients.

VibeHauler scans Claude Code, Codex, Cursor, Cherry Studio, Alma, DeepChat, OpenCode, and more. It is designed to review sessions, inspect disk usage, and clean safely.

## Status

This repository is currently a scaffold. The project structure, module boundaries, and documentation are in place; scanner, parser, and cleaner logic will be implemented after the contracts settle.

## Naming

| Surface | Name |
|---|---|
| Product | VibeHauler |
| CLI binary | `vhaul` |
| Rust crate | `vibe-hauler` |
| Repository | `VibeHauler` |
| Tagline | Haul away your agent clutter. |

## Workspace

```txt
crates/
├── vibe-hauler              # CLI binary
├── vibe-hauler-core         # shared data contracts
├── vibe-hauler-config       # config contracts
├── vibe-hauler-discovery    # path discovery contracts
├── vibe-hauler-adapters     # app adapter contracts
├── vibe-hauler-parsers      # parser contracts
├── vibe-hauler-cleaner      # cleanup/restore contracts
├── vibe-hauler-redaction    # redaction contracts
└── vibe-hauler-fixtures     # test fixture helpers
```

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo run -p vibe-hauler -- --help
```

## Documentation

Start with [docs/README.md](docs/README.md).

