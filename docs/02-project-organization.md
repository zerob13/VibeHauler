# Project Organization

This document defines how the future Rust repo should be organized once implementation starts.

## Repository Layout

```txt
vibe-hauler/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── docs/
├── crates/
│   ├── vibe-hauler/              # CLI package, installs bin/vhaul
│   ├── vibe-hauler-core/         # shared models, risk, plan contracts
│   ├── vibe-hauler-config/       # config paths, config loading, defaults
│   ├── vibe-hauler-discovery/    # cross-platform app root discovery
│   ├── vibe-hauler-adapters/     # app-specific inventory/session adapters
│   ├── vibe-hauler-parsers/      # JSONL/Markdown/SQLite/LevelDB parsers
│   ├── vibe-hauler-cleaner/      # backup, trash, quarantine, restore
│   ├── vibe-hauler-redaction/    # secret masking and preview policy
│   └── vibe-hauler-fixtures/     # test fixture builders, publish=false
├── fixtures/
├── tests/
├── xtask/
├── packaging/
│   ├── homebrew/
│   └── npm/
└── .github/
    └── workflows/
```

## Package Naming

| Purpose | Cargo package | Binary / Library |
|---|---|---|
| CLI | `vibe-hauler` | binary `vhaul` |
| Core contracts | `vibe-hauler-core` | library |
| Config | `vibe-hauler-config` | library |
| Discovery | `vibe-hauler-discovery` | library |
| Adapters | `vibe-hauler-adapters` | library |
| Parsers | `vibe-hauler-parsers` | library |
| Cleaner | `vibe-hauler-cleaner` | library |
| Redaction | `vibe-hauler-redaction` | library |
| Fixture helpers | `vibe-hauler-fixtures` | library, `publish = false` |

The public crate on crates.io should be `vibe-hauler`, and it should install the `vhaul` binary. Internal crates can stay unpublished until there is a reason to expose stable library APIs.

## Ownership Boundaries

| Folder | Owner Contract |
|---|---|
| `crates/vibe-hauler` | command UX only |
| `crates/vibe-hauler-core` | stable shared data model |
| `crates/vibe-hauler-discovery` | path resolution and root confidence |
| `crates/vibe-hauler-adapters` | client-specific logic |
| `crates/vibe-hauler-parsers` | format-specific parsing |
| `crates/vibe-hauler-cleaner` | all filesystem mutations |
| `crates/vibe-hauler-redaction` | privacy and secret masking |
| `packaging/` | release channel metadata |
| `fixtures/` | fake homes, app data samples, DB fixtures |

## Feature Flags

Recommended flags:

| Feature | Default | Purpose |
|---|---:|---|
| `sqlite` | yes | SQLite readonly/introspection |
| `leveldb` | yes | LevelDB/localStorage parsing |
| `indexeddb-subset` | yes | bounded IndexedDB/V8 subset scanner |
| `tui` | no | future ratatui interface |
| `vendored-sqlite` | yes for release | predictable cross-platform builds |
| `native-tls` | no | should not be needed for local-first v0.1 |

## Tooling

Recommended development commands once code exists:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p vibe-hauler -- scan --dry-run
```

Release commands should live in `xtask` or CI, not in ad hoc shell snippets.

