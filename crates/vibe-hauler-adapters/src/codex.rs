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
pub struct CodexAdapter;

impl AppAdapter for CodexAdapter {
    fn id(&self) -> AppId {
        AppId::Codex
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &["config.toml", "auth.json", "history.jsonl"],
        )
    }

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>> {
        let mut items = Vec::new();
        for child in ["config.toml", "auth.json"] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    if child == "auth.json" {
                        ItemKind::Credential
                    } else {
                        ItemKind::Config
                    },
                    RiskLevel::Red,
                    Recommendation::Protect,
                    "Codex auth and config files are protected",
                    None,
                    vec![child.to_owned()],
                )?);
            }
        }
        for child in ["cache", "logs", "tmp"] {
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
        for path in find_files(&instance.root, &["history.jsonl"], &[]) {
            items.push(forced_inventory_item(
                instance,
                path,
                ItemKind::Session,
                RiskLevel::Yellow,
                Recommendation::Review,
                "Codex history JSONL requires review and backup",
                Some("jsonl-session"),
                vec!["history JSONL".to_owned()],
            )?);
        }
        Ok(items)
    }

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        let parser = JsonlSessionParser;
        find_files(&instance.root, &["history.jsonl"], &[])
            .into_iter()
            .map(|path| {
                parser.parse(ParserInput {
                    path,
                    app_hint: Some("codex".to_owned()),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map(|nested| nested.into_iter().flatten().collect())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "logs-cache-tmp".to_owned(),
            description: "Clean Codex logs, caches, and temporary files".to_owned(),
        }]
    }
}
