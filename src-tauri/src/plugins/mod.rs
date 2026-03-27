pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;