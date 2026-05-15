# Config Module

Cargo package: `vibe-hauler-config`

## Responsibility

Config loads, validates, and merges VibeHauler settings.

Order of precedence:

1. CLI flags.
2. Environment variables.
3. Config file.
4. Built-in defaults.

## Config Paths

| OS | Config Path |
|---|---|
| macOS | `~/Library/Application Support/vibe-hauler/config.toml` or `~/.config/vibe-hauler/config.toml` |
| Linux | `${XDG_CONFIG_HOME:-~/.config}/vibe-hauler/config.toml` |
| Windows | `%APPDATA%\vibe-hauler\config.toml` |

Data paths:

| OS | Data Path |
|---|---|
| macOS | `~/Library/Application Support/vibe-hauler/` |
| Linux | `${XDG_DATA_HOME:-~/.local/share}/vibe-hauler/` |
| Windows | `%APPDATA%\vibe-hauler\` |

## Example

```toml
[general]
default_days = 30
use_trash = true
backup_before_delete = true
local_only = true
show_raw_preview = false

[scan]
follow_symlinks = false
max_depth = 8
include_windows_home_from_wsl = false

[apps.claude]
enabled = true
custom_roots = []

[apps.codex]
enabled = true
codex_home = ""

[apps.cursor]
enabled = true
advanced_sqlite_cleanup = false

[apps.cherry]
enabled = true
backup_files = []
indexeddb_subset_parser = true

[redaction]
mask_api_keys = true
mask_bearer_tokens = true
mask_url_credentials = true
mask_pii = false
max_preview_chars = 200
```

## Validation

Config validation should reject:

- negative durations;
- non-existent custom roots unless explicitly allowed;
- `follow_symlinks = true` without warning state;
- advanced cleanup enabled for unsupported apps;
- raw previews enabled in global config without a warning in `doctor`.

## Testing

- load default config;
- merge CLI over file;
- platform path tests;
- invalid TOML errors with helpful messages.

