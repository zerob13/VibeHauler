use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{ManifestId, PlanId};
use crate::models::{AppId, OsKind};
use crate::risk::RiskLevel;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanPlan {
    pub id: PlanId,
    pub created_at: String,
    pub version: String,
    pub platform: OsKind,
    pub mode: PlanMode,
    pub actions: Vec<PlannedAction>,
    pub totals: PlanTotals,
    pub warnings: Vec<PlanWarning>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlanMode {
    SafeCleanup,
    SessionCleanup,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlanTotals {
    pub actions: u64,
    pub files: u64,
    pub bytes: u64,
    pub green: u64,
    pub yellow: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlanWarning {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlannedAction {
    pub app: AppId,
    pub kind: ActionKind,
    pub path: PathBuf,
    pub risk: RiskLevel,
    pub size_bytes: u64,
    pub backup_required: bool,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActionKind {
    Trash,
    BackupAndTrash,
    Quarantine,
    ReportOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanManifest {
    pub id: ManifestId,
    pub plan_id: PlanId,
    pub created_at: String,
    pub platform: OsKind,
    pub actions: Vec<ExecutedAction>,
    pub restore_state: RestoreState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RestoreState {
    NotNeeded,
    Restorable,
    PartiallyRestorable,
    NotRestorable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutedAction {
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    pub backup: Option<PathBuf>,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub risk: RiskLevel,
    pub status: ExecutionStatus,
    pub restore_possible: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExecutionStatus {
    MovedToTrash,
    Quarantined,
    Skipped,
    Failed,
}
