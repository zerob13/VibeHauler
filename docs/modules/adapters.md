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
```

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
| Protective | detection, inventory, partial sessions/report, Green-only cleanup | Cursor, Cherry, DeepChat, Alma, Goose |
| Sniffer | detection and generic file/session hints | OpenCode, Factory Droid, Copilot CLI |

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
├── opencode.rs
├── cursor.rs
├── cherry/
│   ├── mod.rs
│   ├── backup_json.rs
│   ├── local_storage.rs
│   └── indexeddb.rs
├── deepchat.rs
├── goose.rs
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

