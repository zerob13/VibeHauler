# Fixtures and Testing

Cargo package: `vibe-hauler-fixtures`  
Folder: `fixtures/`

## Goals

Testing should make VibeHauler confident without touching a real home directory.

The fixture system needs to cover:

- cross-platform path discovery;
- parser behavior;
- risk classification;
- pre-execution immutability;
- backup/trash/restore;
- locked DB and malformed data.

## Fixture Layout

```txt
fixtures/
├── macos-home/
│   ├── .claude/
│   ├── .codex/
│   ├── .gemini/
│   └── Library/Application Support/CherryStudio/
├── linux-home/
│   ├── .config/Cursor/User/
│   ├── .local/share/opencode/
│   └── .local/share/goose/
├── windows-home/
│   ├── AppData/Roaming/Cursor/User/
│   ├── AppData/Roaming/DeepChat/
│   └── .codex/
└── repos/
    └── aider-project/
        ├── .aider.chat.history.md
        └── .aider.input.history
```

## Test Categories

| Type | Purpose |
|---|---|
| Unit | path helpers, parser functions, redaction, risk rules |
| Snapshot | TUI screens, plan/manifest JSON |
| Integration | TUI startup against fixture homes |
| Mutation | backup, trash/quarantine, restore |
| Cross-platform | Windows paths, WSL, XDG, symlinks |
| Regression | real anonymized schema samples |

## Minimum v0.1 Test Matrix

| Area | Required Tests |
|---|---|
| CLI/TUI | help, version, app selection, safe cleanup screen |
| Discovery | macOS/Linux/Windows fixture roots |
| Claude | JSONL parse with unknown event |
| Codex | history and sessions path parse |
| Gemini | tmp chats parse |
| Aider | Markdown segmentation |
| Cleaner | no mutation before confirmation, backup hash, manifest |
| Redaction | API keys, bearer tokens, URL credentials |

Post-v0.1 fixture areas:

| Area | First Stage |
|---|---|
| Cursor state.vscdb readonly schema report | v0.2 |
| Goose sessions.db readonly report | v0.2 |
| Cherry LocalStorage/backup JSON samples | v0.3 |
| DeepChat/Alma SQLite schema reports | v0.3 |

## Golden Files

Use snapshots for:

- app selection screen;
- safe cleanup screen;
- session browser screen;
- parser outputs;
- manifest outputs.

Snapshots should avoid absolute machine paths by using fixture path normalization.
