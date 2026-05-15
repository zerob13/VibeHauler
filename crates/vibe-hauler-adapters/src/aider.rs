use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, InventoryItem, ItemKind, Recommendation, RiskLevel,
};
use vibe_hauler_discovery::PathContext;
use vibe_hauler_parsers::{MarkdownHistoryParser, ParserInput, SessionParser};

use crate::{
    AppAdapter, CleanRule,
    common::{detect_from_roots, existing_child, forced_inventory_item},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct AiderAdapter;

impl AppAdapter for AiderAdapter {
    fn id(&self) -> AppId {
        AppId::Aider
    }

    fn display_name(&self) -> &'static str {
        "Aider"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &[".aider.chat.history.md", ".aider.input.history"],
        )
    }

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>> {
        let mut items = Vec::new();
        if let Some(path) = existing_child(&instance.root, ".aider.conf.yml") {
            items.push(forced_inventory_item(
                instance,
                path,
                ItemKind::Config,
                RiskLevel::Red,
                Recommendation::Protect,
                "Aider project configuration is protected",
                None,
                vec![".aider.conf.yml".to_owned()],
            )?);
        }
        for child in [
            ".aider.chat.history.md",
            ".aider.input.history",
            ".aider.llm.history",
        ] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    ItemKind::Session,
                    RiskLevel::Yellow,
                    Recommendation::Review,
                    "Aider history requires review and backup",
                    Some("aider-markdown-history"),
                    vec![child.to_owned()],
                )?);
            }
        }
        Ok(items)
    }

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        let parser = MarkdownHistoryParser;
        [
            ".aider.chat.history.md",
            ".aider.input.history",
            ".aider.llm.history",
        ]
        .into_iter()
        .filter_map(|child| existing_child(&instance.root, child))
        .map(|path| {
            parser.parse(ParserInput {
                path,
                app_hint: Some("aider".to_owned()),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()
        .map(|nested| nested.into_iter().flatten().collect())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "history-review".to_owned(),
            description: "Review Aider chat and input history".to_owned(),
        }]
    }
}
