use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, InventoryItem, ItemKind, Recommendation, RiskLevel,
};
use vibe_hauler_discovery::PathContext;

use crate::{
    AppAdapter, CleanRule,
    common::{detect_from_roots, existing_child, existing_descendant, forced_inventory_item},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CursorAdapter;

impl AppAdapter for CursorAdapter {
    fn id(&self) -> AppId {
        AppId::Cursor
    }

    fn display_name(&self) -> &'static str {
        "Cursor"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &[
                "settings.json",
                "globalStorage/state.vscdb",
                "workspaceStorage",
            ],
        )
    }

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>> {
        let mut items = Vec::new();

        for child in ["settings.json", "keybindings.json", "snippets"] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    ItemKind::Config,
                    RiskLevel::Red,
                    Recommendation::Protect,
                    "Cursor user settings and editor state are protected",
                    None,
                    vec![format!("Cursor {child}")],
                )?);
            }
        }

        if let Some(path) = existing_descendant(&instance.root, &["globalStorage", "state.vscdb"]) {
            items.push(database_report_item(
                instance,
                path,
                "Cursor global storage database is report-only; it indexes chat and workspace state",
                "globalStorage state.vscdb",
            )?);
        }
        for path in crate::common::find_files(
            &instance.root.join("workspaceStorage"),
            &["state.vscdb"],
            &[],
        ) {
            items.push(database_report_item(
                instance,
                path,
                "Cursor workspace storage database is report-only; it may contain chat and project state",
                "workspaceStorage state.vscdb",
            )?);
        }

        if let Some(parent) = instance.root.parent() {
            for child in [
                "Cache",
                "CachedData",
                "Code Cache",
                "GPUCache",
                "DawnCache",
                "logs",
                "CachedExtensionVSIXs",
            ] {
                if let Some(path) = existing_child(parent, child) {
                    items.push(forced_inventory_item(
                        instance,
                        path,
                        if child == "logs" {
                            ItemKind::Log
                        } else {
                            ItemKind::Cache
                        },
                        RiskLevel::Green,
                        Recommendation::Clean,
                        "Cursor rebuildable cache and log data",
                        None,
                        vec![format!("{child} directory")],
                    )?);
                }
            }
        }

        Ok(items)
    }

    fn sessions(&self, _instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        Ok(Vec::new())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "cursor-cache-logs".to_owned(),
            description: "Clean Cursor cache and log directories; databases are report-only"
                .to_owned(),
        }]
    }
}

fn database_report_item(
    instance: &AppInstance,
    path: std::path::PathBuf,
    reason: &str,
    evidence: &str,
) -> anyhow::Result<InventoryItem> {
    forced_inventory_item(
        instance,
        path,
        ItemKind::Database,
        RiskLevel::Black,
        Recommendation::ReportOnly,
        reason,
        None,
        vec![evidence.to_owned()],
    )
}
