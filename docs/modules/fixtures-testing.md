# Fixtures and Testing

Cargo package: `vibe-hauler-fixtures`  
Folder: `fixtures/`

## Goals

Testing should make VibeHauler confident without touching a real home directory.

The fixture system needs to cover:

- cross-platform path discovery;
- parser behavior;
- risk classification;
- dry-run immutability;
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
| Snapshot | table output, JSON output, plan/manifest JSON |
| Integration | CLI against fixture homes |
| Mutation | backup, trash/quarantine, restore |
| Cross-platform | Windows paths, WSL, XDG, symlinks |
| Regression | real anonymized schema samples |

## Minimum v0.1 Test Matrix

| Area | Required Tests |
|---|---|
| CLI | help, scan JSON, sessions list, plan dry-run |
| Discovery | macOS/Linux/Windows fixture roots |
| Claude | JSONL parse with unknown event |
| Codex | history and sessions path parse |
| Gemini | tmp chats parse |
| Aider | Markdown segmentation |
| Cursor | state.vscdb readonly schema report |
| Cherry | LocalStorage UTF-16/NUL sample and backup JSON |
| Cleaner | dry-run no mutation, backup hash, restore |
| Redaction | API keys, bearer tokens, URL credentials |

## Golden Files

Use snapshots for:

- `scan --json`;
- `sessions list --json`;
- `plan --safe --json`;
- parser outputs;
- manifest outputs.

Snapshots should avoid absolute machine paths by using fixture path normalization.

