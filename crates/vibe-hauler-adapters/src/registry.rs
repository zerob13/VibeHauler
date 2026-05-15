use crate::AppAdapter;

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
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}
