pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;
pub mod wasm_loader;
pub mod loader;
pub mod console;
pub mod commands;
pub mod rune_loader;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Plugin};
pub use registry::PluginRegistry;
pub use wasm_loader::WasmPluginLoader;
pub use loader::PluginLoader;

/// Plugin source loader trait - implemented by WasmPluginLoader and RunePluginLoader
pub trait PluginSource: Send + Sync {
    fn can_load(&self, extension: &str) -> bool;
    fn load(&self, source: &[u8], capabilities: &[Capability]) -> Result<Box<dyn Plugin>, PluginError>;
}
