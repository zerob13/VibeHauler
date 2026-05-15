use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, InventoryItem, ItemKind, Recommendation, RiskLevel,
};
use vibe_hauler_discovery::PathContext;

use crate::{
    AppAdapter, CleanRule,
    common::{detect_from_roots, existing_child, existing_descendant, forced_inventory_item},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CherryStudioAdapter;

impl AppAdapter for CherryStudioAdapter {
    fn id(&self) -> AppId {
        AppId::CherryStudio
    }

    fn display_name(&self) -> &'static str {
        "Cherry Studio"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &["config.json", "Local Storage", "IndexedDB", "Data"],
        )
    }

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>> {
        let mut items = Vec::new();

        for child in [
            "config.json",
            "settings.json",
            "app-settings.json",
            "Preferences",
        ] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    ItemKind::Config,
                    RiskLevel::Red,
                    Recommendation::Protect,
                    "Cherry Studio settings and provider configuration are protected",
                    None,
                    vec![format!("Cherry Studio {child}")],
                )?);
            }
        }

        for (parts, evidence) in [
            (
                &["Local Storage", "leveldb"][..],
                "Local Storage LevelDB contains provider and assistant state",
            ),
            (
                &["IndexedDB"][..],
                "IndexedDB contains chat topics and message blocks",
            ),
            (
                &["Data", "agents.db"][..],
                "agents.db is a protected SQLite agent store",
            ),
        ] {
            if let Some(path) = existing_descendant(&instance.root, parts) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    ItemKind::Database,
                    RiskLevel::Black,
                    Recommendation::ReportOnly,
                    evidence,
                    None,
                    vec![evidence.to_owned()],
                )?);
            }
        }

        for child in [
            "Cache",
            "CachedData",
            "Code Cache",
            "GPUCache",
            "DawnCache",
            "logs",
            "trace",
            "crashpad",
        ] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    if matches!(child, "logs" | "trace") {
                        ItemKind::Log
                    } else {
                        ItemKind::Cache
                    },
                    RiskLevel::Green,
                    Recommendation::Clean,
                    "Cherry Studio rebuildable trace, cache, and log data",
                    None,
                    vec![format!("{child} directory")],
                )?);
            }
        }

        Ok(items)
    }

    fn sessions(&self, _instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        Ok(Vec::new())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "cherry-cache-trace-logs".to_owned(),
            description: "Clean Cherry Studio trace, cache, and log directories".to_owned(),
        }]
    }
}
