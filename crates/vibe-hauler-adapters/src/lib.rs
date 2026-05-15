#![allow(clippy::missing_errors_doc)]

use vibe_hauler_core::{AgentSession, AppId, AppInstance, InventoryItem};
use vibe_hauler_discovery::PathContext;

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
