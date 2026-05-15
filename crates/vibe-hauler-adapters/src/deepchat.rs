use vibe_hauler_core::{
    AgentSession, AppId, AppInstance, InventoryItem, ItemKind, Recommendation, RiskLevel,
};
use vibe_hauler_discovery::PathContext;
use vibe_hauler_parsers::{ParserInput, SessionParser, sqlite::DeepChatSqliteParser};

use crate::{
    AppAdapter, CleanRule,
    common::{detect_from_roots, existing_child, existing_descendant, forced_inventory_item},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct DeepChatAdapter;

impl AppAdapter for DeepChatAdapter {
    fn id(&self) -> AppId {
        AppId::DeepChat
    }

    fn display_name(&self) -> &'static str {
        "DeepChat"
    }

    fn detect(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        detect_from_roots(
            &self.id(),
            self.display_name(),
            ctx,
            &["app-settings.json", "app_db/agent.db", "app_db/chat.db"],
        )
    }

    fn inventory(&self, instance: &AppInstance) -> anyhow::Result<Vec<InventoryItem>> {
        let mut items = Vec::new();

        for child in [
            "app-settings.json",
            "config.json",
            "settings.json",
            "Preferences",
        ] {
            if let Some(path) = existing_child(&instance.root, child) {
                items.push(forced_inventory_item(
                    instance,
                    path,
                    ItemKind::Config,
                    RiskLevel::Red,
                    Recommendation::Protect,
                    "DeepChat settings and provider configuration are protected",
                    None,
                    vec![format!("DeepChat {child}")],
                )?);
            }
        }

        for (parts, evidence) in [
            (
                &["app_db", "agent.db"][..],
                "agent.db stores current DeepChat sessions, messages, agents, and settings",
            ),
            (
                &["app_db", "chat.db"][..],
                "chat.db is a legacy DeepChat conversation store",
            ),
            (
                &["app_db", "knowledge.duckdb"][..],
                "DuckDB knowledge bases are protected",
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
            "tmp",
            "temp",
        ] {
            if let Some(path) = existing_child(&instance.root, child) {
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
                    "DeepChat rebuildable cache, log, and temporary data",
                    None,
                    vec![format!("{child} directory")],
                )?);
            }
        }

        Ok(items)
    }

    fn sessions(&self, instance: &AppInstance) -> anyhow::Result<Vec<AgentSession>> {
        if let Some(path) = existing_descendant(&instance.root, &["app_db", "agent.db"]) {
            let sessions = parse_db(path)?;
            if !sessions.is_empty() {
                return Ok(sessions);
            }
        }
        if let Some(path) = existing_descendant(&instance.root, &["app_db", "chat.db"]) {
            return parse_db(path);
        }
        Ok(Vec::new())
    }

    fn clean_rules(&self) -> Vec<CleanRule> {
        vec![CleanRule {
            name: "deepchat-cache-logs".to_owned(),
            description: "Clean DeepChat cache, log, and temporary directories".to_owned(),
        }]
    }
}

fn parse_db(path: std::path::PathBuf) -> anyhow::Result<Vec<AgentSession>> {
    DeepChatSqliteParser.parse(ParserInput {
        path,
        app_hint: Some("deepchat".to_owned()),
    })
}
