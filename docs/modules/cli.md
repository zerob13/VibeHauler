# CLI Module

Cargo package: `vibe-hauler`  
Binary: `vhaul`

## Responsibility

The CLI module owns terminal UX:

- parse commands and flags with `clap`;
- load config;
- call discovery/adapters/core/cleaner in order;
- format human tables and JSON output;
- ask typed confirmations before mutation;
- expose shell completions and manpages.

It must not parse app data directly and must not delete files directly.

## Public Commands

```txt
vhaul scan
vhaul sessions list
vhaul sessions show
vhaul sessions export
vhaul plan
vhaul clean
vhaul restore
vhaul doctor
vhaul completion
```

## Implementation Slices

### Slice 1: command skeleton

- Define `Cli`, `Commands`, and per-command args.
- Wire `--json`, `--config`, `--apps`, `--roots`, `--portable-root`.
- Print help, version, and shell completions.

Acceptance:

```bash
cargo run -p vibe-hauler -- --help
cargo run -p vibe-hauler -- scan --help
cargo run -p vibe-hauler -- completion zsh
```

### Slice 2: read-only command pipeline

- `scan` calls config, discovery, adapters, risk summary.
- `sessions list` calls adapters and parser registry.
- `doctor` reports config paths, enabled adapters, platform, and feature support.

Acceptance:

```bash
vhaul scan --portable-root fixtures/macos-home --json
vhaul sessions list --portable-root fixtures/macos-home --app claude
```

### Slice 3: plan and clean

- `plan` writes a plan file under `.vhaul/plans/` or configured data dir.
- `clean --dry-run` renders what would happen.
- `clean --execute` requires typed confirmation.

Acceptance:

```bash
vhaul plan --safe --portable-root fixtures/macos-home
vhaul clean --plan .vhaul/plans/example.json --dry-run
```

## Output Rules

Human output:

- default to compact tables;
- show risk totals;
- show why Red/Black items are skipped;
- truncate long paths with home-relative display.

JSON output:

- stable schema;
- full IDs;
- absolute paths;
- no ANSI;
- include warnings and partial failures.

## Testing

- `assert_cmd` for command behavior.
- `insta` snapshots for table and JSON output.
- one fixture test per supported OS layout.
- no test should touch the real home directory.

