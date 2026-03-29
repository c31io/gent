use crate::plugins::errors::PluginError;
use crate::scripts::engine::{ConsoleLine, RUNE_ENGINE};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptContent {
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub run_id: String,
    pub console_lines: Vec<ConsoleLine>,
}

/// Returns the user scripts directory, creating it if needed
fn user_scripts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let scripts_dir = app_data.join("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;
    }
    Ok(scripts_dir)
}

/// Returns the bundled scripts directory from resources.
/// Falls back to `public/scripts` (Trunk dev server static files) during dev builds
/// when the resource dir path doesn't contain scripts.
fn bundled_scripts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let resource_scripts = resource_dir.join("scripts");
    if resource_scripts.exists() {
        return Ok(resource_scripts);
    }
    // Dev fallback: Trunk serves static files from `public/` at the project root
    // (two dirs up from src-tauri/ where cargo runs)
    let dev_path = PathBuf::from(".").join("..").join("public").join("scripts");
    if dev_path.exists() {
        return Ok(dev_path);
    }
    Ok(resource_scripts)
}

/// List all available scripts (bundled + user)
#[tauri::command]
pub fn list_scripts(app: AppHandle) -> Result<Vec<ScriptInfo>, String> {
    let mut scripts = Vec::new();

    // Bundled scripts
    let bundled = bundled_scripts_dir(&app).unwrap_or_else(|_| PathBuf::new());
    if bundled.exists() {
        if let Ok(entries) = fs::read_dir(&bundled) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rn") {
                    if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                        let source = fs::read_to_string(&path).unwrap_or_default();
                        let description = source.lines()
                            .next()
                            .map(|l| l.trim_start_matches("//").trim().to_string())
                            .unwrap_or_default();
                        scripts.push(ScriptInfo {
                            id: id.into(),
                            name: id.into(),
                            origin: "bundled".into(),
                            description,
                        });
                    }
                }
            }
        }
    }

    // User scripts
    let user_dir = user_scripts_dir(&app)?;
    if user_dir.exists() {
        if let Ok(entries) = fs::read_dir(&user_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rn") {
                    if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                        let source = fs::read_to_string(&path).unwrap_or_default();
                        let description = source.lines()
                            .next()
                            .map(|l| l.trim_start_matches("//").trim().to_string())
                            .unwrap_or_default();
                        scripts.push(ScriptInfo {
                            id: id.into(),
                            name: id.into(),
                            origin: "user".into(),
                            description,
                        });
                    }
                }
            }
        }
    }

    Ok(scripts)
}

/// Read a script by ID (bundled or user)
#[tauri::command]
pub fn read_script(app: AppHandle, id: String) -> Result<ScriptContent, String> {
    // Validate ID: alphanumeric ASCII only
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("invalid script ID: must be alphanumeric".into());
    }

    // Check user scripts first (user takes precedence over bundled)
    let user_path = user_scripts_dir(&app)?.join(format!("{}.rn", id));
    if user_path.exists() {
        let source = fs::read_to_string(&user_path).map_err(|e| e.to_string())?;
        return Ok(ScriptContent { source });
    }

    // Check bundled scripts
    let bundled_path = bundled_scripts_dir(&app)?.join(format!("{}.rn", id));
    if bundled_path.exists() {
        let source = fs::read_to_string(&bundled_path).map_err(|e| e.to_string())?;
        return Ok(ScriptContent { source });
    }

    Err(format!("script not found: {}", id))
}

/// Save a user script
#[tauri::command]
pub fn save_script(app: AppHandle, id: String, content: String) -> Result<(), String> {
    // Validate ID
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("invalid script ID: must be alphanumeric".into());
    }

    // Reject if matching bundled script
    let bundled_path = bundled_scripts_dir(&app)?.join(format!("{}.rn", id));
    if bundled_path.exists() {
        return Err("cannot overwrite bundled script".into());
    }

    let user_path = user_scripts_dir(&app)?.join(format!("{}.rn", id));
    fs::write(&user_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Run a script and stream console output
#[tauri::command]
pub async fn run_script(
    app: AppHandle,
    id: String,
    input: serde_json::Value,
) -> Result<RunResult, String> {
    let run_id = Uuid::new_v4().to_string();

    // Read script source in a blocking task to avoid blocking the async executor
    let source = tokio::task::spawn_blocking({
        let app = app.clone();
        let id = id.clone();
        move || read_script(app, id).map(|c| c.source)
    })
    .await
    .map_err(|e| format!("task join error: {}", e))?
    .map_err(|e| e.to_string())?;

    // Get RUNE_ENGINE
    let engine = RUNE_ENGINE.get().ok_or_else(|| String::from("Rune engine not initialized"))?;

    // Run synchronously in a blocking task to avoid blocking the async runtime
    let run_id_clone = run_id.clone();
    let input_clone = input.clone();
    let lines: Vec<ConsoleLine> = tokio::task::spawn_blocking(move || {
        engine.run(&source, input_clone, &run_id_clone)
    })
    .await
    .map_err(|e| format!("task join error: {}", e))?
    .map_err(|e: PluginError| e.to_string())?;

    // Emit each line as a Tauri event for real-time streaming
    for line in &lines {
        let _ = app.emit("script-console-line", line.clone());
    }

    Ok(RunResult {
        run_id,
        console_lines: lines,
    })
}