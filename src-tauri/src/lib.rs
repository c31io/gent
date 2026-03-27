use std::process::Command;
use std::sync::Arc;
use crate::plugins::{PluginLoader, PluginRegistry};
use crate::plugins::commands::{
    self, call_plugin, list_plugins, load_plugin, unload_plugin, PluginState
};

mod plugins;

#[tauri::command]
fn execute_code(code: String) -> Result<String, String> {
    // Run via sh on mac/linux, cmd on windows
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &code])
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(["-c", &code])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let plugin_state = Arc::new(PluginState {
        registry: PluginRegistry::new(),
        loader: PluginLoader::new(),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(plugin_state)
        .invoke_handler(tauri::generate_handler![
            execute_code,
            load_plugin,
            list_plugins,
            unload_plugin,
            call_plugin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}