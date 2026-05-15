# CLI Spec

The installed binary is `vhaul`.

`vhaul` is a TUI-first application: running the binary opens the interactive cleanup interface. Discovery, analysis, session review, cleanup confirmation, and restore guidance happen inside that interface instead of through public subcommands.

## Command Map

```bash
vhaul
vhaul --help
vhaul --version
```

There are no public operational subcommands in the product surface. Discovery, session browsing, planning, and cleanup are internal workflow states owned by the TUI.

## Startup Flags

Startup flags configure the TUI before it opens. They do not execute cleanup by themselves.

| Flag | Meaning |
|---|---|
| `--config <path>` | use a specific config file |
| `--no-color` | disable ANSI color while keeping the TUI layout usable |
| `--portable-root <path>` | treat path as a fake home for fixtures/tests |
| `--roots app=path,...` | add or override app roots before discovery |

## UX Rules

1. `vhaul` opens the TUI when running in a terminal.
2. The initial discovery pass is internal and read-only.
3. Detected readable apps are selected by default on the app selection screen.
4. Green cleanup candidates are selected by default.
5. Yellow session/history items are never selected by default.
6. Red and Black items are shown only as protected/report-only.
7. Session deletion requires backup and typed confirmation.
8. Raw previews require a separate opt-in confirmation.
9. Every mutation writes a manifest and displays restore guidance.
10. Permanent deletion is not part of the default TUI.

The full screen flow is specified in [Interactive Cleanup Flow](./06-interactive-cleanup-flow.md).

## Startup Example

```txt
$ vhaul

+-- VibeHauler -------------------------------- Local cleanup ----+
| Finding local agent data...                                     |
|                                                                 |
| [ok] Claude Code    ~/.claude                                   |
| [ok] Codex          ~/.codex                                    |
| [ok] Cursor         ~/Library/Application Support/Cursor         |
| [--] OpenCode       not found                                   |
+-----------------------------------------------------------------+
```

The next screen is app selection. The user continues through the workflow using keyboard controls inside the TUI.

## Exit Codes

| Code | Meaning |
|---:|---|
| 0 | success or user quit without changes |
| 1 | general error |
| 2 | invalid startup flags/config |
| 3 | partial discovery or analysis failure, report still shown |
| 4 | cleanup aborted by safety check |
| 5 | restore guidance or manifest write failed |
