use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{InstanceId, ItemId, SessionId};
use crate::risk::{Recommendation, RiskLevel};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AppId {
    Claude,
    Codex,
    OpenCode,
    Cursor,
    CherryStudio,
    DeepChat,
    Gemini,
    Goose,
    Aider,
    Alma,
    FactoryDroid,
    CopilotCli,
    Unknown(String),
}

impl AppId {
    #[must_use]
    pub fn display_name(&self) -> &str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::OpenCode => "OpenCode",
            Self::Cursor => "Cursor",
            Self::CherryStudio => "Cherry Studio",
            Self::DeepChat => "DeepChat",
            Self::Gemini => "Gemini CLI",
            Self::Goose => "Goose",
            Self::Aider => "Aider",
            Self::Alma => "Alma",
            Self::FactoryDroid => "Factory Droid",
            Self::CopilotCli => "Copilot CLI",
            Self::Unknown(value) => value.as_str(),
        }
    }

    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Cursor => "cursor",
            Self::CherryStudio => "cherry",
            Self::DeepChat => "deepchat",
            Self::Gemini => "gemini",
            Self::Goose => "goose",
            Self::Aider => "aider",
            Self::Alma => "alma",
            Self::FactoryDroid => "factory-droid",
            Self::CopilotCli => "copilot-cli",
            Self::Unknown(value) => value.as_str(),
        }
    }

    #[must_use]
    pub fn from_key(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Self::Claude,
            "codex" => Self::Codex,
            "opencode" | "open-code" => Self::OpenCode,
            "cursor" => Self::Cursor,
            "cherry" | "cherry-studio" | "cherrystudio" => Self::CherryStudio,
            "deepchat" | "deep-chat" => Self::DeepChat,
            "gemini" | "gemini-cli" => Self::Gemini,
            "goose" => Self::Goose,
            "aider" => Self::Aider,
            "alma" => Self::Alma,
            "factory-droid" | "factorydroid" => Self::FactoryDroid,
            "copilot" | "copilot-cli" => Self::CopilotCli,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum OsKind {
    MacOS,
    Linux,
    Windows,
    Wsl,
}

impl OsKind {
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RootKind {
    Config,
    Data,
    Cache,
    State,
    ElectronUserData,
    RepoLocal,
    UserProvided,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize)]
pub enum DetectionConfidence {
    Weak,
    Strong,
    Exact,
    UserProvided,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppInstance {
    pub id: InstanceId,
    pub app: AppId,
    pub display_name: String,
    pub root: PathBuf,
    pub root_kind: RootKind,
    pub platform: OsKind,
    pub confidence: DetectionConfidence,
    pub evidence: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ItemKind {
    File,
    Directory,
    Database,
    Log,
    Cache,
    Session,
    Config,
    Credential,
    Memory,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InventoryItem {
    pub id: ItemId,
    pub app: AppId,
    pub instance_id: InstanceId,
    pub path: PathBuf,
    pub kind: ItemKind,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub risk: RiskLevel,
    pub recommendation: Recommendation,
    pub reason: String,
    pub backup_required: bool,
    pub parser: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentSession {
    pub id: SessionId,
    pub app: AppId,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub turns: Option<u32>,
    pub tokens: Option<TokenUsage>,
    pub files: Vec<PathBuf>,
    pub size_bytes: u64,
    pub preview: Option<String>,
    pub preview_hash: Option<String>,
    pub risk: RiskLevel,
    pub source: SessionSource,
    pub parser: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TokenUsage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub total: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SessionSource {
    File(PathBuf),
    Directory(PathBuf),
    Database(PathBuf),
    Backup(PathBuf),
    Unknown,
}
