#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

use vibe_hauler_core::{AgentSession, AppId, AppInstance, InventoryItem};
use vibe_hauler_discovery::PathContext;

pub mod aider;
pub mod claude;
pub mod codex;
mod common;
pub mod gemini;
pub mod registry;

pub trait AppAdapter: Send + Sync {
    fn id(&self) -> AppId;

    fn display_name(&self) -> &'static str;

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>>;

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>>;

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>>;

    fn clean_rules(&self) -> Vec<CleanRule>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanRule {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use vibe_hauler_core::{AppId, RiskLevel};
    use vibe_hauler_discovery::PathContext;

    use crate::{
        AppAdapter, aider::AiderAdapter, claude::ClaudeAdapter, codex::CodexAdapter,
        gemini::GeminiAdapter,
    };

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn each_v01_adapter_detects_inventory_and_sessions() {
        for fixture_name in ["macos-home", "linux-home", "windows-home"] {
            let root = fixture(fixture_name);
            let os = PathContext::infer_portable_os(&root);
            let ctx = PathContext::for_portable_root(root, os);
            let adapters: Vec<Box<dyn AppAdapter>> = vec![
                Box::new(ClaudeAdapter),
                Box::new(CodexAdapter),
                Box::new(GeminiAdapter),
                Box::new(AiderAdapter),
            ];

            for adapter in adapters {
                let instances = adapter.detect(&ctx).expect("detect");
                assert!(!instances.is_empty(), "{fixture_name} {:?}", adapter.id());
                let inventory = adapter.inventory(&instances[0]).expect("inventory");
                let sessions = adapter.sessions(&instances[0]).expect("sessions");
                assert!(!inventory.is_empty(), "{fixture_name} {:?}", adapter.id());
                assert!(!sessions.is_empty(), "{fixture_name} {:?}", adapter.id());
                assert!(
                    inventory.iter().any(|item| item.risk == RiskLevel::Yellow),
                    "{fixture_name} {:?} should expose Yellow history",
                    adapter.id()
                );
                if adapter.id() != AppId::Aider {
                    assert!(
                        inventory.iter().any(|item| item.risk == RiskLevel::Green),
                        "{fixture_name} {:?} should expose Green cleanup",
                        adapter.id()
                    );
                }
            }
        }
    }

    #[test]
    fn credentials_and_config_are_protected() {
        let ctx =
            PathContext::for_portable_root(fixture("macos-home"), vibe_hauler_core::OsKind::MacOS);
        let adapter = CodexAdapter;
        let instance = adapter.detect(&ctx).expect("detect").remove(0);
        let inventory = adapter.inventory(&instance).expect("inventory");

        assert!(
            inventory
                .iter()
                .any(|item| { item.path.ends_with("auth.json") && item.risk == RiskLevel::Red })
        );
        assert!(
            inventory
                .iter()
                .any(|item| { item.path.ends_with("cache") && item.risk == RiskLevel::Green })
        );
    }
}
