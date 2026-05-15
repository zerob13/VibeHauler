# CLI Spec

The CLI binary is `vhaul`.

Every command must be useful in a terminal, scriptable with JSON, and safe by default.

## Command Map

```bash
vhaul scan
vhaul scan --apps claude,codex,opencode --json
vhaul scan --roots cursor="/Users/me/Library/Application Support/Cursor/User"
vhaul scan --portable-root ./fixtures/macos-home

vhaul sessions list
vhaul sessions list --app claude --since 30d --sort size
vhaul sessions list --cwd ~/work/my-repo
vhaul sessions show <session-id> --format markdown
vhaul sessions export <session-id> --format jsonl --output ./session.jsonl

vhaul plan --safe
vhaul plan --include-yellow --older-than 90d --apps claude,codex,aider
vhaul clean --plan .vhaul/plans/plan.json --dry-run
vhaul clean --plan .vhaul/plans/plan.json --execute --safe-only

vhaul restore .vhaul/manifests/20260515-231722.json
vhaul doctor
vhaul completion zsh
```

## Global Flags

| Flag | Meaning |
|---|---|
| `--config <path>` | use a specific config file |
| `--json` | emit machine-readable JSON |
| `--no-color` | disable ANSI colors |
| `--verbose` | more diagnostic output |
| `--quiet` | suppress nonessential text |
| `--portable-root <path>` | treat path as fake home for fixtures/tests |
| `--roots app=path,...` | add or override app roots |
| `--apps a,b,c` | restrict adapters |

## UX Rules

1. `scan`, `sessions list`, `plan`, and `doctor` are read-only.
2. `clean` defaults to dry-run unless `--execute` is passed.
3. `--execute` requires either `--safe-only` or an explicit plan that includes Yellow selections.
4. Yellow actions require backup unless the user passes a future dangerous override.
5. Red and Black actions are skipped by default and must be shown with reasons.
6. Any raw session preview requires a second confirmation or `--raw`.
7. JSON output must contain the same IDs as table output.

## Output Examples

```txt
$ vhaul scan

VibeHauler Scan
Local only · dry-run · built-in parsers · 12 adapters

App            Root             Size      Candidates  Recommendation
Claude Code    ~/.claude        3.42 GB   184         1.87 GB cleanable
Codex          ~/.codex         1.18 GB   92          622 MB cleanable
Cursor         App Support      11.6 GB   9           backup + vacuum plan
Cherry Studio  App Support      2.24 GB   17          trace/cache only

Green   2.9 GB  cache/log/tmp
Yellow  4.1 GB  old sessions/trace/checkpoints
Red     6.8 GB  credentials/config/memory/databases
Black   0.4 GB  locked or unknown
```

```txt
$ vhaul clean --plan .vhaul/plans/20260515.json --execute --safe-only

Plan summary
  Green   1.42 GB  93 files
  Yellow  skipped
  Red     protected

Destination: OS Trash
Manifest:    ~/.local/share/vibe-hauler/manifests/20260515-231722.json

Type CLEAN to continue: _
```

## Exit Codes

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | general error |
| 2 | invalid arguments/config |
| 3 | partial scan failure, report still emitted |
| 4 | cleanup aborted by safety check |
| 5 | restore failed |

