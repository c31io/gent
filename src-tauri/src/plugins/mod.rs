pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;
pub mod rust_loader;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;
pub use rust_loader::RustWasmLoader;

/// WASM loader trait - implemented by RuneLoader and RustWasmLoader
pub trait WasmLoader: Send + Sync {
    fn can_load(&self, wasm: &[u8]) -> bool;
    fn load(&self, wasm: &[u8], capabilities: &[Capability]) -> Result<Box<dyn Plugin>, PluginError>;
}