use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::Plugin;
use crate::plugins::{RuneLoader, RustWasmLoader, WasmLoader};
use std::sync::Arc;

/// Registry of WASM loaders for general plugin loading (Rust WASM only)
pub struct PluginLoader {
    loaders: Vec<Arc<dyn WasmLoader>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        let loaders = vec![
            Arc::new(RustWasmLoader::new().unwrap()) as Arc<dyn WasmLoader>,
        ];
        Self { loaders }
    }

    /// Check if any loader can handle this WASM binary
    pub fn can_load(&self, wasm: &[u8]) -> bool {
        self.loaders.iter().any(|l| l.can_load(wasm))
    }

    /// Load a plugin using the appropriate loader
    pub fn load_plugin(
        &self,
        wasm: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError> {
        for loader in &self.loaders {
            if loader.can_load(wasm) {
                return loader.load(wasm, capabilities);
            }
        }
        Err(PluginError::Loader(
            "no loader found for this WASM binary".into(),
        ))
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Load the one and only Rune script engine.
///
/// The Rune engine is a singleton embedded in Gent, not a general plugin.
/// Separate from PluginLoader to avoid loader detection ambiguity.
pub fn load_rune_engine(
    wasm: &[u8],
    capabilities: &[Capability],
) -> Result<Box<dyn Plugin>, PluginError> {
    RuneLoader::new()
        .and_then(|loader| loader.load(wasm, capabilities))
}