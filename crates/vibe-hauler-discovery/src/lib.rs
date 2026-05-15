#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use vibe_hauler_core::{AppId, DetectionConfidence, OsKind, RootKind};
use walkdir::WalkDir;

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

impl PathContext {
    pub fn from_env() -> anyhow::Result<Self> {
        let os = OsKind::current();
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
            .ok_or_else(|| anyhow::anyhow!("HOME/USERPROFILE is not set"))?;
        Ok(Self::for_home(home, os, None))
    }

    #[must_use]
    pub fn for_portable_root(root: impl Into<PathBuf>, os: OsKind) -> Self {
        let root = root.into();
        Self::for_home(root.clone(), os, Some(root))
    }

    #[must_use]
    pub fn for_home(home: impl AsRef<Path>, os: OsKind, portable_root: Option<PathBuf>) -> Self {
        let home = home.as_ref().to_path_buf();
        match os {
            OsKind::MacOS => Self {
                os,
                home: home.clone(),
                config_dir: Some(home.join("Library").join("Application Support")),
                data_dir: Some(home.join("Library").join("Application Support")),
                cache_dir: Some(home.join("Library").join("Caches")),
                state_dir: None,
                app_data_roaming: None,
                app_data_local: None,
                portable_root,
            },
            OsKind::Linux | OsKind::Wsl => Self {
                os,
                home: home.clone(),
                config_dir: Some(home.join(".config")),
                data_dir: Some(home.join(".local").join("share")),
                cache_dir: Some(home.join(".cache")),
                state_dir: Some(home.join(".local").join("state")),
                app_data_roaming: None,
                app_data_local: None,
                portable_root,
            },
            OsKind::Windows => Self {
                os,
                home: home.clone(),
                config_dir: Some(home.join("AppData").join("Roaming")),
                data_dir: Some(home.join("AppData").join("Roaming")),
                cache_dir: Some(home.join("AppData").join("Local")),
                state_dir: None,
                app_data_roaming: Some(home.join("AppData").join("Roaming")),
                app_data_local: Some(home.join("AppData").join("Local")),
                portable_root,
            },
        }
    }

    #[must_use]
    pub fn infer_portable_os(path: &Path) -> OsKind {
        let text = path.to_string_lossy().to_ascii_lowercase();
        if text.contains("windows") {
            OsKind::Windows
        } else if text.contains("macos") || text.contains("darwin") {
            OsKind::MacOS
        } else {
            OsKind::Linux
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateRoot {
    pub app: AppId,
    pub path: PathBuf,
    pub kind: RootKind,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}

impl CandidateRoot {
    #[must_use]
    pub fn user_provided(app: AppId, path: PathBuf) -> Self {
        Self {
            app,
            path,
            kind: RootKind::UserProvided,
            confidence: DetectionConfidence::UserProvided,
            evidence: vec!["user provided root".to_owned()],
        }
    }
}

pub trait RootResolver {
    type Error;

    fn candidate_roots(
        &self,
        app: &AppId,
        ctx: &PathContext,
    ) -> Result<Vec<CandidateRoot>, Self::Error>;
}

#[derive(Clone, Debug, Default)]
pub struct StandardRootResolver {
    overrides: BTreeMap<AppId, Vec<PathBuf>>,
}

impl StandardRootResolver {
    #[must_use]
    pub fn new(overrides: BTreeMap<AppId, Vec<PathBuf>>) -> Self {
        Self { overrides }
    }

    fn known_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        match app {
            AppId::Claude => {
                push(
                    &mut roots,
                    app,
                    ctx.home.join(".claude"),
                    RootKind::Config,
                    "home .claude",
                );
                if let Some(config_dir) = &ctx.config_dir {
                    push(
                        &mut roots,
                        app,
                        config_dir.join("claude"),
                        RootKind::Config,
                        "config dir",
                    );
                    push(
                        &mut roots,
                        app,
                        config_dir.join("Claude"),
                        RootKind::Config,
                        "config dir",
                    );
                }
            }
            AppId::Codex => {
                let env_home = env::var_os("CODEX_HOME").map(PathBuf::from);
                if let Some(env_home) = env_home {
                    roots.push(CandidateRoot {
                        app: app.clone(),
                        path: env_home,
                        kind: RootKind::Config,
                        confidence: DetectionConfidence::Exact,
                        evidence: vec!["CODEX_HOME".to_owned()],
                    });
                }
                push(
                    &mut roots,
                    app,
                    ctx.home.join(".codex"),
                    RootKind::Config,
                    "home .codex",
                );
                if let Some(config_dir) = &ctx.config_dir {
                    push(
                        &mut roots,
                        app,
                        config_dir.join("codex"),
                        RootKind::Config,
                        "config dir",
                    );
                    push(
                        &mut roots,
                        app,
                        config_dir.join("Codex"),
                        RootKind::Config,
                        "config dir",
                    );
                }
            }
            AppId::Gemini => {
                push(
                    &mut roots,
                    app,
                    ctx.home.join(".gemini"),
                    RootKind::Config,
                    "home .gemini",
                );
                if let Some(config_dir) = &ctx.config_dir {
                    push(
                        &mut roots,
                        app,
                        config_dir.join("gemini"),
                        RootKind::Config,
                        "config dir",
                    );
                    push(
                        &mut roots,
                        app,
                        config_dir.join("Gemini"),
                        RootKind::Config,
                        "config dir",
                    );
                }
            }
            AppId::Aider => {
                roots.extend(find_aider_repos(ctx));
            }
            _ => {}
        }
        roots
    }
}

impl RootResolver for StandardRootResolver {
    type Error = anyhow::Error;

    fn candidate_roots(
        &self,
        app: &AppId,
        ctx: &PathContext,
    ) -> Result<Vec<CandidateRoot>, Self::Error> {
        let mut roots = Self::known_roots(app, ctx);
        if let Some(overrides) = self.overrides.get(app) {
            roots.extend(
                overrides
                    .iter()
                    .cloned()
                    .map(|path| CandidateRoot::user_provided(app.clone(), path)),
            );
        }
        roots.sort_by(|left, right| {
            left.app
                .cmp(&right.app)
                .then_with(|| right.confidence.cmp(&left.confidence))
                .then_with(|| left.path.cmp(&right.path))
        });
        roots.dedup_by(|left, right| left.path == right.path && left.app == right.app);
        Ok(roots)
    }
}

fn push(
    roots: &mut Vec<CandidateRoot>,
    app: &AppId,
    path: PathBuf,
    kind: RootKind,
    evidence: &str,
) {
    roots.push(CandidateRoot {
        app: app.clone(),
        path,
        kind,
        confidence: DetectionConfidence::Weak,
        evidence: vec![evidence.to_owned()],
    });
}

fn find_aider_repos(ctx: &PathContext) -> Vec<CandidateRoot> {
    let scan_roots = aider_scan_roots(ctx);
    let mut roots = scan_roots
        .iter()
        .flat_map(|root| scan_aider_root(root, ctx, aider_scan_depth(ctx)))
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.path.cmp(&right.path));
    roots.dedup_by(|left, right| left.path == right.path);
    roots
}

fn aider_scan_roots(ctx: &PathContext) -> Vec<PathBuf> {
    if ctx.portable_root.is_some() {
        return vec![ctx.home.clone()];
    }

    let mut roots = env::current_dir().ok().into_iter().collect::<Vec<_>>();
    roots.extend(
        [
            "work",
            "workspace",
            "Documents/workspace",
            "Code",
            "Projects",
            "src",
            "dev",
        ]
        .into_iter()
        .map(|child| ctx.home.join(child)),
    );
    roots.retain(|path| path.exists());
    roots.sort();
    roots.dedup();
    roots
}

fn aider_scan_depth(ctx: &PathContext) -> usize {
    if ctx.portable_root.is_some() { 8 } else { 5 }
}

fn scan_aider_root(root: &Path, ctx: &PathContext, max_depth: usize) -> Vec<CandidateRoot> {
    WalkDir::new(root)
        .follow_links(false)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| !is_skipped_dir(entry.path(), &ctx.home))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy();
            matches!(
                name.as_ref(),
                ".aider.chat.history.md" | ".aider.input.history" | ".aider.llm.history"
            )
            .then(|| entry.path().parent().map(Path::to_path_buf))
            .flatten()
        })
        .map(|path| CandidateRoot {
            app: AppId::Aider,
            path,
            kind: RootKind::RepoLocal,
            confidence: DetectionConfidence::Strong,
            evidence: vec!["aider history file".to_owned()],
        })
        .collect::<Vec<_>>()
}

fn is_skipped_dir(path: &Path, home: &Path) -> bool {
    if path == home || !path.is_dir() {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".git"
            | "node_modules"
            | "target"
            | ".vibe-hauler"
            | "Library"
            | "AppData"
            | ".cache"
            | ".cargo"
            | ".rustup"
    ) || (name.starts_with('.') && name != ".config" && name != ".local")
}

#[cfg(test)]
mod tests {
    use super::{PathContext, RootResolver, StandardRootResolver};
    use vibe_hauler_core::{AppId, OsKind};

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn resolves_cross_platform_roots() {
        for (fixture_name, os) in [
            ("macos-home", OsKind::MacOS),
            ("linux-home", OsKind::Linux),
            ("windows-home", OsKind::Windows),
        ] {
            let ctx = PathContext::for_portable_root(fixture(fixture_name), os);
            let resolver = StandardRootResolver::default();
            for app in [AppId::Claude, AppId::Codex, AppId::Gemini, AppId::Aider] {
                let roots = resolver
                    .candidate_roots(&app, &ctx)
                    .expect("roots should resolve");
                assert!(
                    roots.iter().any(|root| root.path.exists()),
                    "{fixture_name:?} missing {app:?}"
                );
            }
        }
    }
}
