# VibeHauler

<div align="center">
  <img src="assets/vibehauler-mascot.png" alt="VibeHauler mascot logo" width="220">
</div>

A local-first cleaner for AI agent clients.

VibeHauler opens a terminal UI that finds local AI agent data, explains what is safe to remove, and helps clean sessions, caches, logs, and temporary files without touching credentials or long-term configuration.

## Status

Current `main` supports the original CLI clients plus protective desktop-client cleanup:

| Client | Tier | Detection | Inventory | Sessions | Safe cleanup |
|---|---|---:|---:|---:|---:|
| Claude Code | Full | yes | yes | JSONL | logs/cache/tmp |
| Codex | Full | yes | yes | JSONL/history | logs/cache/tmp |
| Gemini CLI | Full | yes | yes | JSONL chats | logs/cache/tmp |
| Cursor | Protective | yes | report-only DB state | none yet | cache/log only |
| Cherry Studio | Protective | yes | report-only LevelDB/IndexedDB/SQLite state | none yet | trace/cache/log only |
| DeepChat | Protective | yes | read-only `agent.db` sessions | report-only | cache/log/tmp only |

Protective adapters never mutate app databases or opaque browser stores. Aider support was removed from the default registry; the old Markdown parser remains as a regression fixture only.

## Install

From source:

```bash
git clone https://github.com/zerob13/VibeHauler.git
cd VibeHauler
cargo install --path crates/vibe-hauler
vhaul
```

During development, run directly:

```bash
cargo run -p vibe-hauler --
```

Homebrew:

```bash
brew install zerob13/tap/vibe-hauler
vhaul
```

Planned npm distribution:

```bash
npm install -g vibe-hauler
```

Direct release archives should contain `vhaul` or `vhaul.exe`, `README.md`, and `LICENSE`.

## Usage

Open the interactive TUI:

```bash
vhaul
```

Useful startup flags:

```bash
vhaul --help
vhaul --version
vhaul --no-color
vhaul --config path/to/config.toml
vhaul --portable-root fixtures/macos-home
vhaul --roots claude=/tmp/home/.claude,codex=/tmp/home/.codex
```

Safety model:

- discovery and analysis are read-only;
- Green items are rebuildable logs, caches, and temp files, selected by default;
- Yellow items are sessions/history and require explicit selection plus backup;
- Red and Black items are protected/report-only;
- cleanup moves files to OS Trash when available or VibeHauler quarantine;
- every mutation writes a manifest under the VibeHauler data directory;
- execution re-checks source files before moving them and skips changed files.

TUI flow:

```txt
Boot -> AppSelection -> Analyze -> SafeSelection -> SafeExecution
     -> SessionOverview -> SessionBrowser -> SessionConfirmation
     -> SessionExecution -> FinalSummary
```

For fixture-safe usage while testing:

```bash
cargo run -p vibe-hauler -- --portable-root fixtures/macos-home --no-color
```

Do not run mutation tests against a real home directory. Use `--portable-root fixtures/<os>-home/` or a temporary copy of a fixture home.

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
├── vibe-hauler-core         # shared models, risk engine, plan builder
├── vibe-hauler-config       # config loading and validation
├── vibe-hauler-discovery    # cross-platform root discovery
├── vibe-hauler-adapters     # app-specific inventory/session extraction
├── vibe-hauler-parsers      # JSONL and Markdown session parsers
├── vibe-hauler-cleaner      # backup, trash/quarantine, manifest, restore
├── vibe-hauler-redaction    # secret masking and preview truncation
└── vibe-hauler-fixtures     # test fixture helpers
```

## Development

Requirements:

- Rust toolchain from `rust-toolchain.toml`;
- `cargo`;
- optional `tmux` for local TUI snapshot driving.

Common commands:

```bash
cargo fmt --all
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p vibe-hauler --
```

Recommended pre-PR check:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Run against a fixture home:

```bash
cargo run -p vibe-hauler -- --portable-root fixtures/macos-home --no-color
cargo run -p vibe-hauler -- --portable-root fixtures/linux-home --no-color
cargo run -p vibe-hauler -- --portable-root fixtures/windows-home --no-color
```

Add new behavior in module order:

1. core models/risk/plan;
2. config/discovery;
3. parsers;
4. adapters;
5. cleaner;
6. CLI/TUI;
7. fixtures and tests.

Keep adapter semantics out of `vibe-hauler-core`, and keep filesystem mutation inside `vibe-hauler-cleaner`.

## Testing

The test suite is fixture-first. It must not depend on the developer's real home directory.

```bash
cargo test --workspace --all-features
```

Covered areas include:

- adapter detect/inventory/sessions for Claude, Codex, and Gemini;
- protective detect/inventory/no-session behavior for Cursor and Cherry Studio;
- read-only DeepChat `agent.db` session parsing;
- JSONL known and unknown events;
- legacy Markdown history parsing;
- RiskLevel assignment;
- CleanPlan generation and serialization;
- CleanManifest generation;
- dry-run no-mutation behavior;
- backup -> trash -> manifest -> restore roundtrip;
- symlink escape rejection;
- mock process guard;
- CLI help and fixture-backed TUI startup.

Optional manual TUI smoke with `tmux`:

```bash
tmux new-session -d -s vhaul-test
tmux send-keys -t vhaul-test 'cargo run -p vibe-hauler -- --portable-root fixtures/macos-home --no-color' Enter
sleep 2
tmux capture-pane -t vhaul-test -p
tmux send-keys -t vhaul-test Enter
tmux send-keys -t vhaul-test Enter
tmux send-keys -t vhaul-test s
tmux capture-pane -t vhaul-test -p
tmux kill-session -t vhaul-test
```

For mutation smoke tests, copy fixtures to a temporary directory first:

```bash
tmp="$(mktemp -d)"
cp -R fixtures/macos-home "$tmp/macos-home"
cargo run -p vibe-hauler -- --portable-root "$tmp/macos-home" --no-color
```

## Packaging

Release builds:

```bash
cargo build -p vibe-hauler --release
target/release/vhaul --version
target/release/vhaul --help
```

Suggested artifact names:

```txt
vhaul-v0.2.0-aarch64-apple-darwin.tar.gz
vhaul-v0.2.0-x86_64-apple-darwin.tar.gz
vhaul-v0.2.0-x86_64-unknown-linux-gnu.tar.gz
vhaul-v0.2.0-x86_64-pc-windows-msvc.zip
checksums.txt
```

Each archive should include:

```txt
vhaul or vhaul.exe
README.md
LICENSE
```

Homebrew packaging lives in:

```txt
packaging/homebrew/Formula/vibe-hauler.rb
```

npm packaging lives in:

```txt
packaging/npm/vibe-hauler/
packaging/npm/vibe-hauler-darwin-arm64/
packaging/npm/vibe-hauler-darwin-x64/
packaging/npm/vibe-hauler-linux-x64/
packaging/npm/vibe-hauler-win32-x64/
```

The npm package is a native binary wrapper, not a Node rewrite. Copy the built `vhaul` binary into the matching platform package before `npm pack` or publish.

Dry-run package checks:

```bash
npm pack --dry-run --prefix packaging/npm/vibe-hauler
npm pack --dry-run --prefix packaging/npm/vibe-hauler-darwin-arm64
npm pack --dry-run --prefix packaging/npm/vibe-hauler-darwin-x64
npm pack --dry-run --prefix packaging/npm/vibe-hauler-linux-x64
npm pack --dry-run --prefix packaging/npm/vibe-hauler-win32-x64
```

## Release

Before tagging:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p vibe-hauler -- --help
cargo run -p vibe-hauler -- --version
```

Release checklist:

1. Update version numbers in Cargo and packaging metadata.
2. Update release notes or `CHANGELOG.md` if present.
3. Build target artifacts for macOS arm64, macOS x64, Linux x64, and Windows x64.
4. Generate `checksums.txt`.
5. Smoke-test `vhaul --version` and `vhaul --help` from packaged artifacts.
6. Create a GitHub Release with binaries and checksums.
7. Update the Homebrew tap formula URL/version/sha256.
8. Publish npm platform packages first, then the `vibe-hauler` wrapper package.
9. Verify install paths:

```bash
brew install zerob13/tap/vibe-hauler
npm install -g vibe-hauler
vhaul --version
vhaul --help
```

Detailed release notes live in:

- [docs/release/checklist.md](docs/release/checklist.md)
- [docs/release/homebrew.md](docs/release/homebrew.md)
- [docs/release/npm.md](docs/release/npm.md)

## Documentation

Start with [docs/README.md](docs/README.md).
