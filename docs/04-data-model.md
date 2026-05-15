# Data Model

This document defines shared structures used across modules. Field names are Rust-oriented but should map cleanly to JSON output.

Timestamps are represented as RFC3339 strings in the current contracts. A future implementation may wrap them in a stronger time type at module boundaries.

## App Identity

```rust
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
```

Display names are UI labels and may contain spaces. `AppId` variants are code identifiers and must stay stable.

| AppId | Display Name |
|---|---|
| `Claude` | Claude Code |
| `Codex` | Codex |
| `OpenCode` | OpenCode |
| `Cursor` | Cursor |
| `CherryStudio` | Cherry Studio |
| `DeepChat` | DeepChat |
| `Gemini` | Gemini CLI |
| `Goose` | Goose |
| `Aider` | Legacy parser-only ID; not registered by default |
| `Alma` | Alma |
| `FactoryDroid` | Factory Droid |
| `CopilotCli` | Copilot CLI |

## AppInstance

Represents one detected installation or data root.

```rust
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
```

## InventoryItem

Represents a file, directory, database, or logical cleanup candidate.

```rust
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
```

## AgentSession

Represents one reviewable session, transcript, conversation, or agent run.

```rust
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
```

Supporting session types:

```rust
pub struct TokenUsage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub total: Option<u64>,
}

pub enum SessionSource {
    File(PathBuf),
    Directory(PathBuf),
    Database(PathBuf),
    Backup(PathBuf),
    Unknown,
}
```

## CleanPlan

Generated before mutation.

```rust
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
```

Supporting plan types:

```rust
pub enum PlanMode {
    SafeCleanup,
    SessionCleanup,
}

pub struct PlanTotals {
    pub actions: u64,
    pub files: u64,
    pub bytes: u64,
    pub green: u64,
    pub yellow: u64,
}

pub struct PlanWarning {
    pub code: String,
    pub message: String,
}

pub struct PlannedAction {
    pub app: AppId,
    pub kind: ActionKind,
    pub path: PathBuf,
    pub risk: RiskLevel,
    pub size_bytes: u64,
    pub backup_required: bool,
    pub reason: String,
}

pub enum ActionKind {
    Trash,
    BackupAndTrash,
    Quarantine,
    ReportOnly,
}
```

Planned actions are immutable once written to disk; execution results belong in the manifest.

## CleanManifest

Generated after mutation.

```rust
pub struct CleanManifest {
    pub id: ManifestId,
    pub plan_id: PlanId,
    pub created_at: String,
    pub platform: OsKind,
    pub actions: Vec<ExecutedAction>,
    pub restore_state: RestoreState,
}
```

Each `ExecutedAction` records original path, destination, backup path if any, size, hash, risk, and whether restore is possible.

Supporting manifest types:

```rust
pub enum RestoreState {
    NotNeeded,
    Restorable,
    PartiallyRestorable,
    NotRestorable,
}

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

pub enum ExecutionStatus {
    MovedToTrash,
    Quarantined,
    Skipped,
    Failed,
}
```

## Stable IDs

IDs should be deterministic inside one scan when possible:

| ID | Suggested Input |
|---|---|
| `InstanceId` | app + normalized root path hash |
| `ItemId` | app + path + kind + mtime/size hash |
| `SessionId` | app + source path + app session id or content hash |
| `PlanId` | timestamp + plan hash |
| `ManifestId` | timestamp + plan id |

IDs shown in tables can be shortened, but JSON must include full IDs.
