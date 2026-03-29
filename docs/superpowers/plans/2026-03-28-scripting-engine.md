# Scripting Engine + Script Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load the Rune scripting engine at backend startup, and add a Scripts tab to the right panel with a script editor, console output, and script selector.

**Architecture:** Backend-hosted Rune (B1) — the `RuneEngine` singleton owns a `rune::RuntimeContext` and compiles Rune source at runtime. Four Tauri commands (`list_scripts`, `read_script`, `save_script`, `run_script`) bridge frontend and backend. Console output streams via Tauri events with per-run `run_id` correlation. Frontend uses Leptos components with a tabbed right panel (Trace/Scripts).

**Tech Stack:** Tauri 2.x, Rust, rune 0.13, Leptos 0.7, wasm-bindgen, web-sys, serde_json, uuid

---

## File Structure

### Backend (src-tauri/)

```
src-tauri/src/
├── lib.rs                         # Add scripts module, initialize RUNE_ENGINE
├── plugins/
│   └── loader.rs                  # Existing: load_rune_engine() for Rune WASM plugins
└── scripts/                       # NEW
    ├── mod.rs                     # Module exports + register Tauri commands
    ├── engine.rs                  # RuneEngine singleton, run()
    └── commands.rs                # Tauri commands: list/read/save/run

resources/                         # NEW (tauri.conf.json bundles this)
└── scripts/
    └── hello.rn                   # Bundled example script
```

### Frontend (src/)

```
src/
├── components/
│   ├── mod.rs                     # Add script_editor, script_console exports
│   ├── app_layout.rs              # Replace ExecutionTrace with RightPanel
│   ├── right_panel.rs              # NEW: Tabbed container (Trace/Scripts)
│   ├── script_editor.rs            # NEW: Script selector + CodeMirror editor + Run/Save
│   ├── script_console.rs           # NEW: Console output pane with run_id routing
│   └── execution_trace.rs          # Existing
```

---

## Task 1: Backend — RuneEngine Singleton

**Files:**
- Create: `src-tauri/src/scripts/engine.rs`
- Create: `src-tauri/src/scripts/mod.rs`
- Create: `src-tauri/src/scripts/commands.rs`
- Modify: `src-tauri/src/lib.rs:1-55`

- [ ] **Step 1: Create `src-tauri/src/scripts/engine.rs`**

```rust
// src-tauri/src/scripts/engine.rs

use crate::plugins::errors::PluginError;
use once_cell::sync::OnceLock;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Source, Sources, Vm};
use serde::Serialize;
use std::sync::Arc;

/// Unique run ID for correlating console output
#[derive(Debug, Clone, Serialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
    pub run_id: String,
}

/// Global Rune engine singleton
pub static RUNE_ENGINE: OnceLock<Arc<RuneEngine>> = OnceLock::new();

pub struct RuneEngine {
    runtime: Arc<rune::RuntimeContext>,
}

impl RuneEngine {
    /// Create a new RuneEngine with default modules
    pub fn new() -> Result<Self, PluginError> {
        let context = Context::with_default_modules()
            .map_err(|e| PluginError::Runtime(format!("failed to create context: {}", e)))?;
        let runtime = Arc::try_new(context.runtime()
            .map_err(|e| PluginError::Runtime(format!("failed to create runtime: {}", e)))?)
            .map_err(|_| PluginError::Runtime("failed to share runtime reference".into()))?;
        Ok(Self { runtime })
    }

    /// Execute a Rune script and return console lines (compile/runtime errors)
    /// Phase 1: result value is discarded, only console output matters
    pub fn run(
        &self,
        source: &str,
        input: serde_json::Value,
        run_id: &str,
    ) -> Result<Vec<ConsoleLine>, PluginError> {
        use rune::sync::Arc as RuneArc;

        let mut sources = Sources::new();
        sources.insert(Source::memory(source)?);

        let mut diagnostics = Diagnostics::new();
        let context = Context::with_default_modules()
            .map_err(|e| PluginError::Runtime(format!("context error: {}", e)))?;

        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build_vm();

        // Collect console lines
        let mut lines = Vec::new();

        // Emit compile errors
        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            if let Err(e) = diagnostics.emit(&mut writer, &sources) {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("diagnostic emit failed: {}", e),
                    run_id: run_id.into(),
                });
            }
            // Also collect as structured error lines
            for i in 0..diagnostics.len() {
                if let Some(diag) = diagnostics.get(i) {
                    lines.push(ConsoleLine {
                        level: "error".into(),
                        message: diag.to_string(),
                        run_id: run_id.into(),
                    });
                }
            }
        }

        let mut vm = result.map_err(|e| {
            lines.push(ConsoleLine {
                level: "error".into(),
                message: format!("vm build error: {}", e),
                run_id: run_id.into(),
            });
            PluginError::Runtime(format!("vm build failed: {}", e))
        })?;

        // Call process function with the provided input
        let input = rune::to_value(&input)?;
        match vm.call(["process"], (input,)) {
            Ok(output) => {
                // Phase 1: ignore output value, only console lines matter
                let _: Result<serde_json::Value, _> = rune::from_value(output);
            }
            Err(e) => {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("runtime error: {}", e),
                    run_id: run_id.into(),
                });
            }
        }

        Ok(lines)
    }
}
```

- [ ] **Step 2: Create `src-tauri/src/scripts/commands.rs`**

```rust
// src-tauri/src/scripts/commands.rs

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

/// Returns the bundled scripts directory from resources
fn bundled_scripts_dir() -> Result<PathBuf, String> {
    // In Tauri 2, resources are in the resource directory
    let resource_dir = std::env::var("RESOURCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    Ok(resource_dir.join("scripts"))
}

/// List all available scripts (bundled + user)
#[tauri::command]
pub fn list_scripts(app: AppHandle) -> Result<Vec<ScriptInfo>, String> {
    let mut scripts = Vec::new();

    // Bundled scripts
    let bundled = bundled_scripts_dir().unwrap_or_else(|_| PathBuf::new());
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
    let bundled_path = bundled_scripts_dir()?.join(format!("{}.rn", id));
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
    let bundled_path = bundled_scripts_dir()?.join(format!("{}.rn", id));
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

    // Read script source
    let source = read_script(app.clone(), id.clone())?.source;

    // Get RUNE_ENGINE
    let engine = RUNE_ENGINE.get().ok_or_else(|| "Rune engine not initialized".into())?;

    // Run synchronously in a blocking task to avoid blocking the async runtime
    let run_id_clone = run_id.clone();
    let input_clone = input.clone();
    let lines = tokio::task::spawn_blocking(move || {
        engine.run(&source, input_clone, &run_id_clone)
    })
    .await
    .map_err(|e| format!("task join error: {}", e))?
    .map_err(|e| e.to_string())?;

    // Emit each line as a Tauri event for real-time streaming
    for line in &lines {
        let _ = app.emit("script-console-line", line.clone());
    }

    Ok(RunResult {
        run_id,
        console_lines: lines,
    })
}
```

- [ ] **Step 3: Create `src-tauri/src/scripts/mod.rs`**

```rust
// src-tauri/src/scripts/mod.rs

pub mod commands;
pub mod engine;

pub use commands::{list_scripts, read_script, save_script, run_script};
```

- [ ] **Step 4: Modify `src-tauri/src/lib.rs` to add `scripts` module and initialize RUNE_ENGINE**

**Note on `load_rune_engine()` integration:** The existing `load_rune_engine()` in `loader.rs` is a `WasmLoader`-compatible function that returns `Box<dyn Plugin>` — it's designed for loading Rune-compiled WASM plugin binaries (Phase 2+). For Phase 1 source script execution, we initialize the `RUNE_ENGINE` singleton directly in `lib.rs`. Future work (Phase 2 `code_execute` node type) may connect these by having the plugin-wrapped Rune call into the singleton.

```rust
// src-tauri/src/lib.rs (full file)

use std::process::Command;
use std::sync::Arc;
use crate::plugins::{PluginLoader, PluginRegistry};
use crate::plugins::commands::{
    self, call_plugin, list_plugins, load_plugin, unload_plugin, PluginState
};
use crate::scripts::engine::RUNE_ENGINE;

mod plugins;
mod scripts;

#[tauri::command]
fn execute_code(code: String) -> Result<String, String> {
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
    // Initialize Rune engine singleton
    let rune_engine = crate::scripts::engine::RuneEngine::new()
        .expect("failed to initialize Rune engine");
    crate::scripts::engine::RUNE_ENGINE
        .set(Arc::new(rune_engine))
        .expect("Rune engine already initialized");

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
            // Script commands
            scripts::list_scripts,
            scripts::read_script,
            scripts::save_script,
            scripts::run_script,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Add `tokio` to `src-tauri/Cargo.toml` for `spawn_blocking`**

Add to Cargo.toml `[dependencies]`:
```toml
tokio = { version = "1", features = ["rt-multi-thread", "sync"] }
```

- [ ] **Step 6: Run `cargo check` to verify backend compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors (may have warnings about unused fields in RuneWasmPlugin - that's fine)

---

## Task 2: Backend — Bundled Example Script

**Files:**
- Create: `src-tauri/resources/scripts/hello.rn`
- Modify: `src-tauri/tauri.conf.json` to include resources

- [ ] **Step 1: Create `src-tauri/resources/scripts/hello.rn`**

```rune
// hello.rn — basic hello world example
// Run with: println("Hello from Rune!")

fn process(input) {
    println("Hello from Rune script!")
    println("Input: " + input.to_string())
    #{ result: "ok" }
}
```

- [ ] **Step 2: Update `src-tauri/tauri.conf.json` to bundle resources**

Read the existing tauri.conf.json and add a `bundle.resources` section pointing to `resources/`.

```json
{
  "bundle": {
    "resources": {
      "resources/**": "."
    }
  }
}
```

**Note:** Tauri 2.x resource configuration varies — verify with `cargo tauri build --debug` or check the [Tauri 2 resource docs](https://v2.tauri.app/distribute/resources/). If resources are embedded differently, adjust the `bundled_scripts_dir()` function in `commands.rs`.

---

## Task 3: Frontend — Right Panel Tab System

**Files:**
- Create: `src/components/right_panel.rs`
- Modify: `src/components/app_layout.rs:1-460`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/right_panel.rs`**

```rust
// src/components/right_panel.rs

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tab {
    Trace,
    Scripts,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Trace
    }
}

#[component]
fn TabBar(active_tab: ReadSignal<Tab>, set_active_tab: WriteSignal<Tab>) -> impl IntoView {
    view! {
        <div class="tab-bar">
            <button
                class=move || format!("tab{}", if active_tab.get() == Tab::Trace { " tab-active" } else { "" })
                on:click={move |_| set_active_tab.set(Tab::Trace)}
            >
                "Trace"
            </button>
            <button
                class=move || format!("tab{}", if active_tab.get() == Tab::Scripts { " tab-active" } else { "" })
                on:click={move |_| set_active_tab.set(Tab::Scripts)}
            >
                "Scripts"
            </button>
        </div>
    }
}

#[component]
pub fn RightPanel(
    execution: Signal<crate::components::execution_engine::ExecutionState>,
) -> impl IntoView {
    let (active_tab, set_active_tab) = signal(Tab::default());

    view! {
        <div class="right-panel">
            <TabBar active_tab={active_tab.into()} set_active_tab={set_active_tab} />
            {move || match active_tab.get() {
                Tab::Trace => view! {
                    <crate::components::execution_trace::ExecutionTrace execution={execution} />
                }.into_any(),
                Tab::Scripts => view! {
                    <crate::components::script_editor::ScriptEditor />
                }.into_any(),
            }}
        </div>
    }
}
```

- [ ] **Step 2: Add `right_panel` to `src/components/mod.rs`**

Add after `pub mod plugin_manager;`:
```rust
pub mod right_panel;
```

- [ ] **Step 3: Modify `src/components/app_layout.rs` to use RightPanel**

In `app_layout.rs`, replace the hardcoded `<ExecutionTrace ... />` in the right panel div with `<RightPanel execution={execution_state.into()} />`.

Change line ~408 from:
```rust
<ExecutionTrace execution={execution_state.into()} />
```
To:
```rust
<RightPanel execution={execution_state.into()} />
```

Also add the import at the top:
```rust
use crate::components::right_panel::RightPanel;
```

---

## Task 4: Frontend — Script Editor Component

**Files:**
- Create: `src/components/script_editor.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/script_editor.rs`**

```rust
// src/components/script_editor.rs

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

/// Script info from list_scripts
#[derive(Debug, Clone)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub description: String,
}

/// Console line from run_script
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
    pub run_id: String,
}

/// Run result from run_script
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RunResult {
    pub run_id: String,
    pub console_lines: Vec<ConsoleLine>,
}

/// Call Tauri backend to list scripts
async fn list_scripts() -> Result<Vec<ScriptInfo>, String> {
    let window = web_sys::window().ok_or_else(|| "no window".into())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".into());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"list_scripts".into());
    args.push(&js_sys::Object::new());

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    let js_value = JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    let scripts: Vec<ScriptInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(scripts)
}

/// Call Tauri backend to read a script
async fn read_script(id: String) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "no window".into())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".into());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"read_script".into());
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    args.push(&opts);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    let js_value = JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    #[derive(Deserialize)]
    struct Content { source: String }
    let content: Content = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(content.source)
}

/// Call Tauri backend to save a script
async fn save_script(id: String, content: String) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "no window".into())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".into());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"save_script".into());
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    js_sys::Reflect::set(&opts, &"content".into(), &content.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    args.push(&opts);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    Ok(())
}

/// Call Tauri backend to run a script
async fn run_script(id: String, input: serde_json::Value) -> Result<RunResult, String> {
    let window = web_sys::window().ok_or_else(|| "no window".into())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".into());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"run_script".into());
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    js_sys::Reflect::set(&opts, &"input".into(), &serde_wasm_bindgen::to_value(&input)
        .map_err(|e| format!("serde error: {:?}", e))?)
    args.push(&opts);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    let js_value = JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    let result: RunResult = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(result)
}

#[component]
pub fn ScriptEditor() -> impl IntoView {
    let (scripts, set_scripts) = signal(Vec::<ScriptInfo>::new());
    let (selected_script, set_selected_script) = signal(Option::<ScriptInfo>::None);
    let (editor_content, set_editor_content) = signal(String::new());
    let (console_lines, set_console_lines) = signal(Vec::<ConsoleLine>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (running, set_running) = signal(false);

    // Load script list on mount
    spawn_local(async move {
        set_loading.set(true);
        match list_scripts().await {
            Ok(list) => {
                set_scripts.set(list);
                // Auto-select first script if available
                if let Some(first) = list.first().cloned() {
                    set_selected_script.set(Some(first.clone()));
                    match read_script(first.id.clone()).await {
                        Ok(source) => set_editor_content.set(source),
                        Err(e) => set_error.set(Some(e)),
                    }
                }
            }
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    // Listen for console events
    spawn_local(async move {
        let window = web_sys::window().unwrap();
        let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into()).ok()?;
        let event = js_sys::Reflect::get(&tauri, &"event".into()).ok()?;
        let listen_fn = js_sys::Reflect::get(&event, &"listen".into()).ok()?;

        let console_line: js_sys::Object = serde_wasm_bindgen::to_value(&ConsoleLine {
            level: String::new(),
            message: String::new(),
            run_id: String::new(),
        }).ok()?.dyn_into().ok()?;

        let args = js_sys::Array::new();
        args.push(&"script-console-line".into());
        let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |line: JsValue| {
            if let Ok(cl) = serde_wasm_bindgen::from_value::<ConsoleLine>(line) {
                set_console_lines.update(|lines| {
                    let mut new_lines = lines.clone();
                    new_lines.push(cl);
                    new_lines
                });
            }
        }) as Box<dyn FnMut(JsValue)>);
        let _ = js_sys::Reflect::apply(&listen_fn.into(), &event, &args);

        // Keep callback alive - not strictly needed for oneshot but good practice
        let _cb = cb;
    });

    let handle_run = move |_| {
        let Some(script) = selected_script.get() else { return };
        set_running.set(true);
        set_console_lines.set(vec![]);

        spawn_local(async move {
            match run_script(script.id.clone(), serde_json::json!({})).await {
                Ok(result) => {
                    // Lines already streamed via events, final result just confirms completion
                    set_running.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_running.set(false);
                }
            }
        });
    };

    let handle_save = move |_| {
        let Some(script) = selected_script.get() else { return };
        let content = editor_content.get();

        spawn_local(async move {
            match save_script(script.id.clone(), content).await {
                Ok(()) => { /* saved */ }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_select = move |id: String| {
        let scripts_snapshot = scripts.get();
        if let Some(script) = scripts_snapshot.iter().find(|s| s.id == id).cloned() {
            set_selected_script.set(Some(script.clone()));
            set_console_lines.set(vec![]);
            spawn_local(async move {
                match read_script(script.id.clone()).await {
                    Ok(source) => set_editor_content.set(source),
                    Err(e) => set_error.set(Some(e)),
                }
            });
        }
    };

    view! {
        <div class="script-editor">
            <div class="panel-header">"Scripts"</div>

            {move || {
                if loading.get() {
                    view! { <p>"Loading scripts..."</p> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <p class="error">{err}</p> }.into_any()
                } else {
                    view! {
                        <>
                            {/* Script selector dropdown */}
                            <div class="script-selector">
                                <select
                                    class="script-select"
                                    on:change={move |ev| {
                                        let id = event_target_value(&ev);
                                        handle_select(id);
                                    }}
                                >
                                    <option value="" disabled=true selected={selected_script.get().is_none()}>
                                        "Select a script..."
                                    </option>
                                    {scripts.get().iter().map(|s| {
                                        let is_selected = selected_script.get().as_ref().map(|sel| sel.id == s.id).unwrap_or(false);
                                        view! {
                                            <option
                                                value={s.id.clone()}
                                                selected={is_selected}
                                            >
                                                {format!("[{}] {}", s.origin, s.name)}
                                            </option>
                                        }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </div>

                            {/* Code editor textarea (CodeMirror integration point) */}
                            <div class="script-editor-area">
                                <textarea
                                    class="code-textarea"
                                    rows="12"
                                    value={editor_content.get()}
                                    on:input={move |ev| set_editor_content.set(event_target_value(&ev))}
                                    placeholder="// Select or create a script..."
                                />
                            </div>

                            {/* Action buttons */}
                            <div class="script-actions">
                                <button
                                    class="btn-run"
                                    on:click={handle_run}
                                    disabled={running.get() || selected_script.get().is_none()}
                                >
                                    {if running.get() { "Running..." } else { "Run" }}
                                </button>
                                <button
                                    class="btn-save"
                                    on:click={handle_save}
                                    disabled={selected_script.get().is_none()}
                                >
                                    "Save"
                                </button>
                            </div>

                            {/* Console output */}
                            <div class="script-console">
                                <div class="console-header">"Console"</div>
                                <div class="console-lines">
                                    {console_lines.get().iter().map(|line| {
                                        let cls = if line.level == "error" { "console-error" } else { "console-info" };
                                        view! {
                                            <div class={cls}>
                                                <span class="console-level">[{&line.level}]</span>
                                                <span class="console-message">{&line.message}</span>
                                            </div>
                                        }.into_any()
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        </>
                    }.into_any()
                }
            }}
        </div>
    }
}
```

---

## Task 5: Styling for Script Tab

**Files:**
- Modify: Find the CSS file (check `src/*.css` or inline styles)

- [ ] **Step 1: Find and add CSS for script components**

Locate the CSS file — likely `src/styles.css` or similar. Add:

```css
/* Right panel tab bar (shared with left panel) */
.tab-bar {
    display: flex;
    background: var(--bg-secondary, #1e1e1e);
    border-bottom: 1px solid var(--border-color, #333);
}

.tab-bar .tab {
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    font-size: 13px;
}

.tab-bar .tab:hover {
    color: var(--text-primary, #fff);
}

.tab-bar .tab-active {
    color: var(--accent-color, #4a9eff);
    border-bottom: 2px solid var(--accent-color, #4a9eff);
}

/* Script editor */
.script-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
}

.script-selector {
    padding: 8px;
    border-bottom: 1px solid var(--border-color, #333);
}

.script-select {
    width: 100%;
    padding: 6px 8px;
    background: var(--bg-input, #2a2a2a);
    color: var(--text-primary, #fff);
    border: 1px solid var(--border-color, #444);
    border-radius: 4px;
    font-size: 13px;
}

.script-editor-area {
    flex: 1;
    min-height: 200px;
    padding: 8px;
    overflow: auto;
}

.code-textarea {
    width: 100%;
    min-height: 200px;
    background: var(--bg-input, #1a1a1a);
    color: var(--text-primary, #d4d4d4);
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    padding: 8px;
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    resize: vertical;
    box-sizing: border-box;
    line-height: 1.5;
}

.script-actions {
    display: flex;
    gap: 8px;
    padding: 8px;
    border-top: 1px solid var(--border-color, #333);
}

.script-actions button {
    padding: 6px 16px;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
    border: none;
}

.btn-run {
    background: var(--accent-color, #4a9eff);
    color: #fff;
}

.btn-run:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-save {
    background: var(--bg-secondary, #333);
    color: var(--text-primary, #fff);
    border: 1px solid var(--border-color, #444);
}

.btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

/* Script console */
.script-console {
    height: 150px;
    border-top: 1px solid var(--border-color, #333);
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.console-header {
    padding: 4px 8px;
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-muted, #666);
    background: var(--bg-secondary, #1e1e1e);
}

.console-lines {
    flex: 1;
    overflow-y: auto;
    padding: 4px 8px;
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 12px;
}

.console-info {
    color: var(--text-secondary, #aaa);
    white-space: pre-wrap;
}

.console-error {
    color: #f48771;
    white-space: pre-wrap;
}

.console-level {
    color: var(--text-muted, #666);
    margin-right: 4px;
}
```

---

## Task 6: Tauri Capability for Script Events

**Files:**
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Add event listener capability**

In Tauri 2.x, `core:event:listen` grants permission to listen for events. Add it to the permissions list:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:listen",
    "opener:default"
  ]
}
```

Backend emission via `app.emit("script-console-line", ...)` is covered by `core:default`. No additional event registration needed.

---

## Task 7: Build Verification

- [ ] **Step 1: Run `cargo check` on backend**

Run: `cd src-tauri && cargo check`
Expected: Compiles cleanly

- [ ] **Step 2: Run `trunk serve` to verify frontend compiles**

Run: `trunk serve` (in project root)
Expected: WASM compiles and serves on http://localhost:1420

- [ ] **Step 3: Run `cargo tauri build` to verify full build**

Run: `cargo tauri build` in `src-tauri/`
Expected: Produces `src-tauri/target/release/gent.exe`

---

## Dependencies Summary

| Dependency | Purpose | Status |
|---|---|---|
| `rune = "0.13"` | Scripting engine | Already in Cargo.toml |
| `rune-modules = "0.13"` | Rune std modules | Already in Cargo.toml |
| `wasmtime = "22"` | WASM runtime | Already in Cargo.toml |
| `once_cell = "1"` | `OnceLock` for singleton | Already in Cargo.toml |
| `uuid = { version = "1", features = ["v4"] }` | run_id generation | Already in Cargo.toml |
| `tokio = { version = "1", features = ["rt-multi-thread", "sync"] }` | `spawn_blocking` | **Needs addition** |
| `serde_wasm_bindgen` | Frontend JSON ↔ JsValue | **Check Cargo.toml / package.json** |
| `gloo-timers` | `TimeoutFuture` | Already in package.json |

**Note on CodeMirror 6:** This plan uses a `<textarea>` as the code editor for simplicity. For production, replace with a proper CodeMirror 6 integration (via the `codemirror` npm package). The textarea is a functional placeholder that lets the feature work end-to-end without the additional CodeMirror complexity.

---

## Verification Checklist

After implementation:
- [ ] `cargo check` passes in `src-tauri/`
- [ ] `trunk serve` runs without frontend errors
- [ ] Scripts tab appears in right panel (Trace/Scripts tabs visible)
- [ ] Script dropdown lists `hello.rn` (bundled) when running in Tauri
- [ ] Clicking Run on `hello.rn` shows "Hello from Rune script!" in console
- [ ] Clicking Save persists a modified user script to `{app_data}/scripts/`
- [ ] Console output lines appear in real-time as script runs
