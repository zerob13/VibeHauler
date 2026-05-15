# Discovery Module

Cargo package: `vibe-hauler-discovery`

## Responsibility

Discovery finds candidate app roots without parsing full app data.

It owns:

- platform detection;
- home/config/data/cache/state directories;
- environment variable overrides;
- WSL handling;
- user-provided roots;
- confidence scoring.

It does not classify cleanup risk or delete anything.

## PathContext

```rust
pub struct PathContext {
    pub os: OsKind,
    pub home: PathBuf,
    pub config_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
    pub state_dir: Option<PathBuf>,
    pub app_data_roaming: Option<PathBuf>,
    pub app_data_local: Option<PathBuf>,
    pub portable_root: Option<PathBuf>,
}
```

## Detection Flow

1. Build `PathContext`.
2. Load environment overrides such as `CLAUDE_CONFIG_DIR` and `CODEX_HOME`.
3. Generate known roots for each app.
4. Apply user roots from `--roots app=path`.
5. Fingerprint each root with lightweight checks.
6. Return `AppInstance` values sorted by confidence.

## Confidence

| Confidence | Meaning |
|---|---|
| `Exact` | official env var or unmistakable app files |
| `Strong` | known app directory with expected children |
| `Weak` | plausible directory name or partial evidence |
| `UserProvided` | explicit user path |

User-provided roots should not skip safety checks; they only affect discovery confidence.

## WSL Strategy

Default: treat WSL as Linux and discover Linux paths only.

Optional:

```toml
[discovery]
include_windows_home_from_wsl = true
```

When enabled, discover `/mnt/c/Users/<name>` candidates and mark them as `windows-from-wsl`. Cleanup defaults to read-only review for these roots.

## Testing

Fixtures:

```txt
fixtures/macos-home/
fixtures/linux-home/
fixtures/windows-home/
fixtures/wsl-home/
```

Tests must override `HOME`, XDG dirs, and Windows env vars with fixture paths.
