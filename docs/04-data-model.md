# Data Model

This document defines shared structures used across modules. Field names are Rust-oriented but should map cleanly to JSON output.

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

## AppInstance

Represents one detected installation or data root.

```rust
pub struct AppInstance {
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
    pub modified_at: Option<OffsetDateTime>,
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
    pub started_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
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

## CleanPlan

Generated before mutation.

```rust
pub struct CleanPlan {
    pub id: PlanId,
    pub created_at: OffsetDateTime,
    pub version: String,
    pub platform: OsKind,
    pub mode: PlanMode,
    pub actions: Vec<PlannedAction>,
    pub totals: PlanTotals,
    pub warnings: Vec<PlanWarning>,
}
```

Planned actions are immutable once written to disk; execution results belong in the manifest.

## CleanManifest

Generated after mutation.

```rust
pub struct CleanManifest {
    pub id: ManifestId,
    pub plan_id: PlanId,
    pub created_at: OffsetDateTime,
    pub platform: OsKind,
    pub actions: Vec<ExecutedAction>,
    pub restore_state: RestoreState,
}
```

Each `ExecutedAction` records original path, destination, backup path if any, size, hash, risk, and whether restore is possible.

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

