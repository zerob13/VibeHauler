# Interactive Cleanup Flow

This document describes the default interactive terminal flow opened by running:

```bash
vhaul
```

The flow is the primary product surface. Users run one command, then complete discovery, selection, cleanup, and confirmation inside the TUI.

Status: primary MVP interface.

## Goals

1. Start with local discovery and show detected agent apps.
2. Default to inspecting all detected apps, while making app selection reversible.
3. Separate safe rebuildable cleanup from session/history cleanup.
4. Make Green cleanup fast, clear, and selected by default.
5. Make Yellow session cleanup deliberate, filterable, previewable, backed up, and confirmed twice.
6. Never offer Red or Black items as cleanup selections.

## Flow Summary

```txt
$ vhaul
  |
  v
Discover local agent roots
  |
  v
App selection
  |  Enter
  v
Analyze selected apps
  |
  v
Safe cleanup selection       Green items, selected by default
  |  Enter
  v
Safe cleanup execution       OS Trash or quarantine + manifest
  |
  v
Session data review          Yellow items, selected manually
  |
  +--> Agent session browser
        |
        +--> quick select by time
        +--> quick select by directory
        +--> preview and manual toggle
        |
        v
      Session deletion confirmation
        |
        v
      Backup + trash + manifest
        |
        v
Final summary
```

## Global Keyboard Model

```txt
Up / Down        move cursor
PageUp / PageDn  scroll page
Space            toggle current row
a                select all visible rows
n                clear all visible rows
/                search or filter
Enter            continue / open / execute focused action
Esc              go back one step
q                quit safely
?                show help
```

Screen-specific keys may add shortcuts, but these keys should behave consistently everywhere.

## Step 0: Boot And Discover

When the user runs `vhaul`, the CLI starts in local-only mode and discovers known app roots.

```txt
$ vhaul

VibeHauler
Local-only agent cleanup

Finding local agent data...

  [ok] Claude Code    ~/.claude
  [ok] Codex          ~/.codex
  [ok] Cursor         ~/Library/Application Support/Cursor
  [--] OpenCode       not found
  [!!] DeepChat       permission denied

Found 3 apps - 1 warning
```

Rules:

- Discovery is read-only.
- Detected apps are selected by default on the next screen.
- Missing apps are hidden by default, but can be revealed in help or diagnostics.
- Permission denied and locked roots are shown as warnings, then marked Black/report-only.

## Step 1: App Selection

The first interactive screen lets the user choose which agent apps should be analyzed.

```txt
+-- VibeHauler ----------------------------- Local-only cleanup --+
| Select apps to inspect                                          |
| All detected apps are selected by default.                      |
|                                                                 |
| [a] Select all    [n] Clear all    [/] Search                   |
+----+------------------+--------------------------+--------------+
|    | App              | Data root                | Status       |
+----+------------------+--------------------------+--------------+
| >  | [x] Claude Code  | ~/.claude                | found        |
|    | [x] Codex        | ~/.codex                 | found        |
|    | [x] Cursor       | ~/Library/.../Cursor     | found        |
|    | [ ] DeepChat     | ~/Library/.../DeepChat   | denied       |
+----+------------------+--------------------------+--------------+
| Selected: 3 apps                                                |
| Enter Continue - Space Toggle - ? Help - q Quit                 |
+-----------------------------------------------------------------+
```

Behavior:

- `Claude Code`, `Codex`, and other detected readable apps default to `[x]`.
- Black/report-only roots such as denied or locked paths default to `[ ]`.
- `a` selects all selectable rows.
- `n` clears all selectable rows.
- `Space` toggles the focused app.
- `Enter` continues only if at least one app is selected.

Empty selection state:

```txt
No apps selected.

Select at least one app to continue, or press q to quit.
```

## Step 2: Analysis

After app selection, VibeHauler analyzes inventory and sessions for selected apps.

```txt
+-- Analyzing selected apps --------------------------------------+
| [================----] 3 / 4 apps                               |
|                                                                 |
| Claude Code   inventory 184 items   sessions 91    done         |
| Codex         inventory 92 items    sessions 37    done         |
| Cursor        inventory 9 items     sessions report pending     |
| Gemini CLI    reading JSONL files...                            |
+-----------------------------------------------------------------+
```

Analysis output is split into two user-facing groups:

1. Safe cleanup candidates: Green inventory items such as cache, logs, temp files, and rebuildable indexes.
2. Session data: Yellow agent sessions, transcripts, checkpoints, traces, and workspace history.

Red and Black items are counted and explained, but never selectable.

## Step 3: Safe Cleanup Selection

The first result section is the safe cleanup list. These are rebuildable Green items and are selected by default.

```txt
+-- Safe Cleanup -------------------------------------------------+
| Rebuildable data selected by default                            |
| Selected: 2.91 GB - 317 files                                   |
+----+--------------------------+-------------+--------+----------+
|    | Item                     | App         | Size   | Why      |
+----+--------------------------+-------------+--------+----------+
| >  | [x] logs/                | Claude Code | 418 MB | logs     |
|    | [x] cache/               | Claude Code | 736 MB | cache    |
|    | [x] tmp/                 | Codex       | 84 MB  | temp     |
|    | [x] session thumbnails   | Cursor      | 1.6 GB | cache    |
+----+--------------------------+-------------+--------+----------+
| Protected: 6.8 GB Red/Black items are report-only               |
| Enter Clean selected - Space Toggle - p Preview - q Quit        |
+-----------------------------------------------------------------+
```

Behavior:

- Only Green items appear as selectable rows.
- Rows are grouped by app and category.
- The default is `[x]` for all safe cleanup rows.
- `Space` lets the user keep any item.
- `p` opens a read-only detail panel with paths, reason, risk, and destination.
- `Enter` executes the selected Green cleanup.

No safe cleanup state:

```txt
+-- Safe Cleanup -------------------------------------------------+
| No rebuildable cleanup candidates found for the selected apps.  |
|                                                                 |
| Enter Continue to session review - q Quit                       |
+-----------------------------------------------------------------+
```

## Step 4: Safe Cleanup Execution

Safe cleanup moves selected Green items to OS Trash when available, otherwise to VibeHauler quarantine. Every mutation writes a manifest.

```txt
+-- Cleaning safe items -----------------------------------------+
| [====================] 317 / 317 files                         |
|                                                                 |
| Moved to Trash:      2.74 GB                                    |
| Quarantined:         170 MB                                     |
| Failed:              0 files                                    |
| Manifest:            ~/.local/share/vibe-hauler/manifests/...   |
+-----------------------------------------------------------------+
```

If a Green item changes between analysis and execution, it is skipped and reported.

```txt
Skipped changed item

Codex tmp/ changed after analysis.
VibeHauler left it untouched. Refresh analysis to rebuild the selection.
```

## Step 5: Session Data Review

After safe cleanup, the second section reviews session data from the selected agents. Session cleanup is not selected by default.

```txt
+-- Session Data -------------------------------------------------+
| Agent sessions are history/transcripts/checkpoints.             |
| Select an app to review its sessions.                           |
+----+------------------+----------+--------+---------------------+
|    | App              | Sessions | Size   | Newest              |
+----+------------------+----------+--------+---------------------+
| >  | Claude Code      | 91       | 3.10GB | 2026-05-15 18:22    |
|    | Codex            | 37       | 622MB  | 2026-05-15 17:49    |
|    | Cursor           | report   | 11.6GB | protected DB        |
+----+------------------+----------+--------+---------------------+
| Enter Open app - s Skip session cleanup - q Quit                |
+-----------------------------------------------------------------+
```

Behavior:

- Session rows are Yellow unless the adapter marks them protected.
- Nothing is selected for deletion when entering this section.
- Apps with only protected databases show `report` and open a read-only explanation.
- `s` skips session cleanup and goes to the final summary.

## Step 6: Agent Session Browser

Opening an app shows its sessions with time, directory, title, and size. The user can manually select rows or use quick selection.

```txt
+-- Sessions / Codex --------------------------------------------+
| 37 sessions - 622 MB total - 0 selected                         |
| [t] Time  [d] Directory  [/] Search  [o] Sort                   |
+----+------------------+------------------------+-------+--------+
|    | Updated          | Directory              | Size  | Title  |
+----+------------------+------------------------+-------+--------+
| >  | [ ] 2026-05-15   | ~/work/VibeHauler      | 18 MB | docs   |
|    | [ ] 2026-05-13   | ~/work/old-prototype   | 74 MB | api    |
|    | [ ] 2026-04-01   | ~/tmp/experiment       | 91 MB | test   |
|    | [ ] 2026-02-18   | ~/work/archived-app    | 203MB | ui     |
+----+------------------+------------------------+-------+--------+
| Preview: docs cleanup flow, README edits, cargo check           |
| Space Toggle - Enter Review selected - Esc Back                 |
+-----------------------------------------------------------------+
```

Behavior:

- `Space` toggles one session.
- `a` selects all visible sessions after filters.
- `n` clears all visible sessions after filters.
- `Enter` continues to confirmation if at least one session is selected.
- The preview line is redacted by default.
- Raw session previews require a separate opt-in action.

## Quick Select By Time

The time selector helps users bulk-select old sessions.

```txt
+-- Select by time ----------------------------------------------+
| Apply to: Codex sessions                                        |
+-----------------------------------------------------------------+
| > [ ] Older than 7 days        14 sessions - 318 MB             |
|   [ ] Older than 30 days       8 sessions  - 241 MB             |
|   [ ] Older than 90 days       3 sessions  - 96 MB              |
|   [ ] Custom range             choose start/end dates           |
+-----------------------------------------------------------------+
| Enter Apply - Esc Cancel                                        |
+-----------------------------------------------------------------+
```

Rules:

- Time selection adds to the current selection.
- The user can review and untoggle individual sessions after applying it.
- Missing timestamps fall back to file modified time and are labeled as such.

## Quick Select By Directory

The directory selector helps users remove sessions from old projects or temporary folders.

```txt
+-- Select by directory -----------------------------------------+
| Grouped by detected cwd / workspace                             |
+----+--------------------------------------+----------+----------+
|    | Directory                            | Sessions | Size     |
+----+--------------------------------------+----------+----------+
| >  | [ ] ~/work/old-prototype             | 12       | 344 MB   |
|    | [ ] ~/tmp/experiment                 | 9        | 188 MB   |
|    | [ ] ~/work/archived-app              | 7        | 279 MB   |
|    | [ ] unknown directory                | 4        | 31 MB    |
+----+--------------------------------------+----------+----------+
| Space Toggle - Enter Apply - Esc Cancel                         |
+-----------------------------------------------------------------+
```

Rules:

- Selecting a directory selects all visible sessions in that directory group.
- Unknown directory is allowed but should be clearly labeled.
- Directory selection never crosses into Red protected data.

## Step 7: Session Deletion Confirmation

Session cleanup requires a deliberate confirmation because these entries are user history. The confirmation summarizes the irreversible app-facing effect, backup behavior, and destination.

```txt
+-- Confirm session cleanup -------------------------------------+
| You selected 19 sessions across 2 apps.                         |
|                                                                 |
| Claude Code       11 sessions   812 MB                          |
| Codex             8 sessions    241 MB                          |
|                                                                 |
| Backup required:  yes                                           |
| Backup size:      1.05 GB                                       |
| Destination:      OS Trash, fallback to quarantine               |
| Manifest:         will be written after cleanup                  |
|                                                                 |
| These sessions will disappear from the agent clients.            |
| Type DELETE 19 to continue: _                                   |
+-----------------------------------------------------------------+
```

Behavior:

- The typed phrase must include the selected session count, for example `DELETE 19`.
- If the selection changes, the required phrase changes.
- Yellow session cleanup is backed up before being moved to Trash/quarantine.
- If backup cannot be completed, cleanup is blocked.
- Red/Black sessions cannot reach this confirmation screen.

Abort state:

```txt
Session cleanup aborted.

No session data was changed.
```

## Step 8: Session Cleanup Execution

```txt
+-- Cleaning sessions -------------------------------------------+
| [==============------] 14 / 19 sessions                         |
|                                                                 |
| Backed up:          14 sessions - 733 MB                        |
| Moved to Trash:     14 sessions                                 |
| Waiting:            5 sessions                                  |
+-----------------------------------------------------------------+
```

Partial failure behavior:

- Already backed-up sessions may be moved to Trash.
- Sessions that fail backup are not removed.
- The manifest records every success, skip, and failure.
- The final summary must show whether restore is available.

## Step 9: Final Summary

```txt
+-- Done ---------------------------------------------------------+
| Safe cleanup                                                    |
|   Cleaned:      2.91 GB - 317 files                             |
|   Skipped:      1 changed item                                  |
|                                                                 |
| Session cleanup                                                 |
|   Cleaned:      19 sessions - 1.05 GB                           |
|   Backed up:    yes                                             |
|                                                                 |
| Manifest                                                        |
|   ~/.local/share/vibe-hauler/manifests/20260515-231722.json     |
|                                                                 |
| Restore                                                         |
|   vhaul restore ~/.local/share/vibe-hauler/manifests/...json    |
+-----------------------------------------------------------------+
```

If the user skipped session cleanup:

```txt
Session cleanup skipped.
No session history was changed.
```

## Protected Items Panel

At any point after analysis, the user can open a protected-items report.

```txt
+-- Protected Items ---------------------------------------------+
| These items are not selectable in interactive cleanup.           |
+------------------+--------+-------------------------------------+
| App              | Risk   | Reason                              |
+------------------+--------+-------------------------------------+
| Claude Code      | Red    | auth/config/memory protected        |
| Cursor           | Red    | core SQLite DB protected            |
| DeepChat         | Black  | permission denied                   |
| OpenCode         | Black  | unknown binary store                |
+------------------+--------+-------------------------------------+
```

## State Machine

```txt
Boot
  -> Scan
  -> AppSelection
  -> Analyze
  -> SafeSelection
  -> SafeExecution
  -> SessionOverview
  -> SessionBrowser
  -> SessionConfirmation
  -> SessionExecution
  -> FinalSummary

Any state
  -> Help
  -> Quit

SessionBrowser
  -> TimeSelector
  -> DirectorySelector
  -> RawPreviewConfirmation
```

## Safety Rules

1. App selection and analysis are read-only.
2. Green cleanup is selected by default; Yellow cleanup is not.
3. Red and Black items are displayed only as protected/report-only.
4. Green cleanup writes a manifest after mutation.
5. Yellow session cleanup requires backup before removal.
6. Yellow session cleanup requires typed confirmation with the selected count.
7. Raw previews require a separate opt-in confirmation.
8. The CLI must re-stat selected paths before mutating them.
9. If an app database is locked or changed, the affected action is skipped.
10. Permanent deletion is not part of this flow.

## Mapping To Internal Workflow

The TUI coordinates the same internal pipeline that the lower crates expose as Rust APIs:

```txt
discover app roots
  -> build inventory
  -> parse sessions
  -> generate selected cleanup actions
  -> execute confirmed actions
  -> write manifest
  -> show restore guidance
```

The TUI may keep draft selections in memory during the session, but persisted manifests must use the shared `CleanManifest` schema.
