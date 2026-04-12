use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::Plugin;
use crate::plugins::rune_loader::RunePluginLoader;
use crate::plugins::{PluginSource, WasmPluginLoader};
use std::sync::Arc;

/// Registry of plugin source loaders for general plugin loading
pub struct PluginLoader {
    loaders: Vec<Arc<dyn PluginSource>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        let loaders = vec![
            Arc::new(WasmPluginLoader::new().unwrap()) as Arc<dyn PluginSource>,
            Arc::new(RunePluginLoader::new()) as Arc<dyn PluginSource>,
        ];
        Self { loaders }
    }

    /// Load a plugin using the appropriate loader
    pub fn load_plugin(
        &self,
        source: &[u8],
        capabilities: &[Capability],
        extension: &str,
    ) -> Result<Box<dyn Plugin>, PluginError> {
        for loader in &self.loaders {
            if loader.can_load(extension) {
                return loader.load(source, capabilities);
            }
        }
        Err(PluginError::Loader(
            "no loader found for this plugin source".into(),
        ))
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
