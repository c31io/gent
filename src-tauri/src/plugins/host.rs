use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::{Input, Output};

/// Host API provided to Gent's execution engine
pub trait PluginHost {
    /// Call a loaded plugin with JSON input, returns JSON output
    fn call_plugin(
        &mut self,
        plugin_id: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError>;

    /// Get the capabilities a plugin was granted
    fn get_capabilities(&self, plugin_id: &str) -> &[Capability];

    /// Revoke and unload a plugin
    fn revoke_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError>;
}
