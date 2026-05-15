use vibe_hauler_core::{AppId, AppInstance};
use vibe_hauler_discovery::PathContext;

use crate::{
    AppAdapter, aider::AiderAdapter, claude::ClaudeAdapter, codex::CodexAdapter,
    gemini::GeminiAdapter,
};

#[derive(Default)]
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn AppAdapter>>,
}

impl AdapterRegistry {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    #[must_use]
    pub fn v01() -> Self {
        Self {
            adapters: vec![
                Box::<ClaudeAdapter>::default(),
                Box::<CodexAdapter>::default(),
                Box::<GeminiAdapter>::default(),
                Box::<AiderAdapter>::default(),
            ],
        }
    }

    #[must_use]
    pub fn filtered(apps: &[AppId]) -> Self {
        let mut registry = Self::v01();
        registry
            .adapters
            .retain(|adapter| apps.iter().any(|app| app == &adapter.id()));
        registry
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    #[must_use]
    pub fn adapters(&self) -> &[Box<dyn AppAdapter>] {
        &self.adapters
    }

    pub fn detect_all(&self, ctx: &PathContext) -> anyhow::Result<Vec<AppInstance>> {
        let mut instances = Vec::new();
        for adapter in &self.adapters {
            instances.extend(adapter.detect(ctx)?);
        }
        instances.sort_by(|left, right| {
            left.app
                .cmp(&right.app)
                .then_with(|| left.root.cmp(&right.root))
        });
        Ok(instances)
    }

    pub fn adapter_for(&self, app: &AppId) -> Option<&dyn AppAdapter> {
        self.adapters
            .iter()
            .find(|adapter| adapter.id() == *app)
            .map(Box::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::AdapterRegistry;
    use vibe_hauler_core::AppId;

    #[test]
    fn v01_registry_has_four_adapters() {
        let registry = AdapterRegistry::v01();
        let ids = registry
            .adapters()
            .iter()
            .map(|adapter| adapter.id())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![AppId::Claude, AppId::Codex, AppId::Gemini, AppId::Aider]
        );
    }
}
