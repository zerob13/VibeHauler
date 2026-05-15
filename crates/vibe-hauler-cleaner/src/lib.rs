#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use thiserror::Error;
use vibe_hauler_core::{
    AppId, CleanManifest, CleanPlan, ExecutedAction, ExecutionStatus, PlannedAction, RestoreState,
    RiskLevel, now_rfc3339,
};
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionMode {
    DryRun,
    Execute,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreRequest {
    pub manifest_path: PathBuf,
    pub force: bool,
}

pub trait PlanExecutor {
    type Error;

    fn execute(&self, plan: &CleanPlan, mode: ExecutionMode) -> Result<CleanManifest, Self::Error>;
}

pub trait Restorer {
    type Error;

    fn restore(&self, request: RestoreRequest) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug)]
pub struct CleanerConfig {
    pub data_dir: PathBuf,
    pub app_roots: BTreeMap<AppId, Vec<PathBuf>>,
    pub use_trash: bool,
    pub backup_space_override: Option<u64>,
}

impl CleanerConfig {
    #[must_use]
    pub fn manifest_path(&self, manifest: &CleanManifest) -> PathBuf {
        self.data_dir
            .join("manifests")
            .join(format!("{}.json", manifest.id))
    }
}

pub trait ProcessGuard: Clone + Send + Sync {
    fn is_blocked(&self, action: &PlannedAction) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopProcessGuard;

impl ProcessGuard for NoopProcessGuard {
    fn is_blocked(&self, _action: &PlannedAction) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub struct MockProcessGuard {
    blocked_apps: Vec<AppId>,
}

impl MockProcessGuard {
    #[must_use]
    pub fn blocking(blocked_apps: Vec<AppId>) -> Self {
        Self { blocked_apps }
    }
}

impl ProcessGuard for MockProcessGuard {
    fn is_blocked(&self, action: &PlannedAction) -> bool {
        self.blocked_apps.iter().any(|app| app == &action.app)
    }
}

#[derive(Clone, Debug)]
pub struct FilesystemCleaner<G: ProcessGuard = NoopProcessGuard> {
    config: CleanerConfig,
    guard: G,
}

impl FilesystemCleaner<NoopProcessGuard> {
    #[must_use]
    pub fn new(config: CleanerConfig) -> Self {
        Self {
            config,
            guard: NoopProcessGuard,
        }
    }
}

impl<G: ProcessGuard> FilesystemCleaner<G> {
    #[must_use]
    pub fn with_guard(config: CleanerConfig, guard: G) -> Self {
        Self { config, guard }
    }

    fn execute_inner(
        &self,
        plan: &CleanPlan,
        mode: ExecutionMode,
    ) -> Result<CleanManifest, CleanerError> {
        reject_protected_actions(plan)?;
        reject_blocked_yellow(plan, &self.guard)?;
        check_backup_space(plan, self.config.backup_space_override)?;

        let mut executed = Vec::new();
        if mode == ExecutionMode::DryRun {
            return Ok(manifest_for(plan, executed));
        }

        fs::create_dir_all(self.config.data_dir.join("backups"))?;
        fs::create_dir_all(self.config.data_dir.join("manifests"))?;
        fs::create_dir_all(self.config.data_dir.join("quarantine"))?;
        if self.config.use_trash {
            fs::create_dir_all(self.config.data_dir.join("trash"))?;
        }

        for (index, action) in plan.actions.iter().enumerate() {
            if self.guard.is_blocked(action) {
                executed.push(skipped(action, "process guard blocked Green action"));
                continue;
            }
            let source_check = self.preflight_source(action, plan)?;
            if source_check == SourcePreflight::ChangedOrMissing {
                executed.push(skipped(
                    action,
                    "source changed or disappeared after analysis",
                ));
                continue;
            }

            let source_hash = sha256_path(&action.path).ok();
            let backup = if action.backup_required {
                Some(self.backup_action(plan, action, index)?)
            } else {
                None
            };

            let (destination, status) = self.move_to_destination(plan, action, index)?;
            let restore_possible = backup.is_some()
                || matches!(
                    status,
                    ExecutionStatus::MovedToTrash | ExecutionStatus::Quarantined
                );
            executed.push(ExecutedAction {
                source: action.path.clone(),
                destination: Some(destination),
                backup,
                size_bytes: action.size_bytes,
                sha256: source_hash,
                risk: action.risk,
                status,
                restore_possible,
            });
        }

        let manifest = manifest_for(plan, executed);
        let path = self.config.manifest_path(&manifest);
        fs::write(&path, serde_json::to_vec_pretty(&manifest)?)?;
        Ok(manifest)
    }

    fn preflight_source(
        &self,
        action: &PlannedAction,
        plan: &CleanPlan,
    ) -> Result<SourcePreflight, CleanerError> {
        ensure_inside_root(action, &self.config.app_roots)?;
        ensure_no_symlink_escape(action, &self.config.app_roots)?;
        let Ok(metadata) = fs::symlink_metadata(&action.path) else {
            return Ok(SourcePreflight::ChangedOrMissing);
        };
        if current_size(&action.path, &metadata) != action.size_bytes {
            return Ok(SourcePreflight::ChangedOrMissing);
        }
        if modified_after_plan(&metadata, &plan.created_at) {
            return Ok(SourcePreflight::ChangedOrMissing);
        }
        Ok(SourcePreflight::Ok)
    }

    fn backup_action(
        &self,
        plan: &CleanPlan,
        action: &PlannedAction,
        index: usize,
    ) -> Result<PathBuf, CleanerError> {
        let backup = self
            .config
            .data_dir
            .join("backups")
            .join(&plan.id)
            .join(format!("{index:04}-{}", safe_name(&action.path)));
        copy_path(&action.path, &backup)?;
        Ok(backup)
    }

    fn move_to_destination(
        &self,
        plan: &CleanPlan,
        action: &PlannedAction,
        index: usize,
    ) -> Result<(PathBuf, ExecutionStatus), CleanerError> {
        let base = if self.config.use_trash {
            self.config.data_dir.join("trash")
        } else {
            self.config.data_dir.join("quarantine")
        };
        let destination = base
            .join(&plan.id)
            .join(format!("{index:04}-{}", safe_name(&action.path)));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::rename(&action.path, &destination) {
            Ok(()) => Ok((
                destination,
                if self.config.use_trash {
                    ExecutionStatus::MovedToTrash
                } else {
                    ExecutionStatus::Quarantined
                },
            )),
            Err(error) if self.config.use_trash => {
                let quarantine = self
                    .config
                    .data_dir
                    .join("quarantine")
                    .join(&plan.id)
                    .join(format!("{index:04}-{}", safe_name(&action.path)));
                if let Some(parent) = quarantine.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&action.path, &quarantine).map_err(|fallback| {
                    CleanerError::MoveFailed {
                        path: action.path.clone(),
                        first: error.to_string(),
                        fallback: fallback.to_string(),
                    }
                })?;
                Ok((quarantine, ExecutionStatus::Quarantined))
            }
            Err(error) => Err(CleanerError::MoveFailed {
                path: action.path.clone(),
                first: error.to_string(),
                fallback: "quarantine disabled".to_owned(),
            }),
        }
    }
}

impl<G: ProcessGuard> PlanExecutor for FilesystemCleaner<G> {
    type Error = CleanerError;

    fn execute(&self, plan: &CleanPlan, mode: ExecutionMode) -> Result<CleanManifest, Self::Error> {
        self.execute_inner(plan, mode)
    }
}

impl<G: ProcessGuard> Restorer for FilesystemCleaner<G> {
    type Error = CleanerError;

    fn restore(&self, request: RestoreRequest) -> Result<(), Self::Error> {
        let value = fs::read_to_string(&request.manifest_path)?;
        let manifest = serde_json::from_str::<CleanManifest>(&value)?;
        for action in manifest.actions {
            let Some(restore_from) = action.backup.or(action.destination) else {
                continue;
            };
            if action.source.exists() && !request.force {
                return Err(CleanerError::RestoreWouldOverwrite(action.source));
            }
            copy_path(&restore_from, &action.source)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourcePreflight {
    Ok,
    ChangedOrMissing,
}

#[derive(Debug, Error)]
pub enum CleanerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("protected action cannot execute: {0}")]
    Protected(PathBuf),
    #[error("yellow cleanup blocked while app process is active: {0:?}")]
    ProcessGuard(AppId),
    #[error("insufficient backup space: need {needed} bytes, available {available} bytes")]
    InsufficientBackupSpace { needed: u64, available: u64 },
    #[error("source path escapes app root: {path}")]
    SymlinkEscape { path: PathBuf },
    #[error("failed to move {path}: {first}; fallback: {fallback}")]
    MoveFailed {
        path: PathBuf,
        first: String,
        fallback: String,
    },
    #[error("restore would overwrite existing path: {0}")]
    RestoreWouldOverwrite(PathBuf),
}

fn reject_protected_actions(plan: &CleanPlan) -> Result<(), CleanerError> {
    if let Some(action) = plan
        .actions
        .iter()
        .find(|action| matches!(action.risk, RiskLevel::Red | RiskLevel::Black))
    {
        return Err(CleanerError::Protected(action.path.clone()));
    }
    Ok(())
}

fn reject_blocked_yellow<G: ProcessGuard>(plan: &CleanPlan, guard: &G) -> Result<(), CleanerError> {
    if let Some(action) = plan
        .actions
        .iter()
        .find(|action| action.risk == RiskLevel::Yellow && guard.is_blocked(action))
    {
        return Err(CleanerError::ProcessGuard(action.app.clone()));
    }
    Ok(())
}

fn check_backup_space(plan: &CleanPlan, available: Option<u64>) -> Result<(), CleanerError> {
    let needed = plan
        .actions
        .iter()
        .filter(|action| action.backup_required)
        .map(|action| action.size_bytes)
        .sum::<u64>()
        .saturating_add(4096);
    if let Some(available) = available
        && needed > available
    {
        return Err(CleanerError::InsufficientBackupSpace { needed, available });
    }
    Ok(())
}

fn ensure_inside_root(
    action: &PlannedAction,
    roots: &BTreeMap<AppId, Vec<PathBuf>>,
) -> Result<(), CleanerError> {
    let source = action
        .path
        .canonicalize()
        .map_err(|_| CleanerError::SymlinkEscape {
            path: action.path.clone(),
        })?;
    let inside = roots.get(&action.app).is_some_and(|roots| {
        roots
            .iter()
            .filter_map(|root| root.canonicalize().ok())
            .any(|root| source == root || source.starts_with(root))
    });
    if inside {
        Ok(())
    } else {
        Err(CleanerError::SymlinkEscape {
            path: action.path.clone(),
        })
    }
}

fn ensure_no_symlink_escape(
    action: &PlannedAction,
    roots: &BTreeMap<AppId, Vec<PathBuf>>,
) -> Result<(), CleanerError> {
    let Some(app_roots) = roots.get(&action.app) else {
        return Err(CleanerError::SymlinkEscape {
            path: action.path.clone(),
        });
    };
    let canonical_roots = app_roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect::<Vec<_>>();
    for entry in WalkDir::new(&action.path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            let Ok(target) = entry.path().canonicalize() else {
                return Err(CleanerError::SymlinkEscape {
                    path: entry.path().to_path_buf(),
                });
            };
            if !canonical_roots
                .iter()
                .any(|root| target == *root || target.starts_with(root))
            {
                return Err(CleanerError::SymlinkEscape {
                    path: entry.path().to_path_buf(),
                });
            }
        }
    }
    Ok(())
}

fn current_size(path: &Path, metadata: &fs::Metadata) -> u64 {
    if metadata.is_file() || metadata.file_type().is_symlink() {
        return metadata.len();
    }
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(std::fs::Metadata::is_file)
        .map(|metadata| metadata.len())
        .sum()
}

fn modified_after_plan(metadata: &fs::Metadata, plan_created_at: &str) -> bool {
    let Ok(modified) = metadata.modified() else {
        return true;
    };
    let Ok(plan_time) = DateTime::parse_from_rfc3339(plan_created_at) else {
        return true;
    };
    let modified = DateTime::<Utc>::from(modified);
    modified > plan_time.with_timezone(&Utc)
}

fn copy_path(source: &Path, destination: &Path) -> Result<(), CleanerError> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(CleanerError::SymlinkEscape {
            path: source.to_path_buf(),
        });
    }
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let relative = entry.path().strip_prefix(source).unwrap_or(entry.path());
        let target = destination.join(relative);
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            return Err(CleanerError::SymlinkEscape {
                path: entry.path().to_path_buf(),
            });
        }
        if metadata.is_dir() {
            fs::create_dir_all(&target)?;
        } else if metadata.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn sha256_path(path: &Path) -> std::io::Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    let mut hasher = Sha256::new();
    if metadata.is_file() {
        hash_file(path, &mut hasher)?;
    } else {
        let mut files = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(walkdir::DirEntry::into_path)
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            hasher.update(
                file.strip_prefix(path)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .as_bytes(),
            );
            hash_file(&file, &mut hasher)?;
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

fn hash_file(path: &Path, hasher: &mut Sha256) -> std::io::Result<()> {
    let mut file = fs::File::open(path)?;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(())
}

fn safe_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("item")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn skipped(action: &PlannedAction, reason: &str) -> ExecutedAction {
    ExecutedAction {
        source: action.path.clone(),
        destination: None,
        backup: None,
        size_bytes: action.size_bytes,
        sha256: None,
        risk: action.risk,
        status: ExecutionStatus::Skipped,
        restore_possible: false,
    }
    .with_reason(reason)
}

trait ExecutedActionExt {
    fn with_reason(self, _reason: &str) -> Self;
}

impl ExecutedActionExt for ExecutedAction {
    fn with_reason(self, _reason: &str) -> Self {
        self
    }
}

fn manifest_for(plan: &CleanPlan, actions: Vec<ExecutedAction>) -> CleanManifest {
    let restore_state = restore_state(&actions);
    CleanManifest {
        id: manifest_id(plan),
        plan_id: plan.id.clone(),
        created_at: now_rfc3339(),
        platform: plan.platform,
        actions,
        restore_state,
    }
}

fn restore_state(actions: &[ExecutedAction]) -> RestoreState {
    if actions.is_empty() {
        return RestoreState::NotNeeded;
    }
    let restorable = actions
        .iter()
        .filter(|action| action.restore_possible)
        .count();
    if restorable == actions.len() {
        RestoreState::Restorable
    } else if restorable > 0 {
        RestoreState::PartiallyRestorable
    } else {
        RestoreState::NotRestorable
    }
}

fn manifest_id(plan: &CleanPlan) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plan.id.as_bytes());
    hasher.update(now_rfc3339().as_bytes());
    format!("manifest_{}", &hex::encode(hasher.finalize())[..16])
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, path::PathBuf};

    use tempfile::tempdir;
    use vibe_hauler_core::{
        AppId, CleanPlan, InventoryItem, ItemKind, OsKind, PlanMode, PlanPolicy, Recommendation,
        RiskLevel,
    };

    use super::{
        CleanerConfig, CleanerError, ExecutionMode, FilesystemCleaner, MockProcessGuard,
        PlanExecutor, RestoreRequest, Restorer,
    };

    fn item(path: PathBuf, size: u64, risk: RiskLevel) -> InventoryItem {
        InventoryItem {
            id: "item".to_owned(),
            app: AppId::Codex,
            instance_id: "inst".to_owned(),
            path,
            kind: if risk == RiskLevel::Green {
                ItemKind::Cache
            } else {
                ItemKind::Session
            },
            size_bytes: size,
            modified_at: None,
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

    fn config(data_dir: PathBuf, root: PathBuf) -> CleanerConfig {
        let mut app_roots = BTreeMap::new();
        app_roots.insert(AppId::Codex, vec![root]);
        CleanerConfig {
            data_dir,
            app_roots,
            use_trash: true,
            backup_space_override: Some(u64::MAX),
        }
    }

    #[test]
    fn dry_run_does_not_mutate_or_write_files() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join(".codex");
        fs::create_dir_all(&root).expect("root");
        let cache = root.join("cache.bin");
        fs::write(&cache, b"cache").expect("cache");
        let plan = CleanPlan::build_with_created_at(
            &[item(cache.clone(), 5, RiskLevel::Green)],
            OsKind::Linux,
            PlanMode::SafeCleanup,
            &PlanPolicy::safe_cleanup(),
            "2999-01-01T00:00:00Z",
        )
        .expect("plan");
        let cleaner = FilesystemCleaner::new(config(dir.path().join("data"), root));
        let manifest = cleaner
            .execute(&plan, ExecutionMode::DryRun)
            .expect("dry run");

        assert!(cache.exists());
        assert_eq!(manifest.actions.len(), 0);
        assert!(!dir.path().join("data").exists());
    }

    #[test]
    fn backup_trash_manifest_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join(".codex");
        fs::create_dir_all(&root).expect("root");
        let history = root.join("history.jsonl");
        fs::write(&history, b"session").expect("history");
        let plan = CleanPlan::build_with_created_at(
            &[item(history.clone(), 7, RiskLevel::Yellow)],
            OsKind::Linux,
            PlanMode::SessionCleanup,
            &PlanPolicy::session_cleanup(),
            "2999-01-01T00:00:00Z",
        )
        .expect("plan");
        let data_dir = dir.path().join("data");
        let cleaner_config = config(data_dir.clone(), root);
        let cleaner = FilesystemCleaner::new(cleaner_config.clone());
        let manifest = cleaner
            .execute(&plan, ExecutionMode::Execute)
            .expect("execute");
        let manifest_path = cleaner_config.manifest_path(&manifest);

        assert!(!history.exists());
        assert_eq!(manifest.actions.len(), 1);
        assert!(
            manifest.actions[0]
                .backup
                .as_ref()
                .is_some_and(|path| path.exists())
        );
        assert!(data_dir.join("manifests").exists());

        cleaner
            .restore(RestoreRequest {
                manifest_path,
                force: false,
            })
            .expect("restore");
        assert!(history.exists());
    }

    #[test]
    fn rejects_symlink_escape() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join(".codex");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&root).expect("root");
        fs::write(&outside, b"secret").expect("outside");
        let link = root.join("cache-link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).expect("symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, &link).expect("symlink");
        let plan = CleanPlan::build_with_created_at(
            &[item(link, 6, RiskLevel::Green)],
            OsKind::Linux,
            PlanMode::SafeCleanup,
            &PlanPolicy::safe_cleanup(),
            "2999-01-01T00:00:00Z",
        )
        .expect("plan");
        let cleaner = FilesystemCleaner::new(config(dir.path().join("data"), root));
        let error = cleaner
            .execute(&plan, ExecutionMode::Execute)
            .expect_err("symlink escape rejected");

        assert!(matches!(error, CleanerError::SymlinkEscape { .. }));
    }

    #[test]
    fn process_guard_mock_blocks_yellow() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join(".codex");
        fs::create_dir_all(&root).expect("root");
        let history = root.join("history.jsonl");
        fs::write(&history, b"session").expect("history");
        let plan = CleanPlan::build_with_created_at(
            &[item(history, 7, RiskLevel::Yellow)],
            OsKind::Linux,
            PlanMode::SessionCleanup,
            &PlanPolicy::session_cleanup(),
            "2999-01-01T00:00:00Z",
        )
        .expect("plan");
        let cleaner = FilesystemCleaner::with_guard(
            config(dir.path().join("data"), root),
            MockProcessGuard::blocking(vec![AppId::Codex]),
        );
        let error = cleaner
            .execute(&plan, ExecutionMode::Execute)
            .expect_err("guard blocks");

        assert!(matches!(error, CleanerError::ProcessGuard(AppId::Codex)));
    }

    #[test]
    fn insufficient_backup_space_blocks_yellow_cleanup() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join(".codex");
        fs::create_dir_all(&root).expect("root");
        let history = root.join("history.jsonl");
        fs::write(&history, b"session").expect("history");
        let plan = CleanPlan::build_with_created_at(
            &[item(history.clone(), 7, RiskLevel::Yellow)],
            OsKind::Linux,
            PlanMode::SessionCleanup,
            &PlanPolicy::session_cleanup(),
            "2999-01-01T00:00:00Z",
        )
        .expect("plan");
        let mut config = config(dir.path().join("data"), root);
        config.backup_space_override = Some(1);
        let cleaner = FilesystemCleaner::new(config);
        let error = cleaner
            .execute(&plan, ExecutionMode::Execute)
            .expect_err("backup space should block");

        assert!(matches!(
            error,
            CleanerError::InsufficientBackupSpace { .. }
        ));
        assert!(history.exists());
    }
}
