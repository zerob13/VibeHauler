use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{ManifestId, PlanId};
use crate::models::AppId;
use crate::risk::RiskLevel;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanPlan {
    pub id: PlanId,
    pub created_at: String,
    pub version: String,
    pub actions: Vec<PlannedAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlannedAction {
    pub app: AppId,
    pub path: PathBuf,
    pub risk: RiskLevel,
    pub backup_required: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanManifest {
    pub id: ManifestId,
    pub plan_id: PlanId,
    pub created_at: String,
    pub actions: Vec<ExecutedAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutedAction {
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    pub backup: Option<PathBuf>,
    pub sha256: Option<String>,
    pub risk: RiskLevel,
    pub restore_possible: bool,
}
