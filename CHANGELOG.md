# Changelog

## v0.2.0 - 2026-05-15

- Removes Aider from the default adapter set and keeps its Markdown parser coverage as a legacy parser regression.
- Adds protective Cursor, Cherry Studio, and DeepChat adapters across macOS, Linux, and Windows-style fixture layouts.
- Parses DeepChat `app_db/agent.db` sessions read-only from `new_sessions` and `deepchat_messages`, with legacy `chat.db` fallback.
- Marks database-backed desktop sessions as report-only so TUI review can inspect them without enabling unsafe DB mutation.
- Improves the TUI session browser with pagination, filtering, scroll-aware cursors, report-only feedback, and global Ctrl+C exit.
- Fixes database-backed session size aggregation so one SQLite database is counted once instead of once per session row.
- Adds fixture coverage for Cursor, Cherry Studio, and DeepChat fake homes.

## v0.1.0 - 2026-05-15

Initial public release of VibeHauler.

- Supports Claude Code, Codex, Gemini CLI, and Aider.
- Detects app roots across macOS, Linux, Windows-style fixture layouts, and portable fake homes.
- Inventories Green, Yellow, Red, and Black data with conservative risk rules.
- Parses JSONL session logs and Aider Markdown chat history.
- Provides a ratatui/crossterm TUI for app selection, safe cleanup, session review, confirmation, execution, and final restore guidance.
- Moves cleanup targets to OS Trash when available, or VibeHauler quarantine, and writes JSON manifests.
- Backs up Yellow session data before moving it to Trash.
- Guards against symlink escapes, modified sources before mutation, insufficient backup space, and running app processes.
- Includes fixture-backed tests and tmux-driven TUI snapshot coverage.
