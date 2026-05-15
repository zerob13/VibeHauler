# CLI Module

Cargo package: `vibe-hauler`  
Binary: `vhaul`

## Responsibility

The CLI module owns the terminal application shell:

- parse startup flags with `clap`;
- launch the `ratatui` + `crossterm` TUI;
- load config and root overrides;
- call discovery/adapters/core/cleaner in the TUI workflow order;
- render interactive lists, detail panels, confirmations, and summaries;
- ask typed confirmations before mutation;
- display manifest and restore guidance after cleanup.

It must not parse app data directly and must not delete files directly.

## Public Command

```txt
vhaul
```

`vhaul` is the product surface. Discovery, session review, planning, cleanup, and restore guidance are TUI states, not public subcommands.

## Startup Flags

```txt
vhaul --config <path>
vhaul --no-color
vhaul --portable-root fixtures/macos-home
vhaul --roots claude=/tmp/home/.claude,codex=/tmp/home/.codex
```

These flags prepare the TUI environment. They never execute cleanup on their own.

## Implementation Slices

### Slice 1: startup shell

- Define `Cli` and startup flags.
- Print help and version.
- Launch a placeholder TUI shell for `vhaul`.
- Add `ratatui` and `crossterm` when the first real screen is implemented.

Acceptance:

```bash
cargo run -p vibe-hauler -- --help
cargo run -p vibe-hauler --
cargo run -p vibe-hauler -- --portable-root fixtures/macos-home
```

### Slice 2: discovery and app selection

- The TUI starts with read-only discovery.
- Detected readable apps are selected by default.
- Users can select all, clear all, search, and toggle rows.

Acceptance:

```txt
Given fixture roots, opening `vhaul --portable-root fixtures/macos-home`
shows Claude Code and Codex selected in the app selection screen.
```

### Slice 3: safe cleanup

- Analyze selected apps after the user presses Enter.
- Show Green cleanup candidates selected by default.
- Execute selected Green actions only after the user confirms in the TUI.
- Write a manifest for every mutation.

Acceptance:

```txt
The safe cleanup screen shows Green rows only, skips changed paths,
and reports Trash/quarantine destinations in the final summary.
```

### Slice 4: session review

- Show Yellow session/history data after safe cleanup.
- Let users enter each agent, preview redacted session metadata, and select sessions manually.
- Support quick selection by time and directory.
- Require backup and typed confirmation before session cleanup.

Acceptance:

```txt
No session is selected by default, and cleanup cannot proceed unless
backup succeeds and the typed confirmation matches the selected count.
```

## Output Rules

TUI output:

- use stable ASCII layout for fixtures and snapshots;
- keep color optional and never required for meaning;
- show risk totals and protected reasons;
- truncate long paths with home-relative display;
- keep raw session previews opt-in.

Test output:

- expose internal state through fixture harnesses, not user-facing subcommands;
- snapshot the TUI screens for app selection, safe cleanup, session review, and final summary.

## Testing

- `assert_cmd` for startup flags, help, and version.
- snapshot tests for rendered TUI screens.
- fixture tests for supported OS layouts.
- no test should touch the real home directory.
