#![allow(clippy::missing_errors_doc)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vibe_hauler_core::{AppId, DetectionConfidence, OsKind, RootKind};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateRoot {
    pub app: AppId,
    pub path: PathBuf,
    pub kind: RootKind,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}

pub trait RootResolver {
    type Error;

    fn candidate_roots(
        &self,
        app: &AppId,
        ctx: &PathContext,
    ) -> Result<Vec<CandidateRoot>, Self::Error>;
}
