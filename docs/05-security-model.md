# Security Model

VibeHauler handles local data that can contain secrets, credentials, personal messages, source code, and long-term AI memory. The security model is therefore part of the product, not an implementation detail.

## Invariants

1. Read-only commands never mutate files.
2. `clean` defaults to dry-run.
3. Red and Black items are never selectable in safe cleanup.
4. Yellow items require backup before cleanup.
5. OS Trash is preferred over permanent deletion.
6. Every mutation writes a manifest.
7. Restore uses the manifest, not fresh discovery.
8. Raw previews are opt-in.
9. Symlink escape must be detected before backup or deletion.
10. Locked databases and unknown binary stores are report-only.

## Default Protected Data

| Data | Reason |
|---|---|
| auth files, OAuth tokens, keychain references | can log users out or expose credentials |
| API keys and provider configs | high security impact |
| settings/config/rules/skills/commands/hooks | user behavior and project configuration |
| long-term memory, embeddings, vector stores | high semantic value and expensive to rebuild |
| core chat databases | deletion may corrupt app state |
| active LevelDB/SQLite stores | lock and consistency risk |
| project files like `CLAUDE.md`, `AGENTS.md`, `.factory/settings.json` | project behavior control |

## Cleanup Destinations

```txt
Try OS Trash
  -> if successful, record trash destination
  -> if unsupported or failed, copy to VibeHauler quarantine
  -> permanent delete is future-only and requires explicit dangerous flags
```

Recommended local storage:

| OS | Data Path |
|---|---|
| macOS | `~/Library/Application Support/vibe-hauler/` |
| Linux | `${XDG_DATA_HOME:-~/.local/share}/vibe-hauler/` |
| Windows | `%APPDATA%\vibe-hauler\` |

Subfolders:

```txt
backups/
manifests/
quarantine/
plans/
logs/
```

## Preflight Checks

Before TUI execution:

1. Confirm selected actions match the saved plan.
2. Re-stat each source path and compare expected size/mtime/hash policy.
3. Check process guards for known desktop apps.
4. Check SQLite/LevelDB locks where relevant.
5. Verify backup destination has enough free space for required backups.
6. Refuse path traversal and symlink escapes.
7. Ask for typed confirmation.

## Restore Rules

Restore should:

- recreate parent directories as needed;
- refuse to overwrite newer files unless `--force` is added in a future version;
- verify backup hash before restoring;
- record restore outcome in the manifest or a sibling restore report;
- never infer missing paths from current scan state.
