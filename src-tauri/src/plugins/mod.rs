pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;
pub mod rust_loader;
pub mod rune_loader;
pub mod loader;
pub mod commands;

#[cfg(test)] mod integration_test;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;
pub use rust_loader::RustWasmLoader;
pub use rune_loader::RuneLoader;
pub use loader::{load_rune_engine, PluginLoader};

/// WASM loader trait - implemented by RuneLoader and RustWasmLoader
pub trait WasmLoader: Send + Sync {
    fn can_load(&self, wasm: &[u8]) -> bool;
    fn load(&self, wasm: &[u8], capabilities: &[Capability]) -> Result<Box<dyn Plugin>, PluginError>;
}