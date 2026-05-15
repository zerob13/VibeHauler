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
pub struct GeminiAdapter;

impl AppAdapter for GeminiAdapter {
    fn id(&self) -> AppId {
        AppId::Gemini
    }

    fn display_name(&self) -> &'static str {
        "Gemini CLI"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &["settings.json", "tmp"],
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
                "Gemini settings are protected",
                None,
                vec!["settings.json".to_owned()],
            )?);
        }
        for child in ["cache", "logs"] {
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
        for path in find_files(&instance.root.join("tmp"), &["session.jsonl"], &["jsonl"]) {
            items.push(forced_inventory_item(
                instance,
                path,
                ItemKind::Session,
                RiskLevel::Yellow,
                Recommendation::Review,
                "Gemini chat JSONL requires review and backup",
                Some("jsonl-session"),
                vec!["tmp chats JSONL".to_owned()],
            )?);
        }
        Ok(items)
    }

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        let parser = JsonlSessionParser;
        find_files(&instance.root.join("tmp"), &["session.jsonl"], &["jsonl"])
            .into_iter()
            .map(|path| {
                parser.parse(ParserInput {
                    path,
                    app_hint: Some("gemini".to_owned()),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map(|nested| nested.into_iter().flatten().collect())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "logs-cache".to_owned(),
            description: "Clean Gemini CLI logs and caches".to_owned(),
        }]
    }
}
