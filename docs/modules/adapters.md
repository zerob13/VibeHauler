# Adapters Module

Cargo package: `vibe-hauler-adapters`

## Responsibility

Adapters translate app-specific layouts into VibeHauler data:

- detect app-specific evidence inside candidate roots;
- produce inventory items;
- produce sessions;
- attach parser names and evidence;
- provide app-specific clean rules.

Adapters do not execute cleanup. They can recommend, but the core risk engine normalizes final risk.

## Trait

```rust
pub trait AppAdapter: Send + Sync {
    fn id(&self) -> AppId;
    fn display_name(&self) -> &'static str;
    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>>;
    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>>;
    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>>;
    fn clean_rules(&self) -> Vec<CleanRule>;
}

pub struct CleanRule {
    pub name: String,
    pub description: String,
}
```

The v0.1 API returns vectors for simplicity. If a fixture shows large-session performance problems, introduce a paged `SessionQuery`/`SessionPage` API before implementing the affected adapter.

## Registry

The registry should allow:

- all adapters by default;
- filtering by `--apps`;
- disabling adapters in config;
- reporting unsupported adapters by platform.

```rust
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn AppAdapter>>,
}
```

## Adapter Tiers

| Tier | Meaning | v0.1 Requirement |
|---|---|---|
| Full | detection, inventory, sessions, Green cleanup | Claude, Codex, Gemini, Aider |
| Protective | detection, inventory, partial sessions/report, Green-only cleanup | not in v0.1; starts with Cursor/Goose in v0.2 |
| Sniffer | detection and generic file/session hints | not in v0.1; starts with OpenCode in v0.2 |

## Per-adapter Files

Recommended layout:

```txt
crates/vibe-hauler-adapters/src/
├── lib.rs
├── registry.rs
├── claude.rs
├── codex.rs
├── gemini.rs
├── aider.rs
├── opencode.rs          # v0.2
├── cursor.rs            # v0.2
├── cherry/
│   ├── mod.rs
│   ├── backup_json.rs
│   ├── local_storage.rs
│   └── indexeddb.rs
├── deepchat.rs
├── goose.rs             # v0.2
├── alma.rs
└── factory_droid.rs
```

## Testing

Each adapter needs:

- detection tests for macOS/Linux/Windows fixtures;
- inventory snapshot;
- session snapshot if supported;
- Red/Green protection test for credentials/config/cache;
- locked DB behavior where applicable.
