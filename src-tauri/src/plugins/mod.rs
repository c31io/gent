pub mod capabilities;
pub mod commands;
pub mod console;
pub mod errors;
pub mod host;
pub mod loader;
pub mod plugin;
pub mod registry;
pub mod rune_loader;
pub mod wasm_loader;

pub use capabilities::Capability;
pub use errors::PluginError;
pub use loader::PluginLoader;
pub use plugin::{Input, Manifest, Plugin};
pub use registry::PluginRegistry;
pub use wasm_loader::WasmPluginLoader;

/// Plugin source loader trait - implemented by WasmPluginLoader and RunePluginLoader
pub trait PluginSource: Send + Sync {
    fn can_load(&self, extension: &str) -> bool;
    fn load(
        &self,
        source: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError>;
}
