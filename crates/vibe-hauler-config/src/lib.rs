#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools
)]

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use vibe_hauler_core::{AppId, OsKind};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VibeHaulerConfig {
    pub general: GeneralConfig,
    #[serde(alias = "scan")]
    pub discovery: DiscoveryConfig,
    pub apps: BTreeMap<String, AppConfig>,
    pub redaction: RedactionConfig,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeneralConfig {
    pub default_days: u32,
    pub use_trash: bool,
    pub backup_before_delete: bool,
    pub local_only: bool,
    pub show_raw_preview: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiscoveryConfig {
    pub follow_symlinks: bool,
    pub max_depth: u8,
    pub include_windows_home_from_wsl: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub custom_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RedactionConfig {
    pub mask_api_keys: bool,
    pub mask_bearer_tokens: bool,
    pub mask_url_credentials: bool,
    pub mask_pii: bool,
    pub max_preview_chars: usize,
}

impl Default for VibeHaulerConfig {
    fn default() -> Self {
        let apps = [
            AppId::Claude,
            AppId::Codex,
            AppId::Gemini,
            AppId::Cursor,
            AppId::CherryStudio,
            AppId::DeepChat,
        ]
        .into_iter()
        .map(|app| {
            (
                app.key().to_owned(),
                AppConfig {
                    enabled: true,
                    custom_roots: Vec::new(),
                },
            )
        })
        .collect();

        Self {
            general: GeneralConfig {
                default_days: 30,
                use_trash: true,
                backup_before_delete: true,
                local_only: true,
                show_raw_preview: false,
            },
            discovery: DiscoveryConfig {
                follow_symlinks: false,
                max_depth: 8,
                include_windows_home_from_wsl: false,
            },
            apps,
            redaction: RedactionConfig {
                mask_api_keys: true,
                mask_bearer_tokens: true,
                mask_url_credentials: true,
                mask_pii: false,
                max_preview_chars: 200,
            },
        }
    }
}

impl VibeHaulerConfig {
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let mut config = Self::default();
        if let Some(path) = path {
            config = merge(config, load_file(path)?);
        } else if let Some(path) = default_config_path(OsKind::current())
            && path.exists()
        {
            config = merge(config, load_file(&path)?);
        }
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.discovery.max_depth == 0 {
            bail!("discovery.max_depth must be greater than zero");
        }
        if self.redaction.max_preview_chars == 0 {
            bail!("redaction.max_preview_chars must be greater than zero");
        }
        if self.discovery.follow_symlinks {
            bail!("discovery.follow_symlinks=true is not supported in v0.1");
        }
        for (app, config) in &self.apps {
            if AppId::from_key(app).key() != app
                && !matches!(AppId::from_key(app), AppId::Unknown(_))
            {
                bail!("unsupported app config key: {app}");
            }
            for root in &config.custom_roots {
                if !root.exists() {
                    bail!("custom root does not exist: {}", root.display());
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn app_enabled(&self, app: &AppId) -> bool {
        self.apps.get(app.key()).is_none_or(|config| config.enabled)
    }

    #[must_use]
    pub fn app_custom_roots(&self, app: &AppId) -> Vec<PathBuf> {
        self.apps
            .get(app.key())
            .map_or_else(Vec::new, |config| config.custom_roots.clone())
    }
}

#[derive(Clone, Debug)]
pub struct FileConfigLoader {
    pub path: Option<PathBuf>,
}

impl ConfigLoader for FileConfigLoader {
    type Error = anyhow::Error;

    fn load(&self) -> Result<VibeHaulerConfig, Self::Error> {
        VibeHaulerConfig::load(self.path.as_deref())
    }
}

pub trait ConfigLoader {
    type Error;

    fn load(&self) -> Result<VibeHaulerConfig, Self::Error>;
}

#[must_use]
pub fn default_config_path(os: OsKind) -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    match os {
        OsKind::MacOS => home.map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("vibe-hauler")
                .join("config.toml")
        }),
        OsKind::Linux | OsKind::Wsl => env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|home| home.join(".config")))
            .map(|dir| dir.join("vibe-hauler").join("config.toml")),
        OsKind::Windows => env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|dir| dir.join("vibe-hauler").join("config.toml")),
    }
}

#[must_use]
pub fn default_data_dir(os: OsKind, portable_root: Option<&Path>) -> Option<PathBuf> {
    if let Some(root) = portable_root {
        return Some(root.join(".vibe-hauler"));
    }

    let home = env::var_os("HOME").map(PathBuf::from);
    match os {
        OsKind::MacOS => home.map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("vibe-hauler")
        }),
        OsKind::Linux | OsKind::Wsl => env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|home| home.join(".local").join("share")))
            .map(|dir| dir.join("vibe-hauler")),
        OsKind::Windows => env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|dir| dir.join("vibe-hauler")),
    }
}

fn load_file(path: &Path) -> anyhow::Result<VibeHaulerConfig> {
    let value = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    toml::from_str(&value).with_context(|| format!("invalid config TOML {}", path.display()))
}

fn merge(mut base: VibeHaulerConfig, file: VibeHaulerConfig) -> VibeHaulerConfig {
    base.general = file.general;
    base.discovery = file.discovery;
    base.redaction = file.redaction;
    for (app, config) in file.apps {
        base.apps.insert(app, config);
    }
    base
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{ConfigLoader, FileConfigLoader, VibeHaulerConfig};

    #[test]
    fn loads_default_config() {
        let config = VibeHaulerConfig::load(None).expect("default config should load");
        assert!(config.general.local_only);
        assert!(config.app_enabled(&vibe_hauler_core::AppId::Claude));
        assert!(config.app_enabled(&vibe_hauler_core::AppId::Cursor));
        assert!(config.app_enabled(&vibe_hauler_core::AppId::CherryStudio));
        assert!(config.app_enabled(&vibe_hauler_core::AppId::DeepChat));
        assert!(
            !config
                .apps
                .contains_key(vibe_hauler_core::AppId::Aider.key())
        );
    }

    #[test]
    fn loads_config_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r"
[general]
default_days = 14
use_trash = true
backup_before_delete = true
local_only = true
show_raw_preview = false

[discovery]
follow_symlinks = false
max_depth = 4
include_windows_home_from_wsl = false

[apps.claude]
enabled = true
custom_roots = []

[redaction]
mask_api_keys = true
mask_bearer_tokens = true
mask_url_credentials = true
mask_pii = false
max_preview_chars = 120
",
        )
        .expect("write config");
        let config = FileConfigLoader { path: Some(path) }
            .load()
            .expect("file config should load");

        assert_eq!(config.general.default_days, 14);
        assert_eq!(config.discovery.max_depth, 4);
        assert_eq!(config.redaction.max_preview_chars, 120);
    }
}
