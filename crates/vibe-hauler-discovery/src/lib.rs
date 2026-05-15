#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

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
        match app {
            AppId::Claude => Self::claude_roots(app, ctx),
            AppId::Codex => Self::codex_roots(app, ctx),
            AppId::Gemini => Self::gemini_roots(app, ctx),
            AppId::Cursor => Self::cursor_roots(app, ctx),
            AppId::CherryStudio => Self::cherry_roots(app, ctx),
            AppId::DeepChat => Self::deepchat_roots(app, ctx),
            _ => Vec::new(),
        }
    }

    fn claude_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        push(
            &mut roots,
            app,
            ctx.home.join(".claude"),
            RootKind::Config,
            "home .claude",
        );
        push_config_variants(
            &mut roots,
            app,
            ctx,
            ["claude", "Claude"],
            RootKind::Config,
            "config dir",
        );
        roots
    }

    fn codex_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        if let Some(env_home) = env::var_os("CODEX_HOME").map(PathBuf::from) {
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
        push_config_variants(
            &mut roots,
            app,
            ctx,
            ["codex", "Codex"],
            RootKind::Config,
            "config dir",
        );
        roots
    }

    fn gemini_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        push(
            &mut roots,
            app,
            ctx.home.join(".gemini"),
            RootKind::Config,
            "home .gemini",
        );
        push_config_variants(
            &mut roots,
            app,
            ctx,
            ["gemini", "Gemini"],
            RootKind::Config,
            "config dir",
        );
        roots
    }

    fn cursor_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        push(
            &mut roots,
            app,
            ctx.home.join(".cursor").join("User"),
            RootKind::ElectronUserData,
            "home .cursor user data",
        );
        if let Some(config_dir) = &ctx.config_dir {
            push(
                &mut roots,
                app,
                config_dir.join("Cursor").join("User"),
                RootKind::ElectronUserData,
                "Cursor Electron user data",
            );
        }
        roots
    }

    fn cherry_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        push_config_variants(
            &mut roots,
            app,
            ctx,
            ["CherryStudio", "CherryStudioDev", "cherry-studio"],
            RootKind::ElectronUserData,
            "Cherry Studio Electron user data",
        );
        roots
    }

    fn deepchat_roots(app: &AppId, ctx: &PathContext) -> Vec<CandidateRoot> {
        let mut roots = Vec::new();
        push_config_variants(
            &mut roots,
            app,
            ctx,
            ["DeepChat", "deepchat"],
            RootKind::ElectronUserData,
            "DeepChat Electron user data",
        );
        if let Some(data_dir) = &ctx.data_dir {
            for name in ["DeepChat", "deepchat"] {
                push(
                    &mut roots,
                    app,
                    data_dir.join(name),
                    RootKind::Data,
                    "DeepChat data dir",
                );
            }
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

fn push_config_variants<const N: usize>(
    roots: &mut Vec<CandidateRoot>,
    app: &AppId,
    ctx: &PathContext,
    names: [&str; N],
    kind: RootKind,
    evidence: &str,
) {
    if let Some(config_dir) = &ctx.config_dir {
        for name in names {
            push(roots, app, config_dir.join(name), kind, evidence);
        }
    }
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
            for app in [
                AppId::Claude,
                AppId::Codex,
                AppId::Gemini,
                AppId::Cursor,
                AppId::CherryStudio,
                AppId::DeepChat,
            ] {
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
