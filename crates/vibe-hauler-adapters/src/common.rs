use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use vibe_hauler_core::{
    AppId, AppInstance, DetectionConfidence, InventoryItem, ItemKind, Recommendation, RiskEngine,
    RiskInput, RiskLevel, RootKind,
};
use vibe_hauler_discovery::{CandidateRoot, PathContext, RootResolver, StandardRootResolver};
use walkdir::WalkDir;

pub fn detect_from_roots(
    app: &AppId,
    display_name: &str,
    ctx: &PathContext,
    evidence_files: &[&str],
) -> anyhow::Result<Vec<AppInstance>> {
    let resolver = StandardRootResolver::default();
    let mut instances = Vec::new();
    for candidate in resolver.candidate_roots(app, ctx)? {
        if let Some(instance) = candidate_to_instance(candidate, display_name, ctx, evidence_files)
        {
            instances.push(instance);
        }
    }
    instances.sort_by(|left, right| {
        left.app
            .cmp(&right.app)
            .then_with(|| right.confidence.cmp(&left.confidence))
            .then_with(|| left.root.cmp(&right.root))
    });
    instances.dedup_by(|left, right| left.app == right.app && left.root == right.root);
    Ok(instances)
}

fn candidate_to_instance(
    candidate: CandidateRoot,
    display_name: &str,
    ctx: &PathContext,
    evidence_files: &[&str],
) -> Option<AppInstance> {
    if !candidate.path.exists() {
        return None;
    }
    let root = candidate.path.canonicalize().ok()?;
    let mut evidence = candidate.evidence;
    let exact = evidence_files.iter().any(|path| {
        let found = root.join(path).exists();
        if found {
            evidence.push(format!("found {path}"));
        }
        found
    });
    let confidence = match (candidate.confidence, exact) {
        (DetectionConfidence::UserProvided, _) => DetectionConfidence::UserProvided,
        (_, true) => DetectionConfidence::Exact,
        (DetectionConfidence::Weak, false) => DetectionConfidence::Strong,
        (confidence, false) => confidence,
    };
    Some(AppInstance {
        id: instance_id(&candidate.app, &root),
        app: candidate.app,
        display_name: display_name.to_owned(),
        root,
        root_kind: candidate.kind,
        platform: ctx.os,
        confidence,
        evidence,
    })
}

pub fn inventory_item(
    instance: &AppInstance,
    path: PathBuf,
    kind: ItemKind,
    parser: Option<&str>,
    evidence: Vec<String>,
) -> anyhow::Result<InventoryItem> {
    let metadata = fs::symlink_metadata(&path)?;
    let size_bytes = path_size(&path);
    let modified_at = metadata
        .modified()
        .ok()
        .map(|time| chrono::DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true));
    let assessment = RiskEngine::classify(&RiskInput {
        path: &path,
        kind,
        evidence: &evidence,
    });
    Ok(InventoryItem {
        id: item_id(
            &instance.app,
            &path,
            kind,
            size_bytes,
            modified_at.as_deref(),
        ),
        app: instance.app.clone(),
        instance_id: instance.id.clone(),
        path,
        kind,
        size_bytes,
        modified_at,
        risk: assessment.risk,
        recommendation: assessment.recommendation,
        reason: assessment.reason,
        backup_required: assessment.backup_required,
        parser: parser.map(str::to_owned),
        evidence,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn forced_inventory_item(
    instance: &AppInstance,
    path: PathBuf,
    kind: ItemKind,
    risk: RiskLevel,
    recommendation: Recommendation,
    reason: &str,
    parser: Option<&str>,
    evidence: Vec<String>,
) -> anyhow::Result<InventoryItem> {
    let metadata = fs::symlink_metadata(&path)?;
    let size_bytes = path_size(&path);
    let modified_at = metadata
        .modified()
        .ok()
        .map(|time| chrono::DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true));
    Ok(InventoryItem {
        id: item_id(
            &instance.app,
            &path,
            kind,
            size_bytes,
            modified_at.as_deref(),
        ),
        app: instance.app.clone(),
        instance_id: instance.id.clone(),
        path,
        kind,
        size_bytes,
        modified_at,
        risk,
        recommendation,
        reason: reason.to_owned(),
        backup_required: risk == RiskLevel::Yellow,
        parser: parser.map(str::to_owned),
        evidence,
    })
}

pub fn existing_child(root: &Path, child: &str) -> Option<PathBuf> {
    let path = root.join(child);
    path.exists().then_some(path)
}

pub fn existing_descendant(root: &Path, parts: &[&str]) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for part in parts {
        path.push(part);
    }
    path.exists().then_some(path)
}

pub fn find_files(root: &Path, names: &[&str], extensions: &[&str]) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| names.iter().any(|candidate| candidate == &name))
                || path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| extensions.iter().any(|candidate| candidate == &ext))
        })
        .collect()
}

pub fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_file() {
        return metadata.len();
    }
    if metadata.file_type().is_symlink() {
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

pub fn instance_id(app: &AppId, root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(app.key().as_bytes());
    hasher.update(root.to_string_lossy().as_bytes());
    format!("inst_{}", &hex::encode(hasher.finalize())[..16])
}

fn item_id(
    app: &AppId,
    path: &Path,
    kind: ItemKind,
    size_bytes: u64,
    modified_at: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(app.key().as_bytes());
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(format!("{kind:?}").as_bytes());
    hasher.update(size_bytes.to_le_bytes());
    if let Some(modified_at) = modified_at {
        hasher.update(modified_at.as_bytes());
    }
    format!("item_{}", &hex::encode(hasher.finalize())[..16])
}

#[allow(dead_code)]
pub fn app_instance_from_root(
    app: AppId,
    root: PathBuf,
    kind: RootKind,
    os: vibe_hauler_core::OsKind,
) -> AppInstance {
    let root = root.canonicalize().unwrap_or(root);
    AppInstance {
        id: instance_id(&app, &root),
        display_name: app.display_name().to_owned(),
        app,
        root,
        root_kind: kind,
        platform: os,
        confidence: DetectionConfidence::UserProvided,
        evidence: vec!["test root".to_owned()],
    }
}
