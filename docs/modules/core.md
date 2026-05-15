# Core Module

Cargo package: `vibe-hauler-core`

## Responsibility

The core module defines stable contracts:

- app IDs and platform IDs;
- inventory items;
- sessions;
- risk levels;
- recommendations;
- cleanup plans;
- manifests;
- summary totals;
- plan validation.

It should have no app-specific filesystem traversal and no terminal formatting.

## Main Types

```rust
pub enum RiskLevel { Green, Yellow, Red, Black }

pub enum Recommendation {
    Clean,
    Review,
    Protect,
    ReportOnly,
}

pub struct InventoryItem { /* see data model */ }
pub struct AgentSession { /* see data model */ }
pub struct CleanPlan { /* planned actions */ }
pub struct CleanManifest { /* executed actions */ }
```

## Risk Engine

Inputs:

- adapter-provided item kind;
- path evidence;
- parser evidence;
- size and age;
- process/lock state;
- global protection rules.

Outputs:

- normalized `RiskLevel`;
- `Recommendation`;
- human-readable reason;
- `backup_required`.

Default rules:

| Pattern | Risk |
|---|---|
| cache, tmp, GPUCache, old logs | Green |
| session transcript, chat history, checkpoints | Yellow |
| auth, token, config, rules, skills, commands, memory | Red |
| locked DB, permission denied, unknown binary | Black |

## Plan Builder

The plan builder takes inventory and user policy:

```rust
pub struct PlanPolicy {
    pub include_green: bool,
    pub include_yellow: bool,
    pub older_than: Option<Duration>,
    pub apps: Option<Vec<AppId>>,
    pub require_backup_for_yellow: bool,
}
```

It returns a deterministic `CleanPlan` with sorted actions and totals.

## Validation

Plan validation should catch:

- actions that target Red or Black items;
- missing backup for Yellow;
- duplicate paths;
- parent/child double-delete conflicts;
- path outside discovered roots unless user-provided;
- stale item metadata before execution.

## Testing

- Unit tests for every risk rule.
- Snapshot tests for plan JSON.
- Property-style tests for duplicate and nested path handling.
- No OS-specific tests in core.

