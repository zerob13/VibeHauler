use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, InventoryItem, ItemKind, Recommendation, RiskLevel,
};
use vibe_hauler_discovery::PathContext;
use vibe_hauler_parsers::{JsonlSessionParser, ParserInput, SessionParser};

use crate::{
    AppAdapter, CleanRule,
    common::{
        detect_from_roots, existing_child, find_files, forced_inventory_item, inventory_item,
    },
};

#[derive(Clone, Copy, Debug, Default)]
pub struct ClaudeAdapter;

impl AppAdapter for ClaudeAdapter {
    fn id(&self) -> AppId {
        AppId::Claude
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &["settings.json", "projects"],
        )
    }

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>> {
        let mut items = Vec::new();
        if let Some(path) = existing_child(&instance.root, "settings.json") {
            items.push(forced_inventory_item(
                instance,
                path,
                ItemKind::Config,
                RiskLevel::Red,
                Recommendation::Protect,
                "Claude settings and credentials are protected",
                None,
                vec!["settings.json".to_owned()],
            )?);
        }
        for child in ["logs", "cache", "tmp"] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(inventory_item(
                    instance,
                    path,
                    if child == "logs" {
                        ItemKind::Log
                    } else {
                        ItemKind::Cache
                    },
                    None,
                    vec![format!("{child} directory")],
                )?);
            }
        }
        for path in find_files(&instance.root.join("projects"), &[], &["jsonl"]) {
            items.push(forced_inventory_item(
                instance,
                path,
                ItemKind::Session,
                RiskLevel::Yellow,
                Recommendation::Review,
                "Claude JSONL transcript requires review and backup",
                Some("jsonl-session"),
                vec!["projects JSONL transcript".to_owned()],
            )?);
        }
        Ok(items)
    }

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        let parser = JsonlSessionParser;
        find_files(&instance.root.join("projects"), &[], &["jsonl"])
            .into_iter()
            .map(|path| {
                parser.parse(ParserInput {
                    path,
                    app_hint: Some("claude".to_owned()),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map(|nested| nested.into_iter().flatten().collect())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "logs-cache-tmp".to_owned(),
            description: "Clean Claude Code logs, caches, and temporary files".to_owned(),
        }]
    }
}
