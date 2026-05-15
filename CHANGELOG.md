# Changelog

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
