# Product Spec

## Identity

VibeHauler is the polished English abstraction of “vibe 垃圾车”: it hauls away agent clutter without pretending every local artifact is disposable.

Taglines:

- Primary: Haul away your agent clutter.
- Technical: Scan, inspect, and clean local agent residue.

One-line description:

> A local-first cleaner for AI agent clients.

Product description:

> VibeHauler scans local AI agent data, explains what is safe to remove, and helps users clean sessions, caches, logs, traces, and stale workspace state across Claude Code, Codex, Cursor, Cherry Studio, Alma, DeepChat, OpenCode, and more.

## Users

Primary users:

- Developers using several coding agents on one machine.
- Power users whose home directory contains months of sessions, traces, caches, logs, workspace storage, SQLite DBs, LevelDB stores, and raw transcripts.
- Users who want CleanMyMac-like confidence but need agent-aware safety rules.

Secondary users:

- Tool authors debugging where their clients store local state.
- Security-conscious users who want to find secrets inside local agent transcripts.
- Teams preparing migration or cleanup policies for developer laptops.

## Product Principles

1. Local-first: no cloud service, no telemetry by default, no remote parsing.
2. Explain before cleaning: every recommendation needs a reason and risk level.
3. Default dry-run: scans and plans are safe by default.
4. Built-in parsers first: runtime should inspect local files directly instead of shelling out to upstream SDKs.
5. Protect credentials and memory: auth, config, rules, skills, long-term memory, provider secrets, and keychain references are Red by default.
6. Prefer reversible cleanup: OS Trash first, backup for Yellow items, manifest for every mutation.
7. Cross-platform by design: macOS, Linux, Windows, and WSL are first-class in the path model.

## MVP Scope

The first working release should ship as a Rust CLI named `vhaul`.

Required commands:

```bash
vhaul scan
vhaul sessions list
vhaul sessions show <session-id>
vhaul plan --safe
vhaul clean --dry-run
vhaul clean --execute --safe-only
vhaul restore <manifest>
vhaul doctor
```

MVP adapters:

| Client | Detection | Size Report | Sessions | Safe Cleanup |
|---|---:|---:|---:|---:|
| Claude Code | yes | yes | JSONL | logs/cache/tmp |
| Codex | yes | yes | JSONL/history | logs/cache/tmp |
| Gemini CLI | yes | yes | JSON/JSONL | logs/cache/tmp |
| Aider | repo scan | yes | Markdown/history | selected history only |
| OpenCode | yes | yes | sniffed JSON/JSONL | logs/cache/tmp |
| Cursor | yes | yes | key-family preview | cache/log only |
| Cherry Studio | yes | yes | backup/partial | trace/cache/log only |
| DeepChat | yes | yes | SQLite report | cache/log only |
| Alma | yes | yes | SQLite report | none by default |
| Goose | yes | yes | SQLite report | logs only |
| Factory Droid | yes | yes | file sniffing | logs/tmp only |

## Risk Language

| Level | Meaning | Default Behavior |
|---|---|---|
| Green | Cache, old logs, tmp files, rebuildable indexes | eligible for `--safe-only` |
| Yellow | Sessions, transcripts, checkpoints, workspace history | visible in plan, requires explicit selection or backup |
| Red | Credentials, config, memory, rules, skills, core DBs | protected |
| Black | Locked, unknown, permission denied, unsafe path | report only |

## Non-goals for v0.1

- No background daemon.
- No GUI or TUI yet.
- No cloud sync.
- No permanent delete by default.
- No automatic SQLite replacement for Cursor/Cherry/DeepChat/Alma; advanced plans can be generated, but execution comes later.
- No unsupported destructive edits to opaque LevelDB/IndexedDB stores.

## Release Stages

| Version | Theme | Deliverable |
|---|---|---|
| v0.1 | Safe scanner and cleaner | CLI, scan, sessions, plan, safe clean, restore |
| v0.2 | Parser depth | Goose, DeepChat, Cursor, Cherry parser improvements |
| v0.3 | TUI | interactive multi-select cleanup and preview |
| v1.0 | Agent data control center | export, archive, privacy report, richer adapter registry |

