use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{InstanceId, SessionId};
use crate::risk::{Recommendation, RiskLevel};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OsKind {
    MacOS,
    Linux,
    Windows,
    Wsl,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootKind {
    Config,
    Data,
    Cache,
    State,
    ElectronUserData,
    RepoLocal,
    UserProvided,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    pub id: String,
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
