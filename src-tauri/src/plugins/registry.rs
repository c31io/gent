use crate::plugins::errors::PluginError;
use crate::plugins::plugin::Plugin;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// #[derive(Debug)]  // Removed - dyn Plugin doesn't implement Debug
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new plugin, returns unique plugin_id
    pub fn register(&self, plugin: Arc<dyn Plugin>) -> Result<String, PluginError> {
        let mut plugins = self.plugins.write().map_err(|_| {
            PluginError::Runtime("registry poisoned".into())
        })?;

        // Generate unique ID
        let id = Uuid::new_v4().to_string();
        plugins.insert(id.clone(), plugin);

        Ok(id)
    }

    /// Unregister a plugin
    pub fn unregister(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().map_err(|_| {
            PluginError::Runtime("registry poisoned".into())
        })?;

        plugins.remove(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;

        Ok(())
    }

    /// Get a plugin by ID
    pub fn get(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().ok()?;
        plugins.get(plugin_id).cloned()
    }

    /// List all plugin IDs
    pub fn list_ids(&self) -> Vec<String> {
        let plugins = self.plugins.read().unwrap();
        plugins.keys().cloned().collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}