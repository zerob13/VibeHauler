# VibeHauler Documentation

VibeHauler is a local-first cleaner for AI agent clients.

It opens a terminal UI for Claude Code, Codex, Cursor, Cherry Studio, Alma, DeepChat, OpenCode, and related tools, then explains disk usage, sessions, caches, logs, traces, SQLite/LevelDB state, and cleanup risk before anything is removed.

Canonical naming:

| Surface | Name |
|---|---|
| Product | VibeHauler |
| CLI binary | `vhaul` |
| Main Rust package | `vibe-hauler` |
| GitHub repo | `zerob13/VibeHauler` |
| Chinese nickname | vibe 垃圾车 |
| Tagline | Haul away your agent clutter. |

## Document Map

Start here:

1. [Product Spec](./00-product-spec.md) defines positioning, principles, MVP scope, risk language, and release stages.
2. [System Architecture](./01-system-architecture.md) explains module boundaries and the full data flow.
3. [Project Organization](./02-project-organization.md) defines the future Rust workspace layout, crate naming, repo folders, and ownership.
4. [CLI Spec](./03-cli-spec.md) defines the one-command `vhaul` TUI entrypoint, startup flags, outputs, and UX rules.
5. [Data Model](./04-data-model.md) defines shared structs, IDs, manifests, plans, and report schemas.
6. [Security Model](./05-security-model.md) defines safety invariants, default protections, backup, trash, quarantine, and restore.
7. [Interactive Cleanup Flow](./06-interactive-cleanup-flow.md) defines the no-args `vhaul` terminal flow with ASCII UI screens.

Implementation docs:

- [Modules Overview](./modules/README.md)
- [CLI Module](./modules/cli.md)
- [Core Module](./modules/core.md)
- [Discovery Module](./modules/discovery.md)
- [Adapters Module](./modules/adapters.md)
- [Parsers Module](./modules/parsers.md)
- [Cleaner Module](./modules/cleaner.md)
- [Redaction Module](./modules/redaction.md)
- [Config Module](./modules/config.md)
- [Fixtures and Testing](./modules/fixtures-testing.md)

Client adapters:

- [Adapter Plan](./adapters/README.md)

Release and distribution:

- [Homebrew Release](./release/homebrew.md)
- [npm Release](./release/npm.md)
- [Release Checklist](./release/checklist.md)

Historical reference:

- [Old Vibe Cleaner Plan](./vibe-cleaner-rust-cli-plan-v2.md)

## Current Decision

The old `Vibe Cleaner / vclean` naming is superseded by `VibeHauler / vhaul / vibe-hauler`.

No implementation code should be started until these docs are stable enough to serve as module contracts. The first coding milestone should create the Rust workspace and implement only the public interfaces described here.
