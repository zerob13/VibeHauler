# Cleaner Module

Cargo package: `vibe-hauler-cleaner`

## Responsibility

The cleaner module is the only module allowed to mutate target files.

It owns:

- plan execution;
- backups;
- OS Trash integration;
- quarantine fallback;
- manifest writing;
- restore.

It does not decide app semantics.

## Execution Flow

```txt
Load plan
  -> validate plan still matches current files
  -> preflight process/lock/path checks
  -> create backup set for required actions
  -> move Green/selected Yellow items to OS Trash
  -> quarantine if Trash fails
  -> write manifest
  -> print execution report
```

## Backup Policy

| Risk | Backup |
|---|---|
| Green | optional unless configured |
| Yellow | required |
| Red | not executed |
| Black | not executed |

Backups should store:

- source relative path;
- source absolute path;
- sha256;
- size;
- mtime;
- compression method;
- app and item id.

## Manifest Policy

The manifest is append-only for execution results. It should include enough data to restore without rescanning.

```json
{
  "id": "20260515-231722",
  "plan_id": "plan_...",
  "actions": [
    {
      "source": "/Users/me/.claude/projects/session.jsonl",
      "backup": "/Users/me/.local/share/vibe-hauler/backups/...",
      "destination": "trash",
      "sha256": "...",
      "risk": "yellow",
      "restore_possible": true
    }
  ]
}
```

## Restore Flow

```txt
Load manifest
  -> verify backup hashes
  -> check destination path conflicts
  -> recreate directories
  -> restore files
  -> verify restored hashes
  -> write restore report
```

## SQLite Advanced Actions

For v0.1, SQLite compaction is plan-only. Future execution must:

1. require target app closed;
2. backup original DB;
3. run integrity check on copy;
4. run `VACUUM INTO` compact DB;
5. run quick check;
6. replace atomically;
7. record both hashes.

## Testing

- dry-run does not mutate files;
- backup hash roundtrip;
- trash fallback to quarantine;
- restore refuses overwrite by default;
- manifest snapshot;
- symlink escape rejection.

