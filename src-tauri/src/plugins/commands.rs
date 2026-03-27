use crate::plugins::{Manifest, PluginError, PluginLoader, PluginRegistry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

pub struct PluginState {
    pub registry: PluginRegistry,
    pub loader: PluginLoader,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadPluginRequest {
    pub wasm_bytes: Vec<u8>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub manifest: Manifest,
}

/// Load a plugin from WASM bytes
#[tauri::command]
pub fn load_plugin(
    state: State<'_, Arc<PluginState>>,
    request: LoadPluginRequest,
) -> Result<PluginInfo, String> {
    // Parse requested capabilities
    let requested_caps: Vec<_> = request
        .capabilities
        .iter()
        .filter_map(|s| crate::plugins::Capability::from_str(s))
        .collect();

    // Validate: all requested capabilities must be Gent-supported
    let supported_caps = &[crate::plugins::Capability::Context,
                           crate::plugins::Capability::Tools,
                           crate::plugins::Capability::Memory,
                           crate::plugins::Capability::Nodes,
                           crate::plugins::Capability::Execution];
    for cap in &requested_caps {
        if !supported_caps.contains(cap) {
            return Err(format!("unsupported capability: {:?}", cap));
        }
    }

    let plugin = state
        .loader
        .load_plugin(&request.wasm_bytes, &requested_caps)
        .map_err(|e| e.to_string())?;

    // Validate: plugin manifest capabilities must be subset of granted capabilities
    let manifest = plugin.manifest();
    for cap in &manifest.capabilities {
        if !requested_caps.contains(cap) {
            return Err(format!(
                "plugin {} requires {:?} capability but it was not granted",
                manifest.name, cap
            ));
        }
    }

    let manifest = plugin.manifest().clone();
    let id = state.registry.register(plugin.into()).map_err(|e| e.to_string())?;

    Ok(PluginInfo { id, manifest })
}

/// List all loaded plugins
#[tauri::command]
pub fn list_plugins(state: State<'_, Arc<PluginState>>) -> Vec<PluginInfo> {
    state
        .registry
        .list_ids()
        .iter()
        .filter_map(|id| {
            state.registry.get(id).map(|p| PluginInfo {
                id: id.clone(),
                manifest: p.manifest().clone(),
            })
        })
        .collect()
}

/// Unload a plugin
#[tauri::command]
pub fn unload_plugin(state: State<'_, Arc<PluginState>>, plugin_id: String) -> Result<(), String> {
    state
        .registry
        .unregister(&plugin_id)
        .map_err(|e| e.to_string())
}

/// Call a plugin's process function
#[tauri::command]
pub fn call_plugin(
    state: State<'_, Arc<PluginState>>,
    plugin_id: String,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let plugin = state
        .registry
        .get(&plugin_id)
        .ok_or_else(|| format!("plugin not found: {}", plugin_id))?;

    let input = crate::plugins::Input(input);
    let output = plugin.process(input).map_err(|e| e.to_string())?;
    Ok(output.0)
}