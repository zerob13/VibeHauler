use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ids::{ManifestId, PlanId};
use crate::models::{AgentSession, AppId, InventoryItem, OsKind, SessionSource};
use crate::risk::{Recommendation, RiskLevel};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

impl CleanPlan {
    pub fn build(
        items: &[InventoryItem],
        platform: OsKind,
        mode: PlanMode,
        policy: &PlanPolicy,
    ) -> Result<Self, PlanError> {
        let created_at = now_rfc3339();
        Self::build_with_created_at(items, platform, mode, policy, &created_at)
    }

    pub fn build_with_created_at(
        items: &[InventoryItem],
        platform: OsKind,
        mode: PlanMode,
        policy: &PlanPolicy,
        created_at: &str,
    ) -> Result<Self, PlanError> {
        let mut warnings = Vec::new();
        let mut actions = Vec::new();

        for item in items {
            if !policy.matches_app(&item.app) || !policy.matches_age(item.modified_at.as_deref()) {
                continue;
            }

            let include = match item.risk {
                RiskLevel::Green => policy.include_green,
                RiskLevel::Yellow => policy.include_yellow,
                RiskLevel::Red | RiskLevel::Black => false,
            };

            if !include {
                continue;
            }

            if item.risk == RiskLevel::Yellow
                && policy.require_backup_for_yellow
                && !item.backup_required
            {
                return Err(PlanError::YellowWithoutBackup {
                    path: item.path.clone(),
                });
            }

            if matches!(item.risk, RiskLevel::Red | RiskLevel::Black) {
                return Err(PlanError::ProtectedAction {
                    path: item.path.clone(),
                    risk: item.risk,
                });
            }

            actions.push(PlannedAction {
                app: item.app.clone(),
                kind: action_kind(item.risk, item.backup_required),
                path: item.path.clone(),
                risk: item.risk,
                size_bytes: item.size_bytes,
                backup_required: item.backup_required,
                reason: item.reason.clone(),
                expected_modified_at: item.modified_at.clone(),
            });
        }

        actions.sort_by(|left, right| {
            left.app
                .cmp(&right.app)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.risk.cmp(&right.risk))
        });
        actions.dedup_by(|left, right| left.path == right.path && left.app == right.app);
        remove_nested_actions(&mut actions, &mut warnings);

        let totals = PlanTotals::from_actions(&actions);
        let id = plan_id(created_at, &actions);

        Ok(Self {
            id,
            created_at: created_at.to_owned(),
            version: VERSION.to_owned(),
            platform,
            mode,
            actions,
            totals,
            warnings,
        })
    }

    pub fn build_for_sessions(
        sessions: &[AgentSession],
        platform: OsKind,
        created_at: &str,
    ) -> Result<Self, PlanError> {
        let items = sessions
            .iter()
            .filter_map(session_as_item)
            .collect::<Vec<_>>();
        Self::build_with_created_at(
            &items,
            platform,
            PlanMode::SessionCleanup,
            &PlanPolicy {
                include_green: false,
                include_yellow: true,
                older_than: None,
                apps: None,
                require_backup_for_yellow: true,
            },
            created_at,
        )
    }

    #[must_use]
    pub fn to_pretty_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_owned())
    }
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
    pub expected_modified_at: Option<String>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanPolicy {
    pub include_green: bool,
    pub include_yellow: bool,
    pub older_than: Option<Duration>,
    pub apps: Option<Vec<AppId>>,
    pub require_backup_for_yellow: bool,
}

impl PlanPolicy {
    #[must_use]
    pub const fn safe_cleanup() -> Self {
        Self {
            include_green: true,
            include_yellow: false,
            older_than: None,
            apps: None,
            require_backup_for_yellow: true,
        }
    }

    #[must_use]
    pub const fn session_cleanup() -> Self {
        Self {
            include_green: false,
            include_yellow: true,
            older_than: None,
            apps: None,
            require_backup_for_yellow: true,
        }
    }

    fn matches_app(&self, app: &AppId) -> bool {
        self.apps
            .as_ref()
            .is_none_or(|apps| apps.iter().any(|candidate| candidate == app))
    }

    fn matches_age(&self, modified_at: Option<&str>) -> bool {
        let Some(age) = self.older_than else {
            return true;
        };
        let Some(modified_at) = modified_at else {
            return false;
        };
        let Ok(modified_at) = DateTime::parse_from_rfc3339(modified_at) else {
            return false;
        };
        let Ok(age) = chrono::Duration::from_std(age) else {
            return false;
        };
        Utc::now().signed_duration_since(modified_at.with_timezone(&Utc)) >= age
    }
}

impl PlanTotals {
    #[must_use]
    pub fn from_actions(actions: &[PlannedAction]) -> Self {
        let mut totals = Self {
            actions: actions.len() as u64,
            files: actions.len() as u64,
            bytes: 0,
            green: 0,
            yellow: 0,
        };
        for action in actions {
            totals.bytes = totals.bytes.saturating_add(action.size_bytes);
            match action.risk {
                RiskLevel::Green => totals.green += 1,
                RiskLevel::Yellow => totals.yellow += 1,
                RiskLevel::Red | RiskLevel::Black => {}
            }
        }
        totals
    }
}

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("protected item cannot be planned: {path} ({risk:?})")]
    ProtectedAction { path: PathBuf, risk: RiskLevel },
    #[error("yellow item requires backup before cleanup: {path}")]
    YellowWithoutBackup { path: PathBuf },
}

fn action_kind(risk: RiskLevel, backup_required: bool) -> ActionKind {
    match (risk, backup_required) {
        (RiskLevel::Green, _) => ActionKind::Trash,
        (RiskLevel::Yellow, true) => ActionKind::BackupAndTrash,
        (RiskLevel::Yellow, false) | (RiskLevel::Red | RiskLevel::Black, _) => {
            ActionKind::ReportOnly
        }
    }
}

fn remove_nested_actions(actions: &mut Vec<PlannedAction>, warnings: &mut Vec<PlanWarning>) {
    let mut filtered = Vec::with_capacity(actions.len());
    for action in actions.drain(..) {
        let nested = filtered.iter().any(|parent: &PlannedAction| {
            action.app == parent.app
                && action.path != parent.path
                && action.path.starts_with(&parent.path)
        });
        if nested {
            warnings.push(PlanWarning {
                code: "nested-path-skipped".to_owned(),
                message: format!("Skipped nested cleanup path {}", action.path.display()),
            });
        } else {
            filtered.push(action);
        }
    }
    *actions = filtered;
}

fn session_as_item(session: &AgentSession) -> Option<InventoryItem> {
    let path = match &session.source {
        SessionSource::File(path)
        | SessionSource::Directory(path)
        | SessionSource::Database(path)
        | SessionSource::Backup(path) => path.clone(),
        SessionSource::Unknown => return None,
    };
    Some(InventoryItem {
        id: session.id.clone(),
        app: session.app.clone(),
        instance_id: format!("{}_session", session.app.key()),
        path,
        kind: crate::models::ItemKind::Session,
        size_bytes: session.size_bytes,
        modified_at: session.updated_at.clone(),
        risk: RiskLevel::Yellow,
        recommendation: Recommendation::Review,
        reason: "selected session history requires backup before trash".to_owned(),
        backup_required: true,
        parser: Some(session.parser.clone()),
        evidence: vec!["session cleanup selection".to_owned()],
    })
}

fn plan_id(created_at: &str, actions: &[PlannedAction]) -> PlanId {
    let mut hasher = Sha256::new();
    hasher.update(created_at.as_bytes());
    for action in actions {
        hasher.update(action.app.key().as_bytes());
        hasher.update(action.path.to_string_lossy().as_bytes());
        hasher.update(action.size_bytes.to_le_bytes());
        hasher.update(format!("{:?}", action.risk).as_bytes());
    }
    format!("plan_{}", &hex::encode(hasher.finalize())[..16])
}

#[must_use]
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{CleanPlan, PlanMode, PlanPolicy};
    use crate::{
        models::{AppId, InventoryItem, ItemKind, OsKind},
        risk::{Recommendation, RiskLevel},
    };

    fn item(path: &str, risk: RiskLevel, size: u64) -> InventoryItem {
        InventoryItem {
            id: path.to_owned(),
            app: AppId::Codex,
            instance_id: "codex_fixture".to_owned(),
            path: PathBuf::from(path),
            kind: if risk == RiskLevel::Green {
                ItemKind::Cache
            } else {
                ItemKind::Session
            },
            size_bytes: size,
            modified_at: Some("2026-05-15T10:00:00Z".to_owned()),
            risk,
            recommendation: if risk == RiskLevel::Green {
                Recommendation::Clean
            } else {
                Recommendation::Review
            },
            reason: "test".to_owned(),
            backup_required: risk == RiskLevel::Yellow,
            parser: None,
            evidence: Vec::new(),
        }
    }

    #[test]
    fn builds_deterministic_safe_plan_and_serializes() {
        let plan = CleanPlan::build_with_created_at(
            &[
                item("/tmp/home/.codex/history.jsonl", RiskLevel::Yellow, 10),
                item("/tmp/home/.codex/cache", RiskLevel::Green, 20),
            ],
            OsKind::Linux,
            PlanMode::SafeCleanup,
            &PlanPolicy::safe_cleanup(),
            "2026-05-15T10:00:00Z",
        )
        .expect("plan should build");

        assert_eq!(plan.totals.actions, 1);
        assert_eq!(plan.totals.green, 1);
        assert!(plan.to_pretty_json().contains("\"mode\": \"SafeCleanup\""));
    }

    #[test]
    fn skips_nested_actions_with_warning() {
        let plan = CleanPlan::build_with_created_at(
            &[
                item("/tmp/home/.codex/cache", RiskLevel::Green, 20),
                item("/tmp/home/.codex/cache/file.bin", RiskLevel::Green, 2),
            ],
            OsKind::Linux,
            PlanMode::SafeCleanup,
            &PlanPolicy::safe_cleanup(),
            "2026-05-15T10:00:00Z",
        )
        .expect("plan should build");

        assert_eq!(plan.totals.actions, 1);
        assert_eq!(plan.warnings[0].code, "nested-path-skipped");
    }
}
