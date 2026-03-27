pub mod errors;
pub mod capabilities;
pub mod plugin;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};