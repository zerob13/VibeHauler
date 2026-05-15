#![allow(clippy::missing_errors_doc, clippy::struct_excessive_bools)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VibeHaulerConfig {
    pub general: GeneralConfig,
    pub scan: ScanConfig,
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
pub struct ScanConfig {
    pub follow_symlinks: bool,
    pub max_depth: u8,
    pub include_windows_home_from_wsl: bool,
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

pub trait ConfigLoader {
    type Error;

    fn load(&self) -> Result<VibeHaulerConfig, Self::Error>;
}
