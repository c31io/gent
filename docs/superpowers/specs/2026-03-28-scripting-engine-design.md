# Scripting Engine + Script Tab Design

**Date:** 2026-03-28
**Status:** Ready for Review
**Author:** c31io

---

## Overview

Gent loads a Rune scripting engine at startup (B1 — backend-hosted via Tauri). Users can write, edit, and run Rune scripts from a new **Scripts tab** in the right panel. The script engine shares the same `process()` / capability-gated interface as Rust WASM plugins, providing a unified scripting and plugin experience.

---

## Architecture

### Backend (Tauri/Rust)

```
┌─────────────────────────────────────────────────────────────┐
│  gent-tauri src-tauri/src/                                  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  RuneEngine (singleton)                              │   │
│  │   - rune::Context with default modules               │   │
│  │   - One Vm instance per script invocation           │   │
│  │   - Scripts: rune source compiled on-the-fly        │   │
│  └─────────────────────────────────────────────────────┘   │
│           ▲ load_rune_engine() called at app startup        │
│           │ (existing rune_loader.rs)                       │
│           │                                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Tauri Commands (commands.rs)                        │   │
│  │   list_scripts()     → Vec<ScriptInfo>               │   │
│  │   read_script(id)   → ScriptContent                  │   │
│  │   save_script(id, content) → ()                      │   │
│  │   run_script(id, input) → RunResult                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Script sources:                                            │
│   - Bundled:  `resources/scripts/*.rn` (readonly, distributed) │
│   - User:     `{app_data_dir}/scripts/*.rn` (platform-specific) │
└─────────────────────────────────────────────────────────────┘
```

**Unified interface:** Both Rust WASM plugins and Rune scripts follow the same `process(input: JSON) -> JSON` entry point. Rust plugins are pre-compiled `.wasm` loaded via `PluginLoader` + `wasmtime`. Rune scripts are Rune source compiled at runtime via the `rune` crate. Both paths produce a `Result<Output, PluginError>`.

**Relationship to plugin system:** `RuneEngine` is a singleton that owns the `rune::Context`. It is not itself a `dyn Plugin` — rather it is the *runtime* that compiles and executes user script files. It reuses the same `Input`/`Output` types and `PluginError` errors as the plugin system. The `load_rune_engine()` function in `loader.rs` (from the 2026-03-27 design) is the integration point where the engine is initialized at startup.

**Capability model for scripts:** Phase 1 does not apply the capability model to scripts — scripts have full access to the `RuneEngine`'s context (no sandboxing). Phase 2 (`code_execute` node type) will revisit whether scripts should be capability-gated like plugins.

### Frontend (Leptos/WASM)

```
┌─────────────────────────────────────────────────────────────┐
│  Right Panel (right_panel.rs)                               │
│                                                             │
│  TabBar: [Trace] [Scripts]                                   │
│                                                             │
│  ScriptsTab:                                                │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  Script selector (dropdown) ←── list_scripts()       │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │                                                      │  │
│  │  CodeMirror 6 editor                                 │  │
│  │   - Rune syntax highlighting                         │  │
│  │   - read_script() on selection                       │  │
│  │                                                      │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │  [Run] [Save] buttons                               │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │  Console output pane                                 │  │
│  │   - Streamed line-by-line via Tauri events          │  │
│  │   - Errors in red, println! in normal text           │  │
│  └─────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Tauri Commands

### `list_scripts() -> Vec<ScriptInfo>`

Returns all available scripts (bundled + user).

```rust
struct ScriptInfo {
    id: String,          // "hello" or "my_script" (user scripts use filename without path)
    name: String,        // display name: "hello" or "my_script"
    origin: String,      // "bundled" | "user"
    description: String,  // first line of script or manifest comment
}
```

### `read_script(id: String) -> ScriptContent`

```rust
struct ScriptContent {
    source: String,  // full script source code
}
```

### `save_script(id: String, content: String) -> ()`

Writes user script to `{app_data_dir}/scripts/{id}.rn`. Script IDs must be alphanumeric ASCII (`a-z`, `A-Z`, `0-9`, `-`, `_`) — path traversal is rejected. Attempting to save a script whose ID matches a bundled script name returns an error.

### `run_script(id: String, input: serde_json::Value) -> RunResult`

```rust
struct ConsoleLine {
    level: String,   // "info" | "error"
    message: String, // raw text
    run_id: String,  // correlation ID for concurrent execution
}

struct RunResult {
    run_id: String,
    console_lines: Vec<ConsoleLine>,  // all output captured during execution
}
```

**Note:** Output streams via Tauri events (`script-console-line`) in real-time as lines are produced, before the command returns. The `run_id` is generated by the backend on each call and included in both streamed events and the final `RunResult`.

---

## Rune VM Lifecycle

### Initialization (on app start)

```rust
// At backend startup (main.rs or lib.rs)
pub static RUNE_ENGINE: OnceLock<Arc<RuneEngine>> = OnceLock::new();

pub struct RuneEngine {
    runtime: Arc<rune::RuntimeContext>,
}

impl RuneEngine {
    pub fn new() -> Result<Self, PluginError> {
        let context = rune::Context::with_default_modules()?;
        let runtime = Arc::try_new(context.runtime()?)
            .map_err(|_| PluginError::Runtime("failed to create runtime".into()))?;
        Ok(Self { runtime })
    }

    pub fn run(
        &self,
        source: &str,
        input: serde_json::Value,
    ) -> Result<Vec<ConsoleLine>, PluginError> {
        use rune::{Context, Diagnostics, Source, Sources, Vm};
        use rune::termcolor::{ColorChoice, StandardStream};
        use rune::sync::Arc;

        let mut sources = Sources::new();
        sources.insert(Source::memory(source)?);

        let mut diagnostics = Diagnostics::new();
        let context = Context::with_default_modules()?;

        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build_vm();

        // Emit compile errors as ConsoleLine
        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            diagnostics.emit(&mut writer, &sources)?;
            // Collect diagnostics as error lines
            // (Implementation: iterate diagnostics and emit each as ConsoleLine)
        }

        let mut vm = result?;

        // Call the `process` function with input as JSON argument
        let input = rune::to_value(&input)?;
        let output = vm.call(["process"], (input,))?;

        // Convert output to JSON
        let result: serde_json::Value = rune::from_value(output)
            .map_err(|e| PluginError::Runtime(format!("output conversion failed: {}", e)))?;

        // In phase 1, result is not displayed — only console output matters
        Ok(vec![])
    }
}
```

**Note:** The `rune::termcolor::StandardStream` routes diagnostics to stderr. Compile errors should be captured and converted to `ConsoleLine` entries. `println!` output in Rune scripts is routed through `std::io::println` — capturing this requires setting up a custom output stream wrapper (out of scope for initial implementation; phase 1 console output focuses on compile/runtime errors).

### Script Execution

1. `run_script(id, input)` called from frontend
2. Source loaded from filesystem via `read_script`
3. `RuneEngine::run(source, input)` executes:
   a. Source compiled via `rune::prepare()` with diagnostics
   b. Compile errors → `diagnostics.emit()` + `ConsoleLine { level: "error" }`
   c. Vm created from compiled unit
   d. `vm.call(["process"], (input,))` invoked
   e. Runtime errors caught and emitted as `ConsoleLine { level: "error" }`
4. All `ConsoleLine` entries (errors, warnings) streamed to frontend via Tauri events
5. `RunResult` returned when complete

---

## Console Streaming (Tauri Events)

Each `run_script` call generates a unique `run_id` (UUID). Tauri events include this ID so the frontend can route lines to the correct console pane (prevents crosstalk if multiple scripts run concurrently).

```rust
struct ConsoleLine {
    level: String,    // "info" | "error"
    message: String,
    run_id: String,  // correlation ID for concurrent execution
}

// Backend emits during script execution
tauri::Emitter::emit(&window, "script-console-line", ConsoleLine { run_id: run_id.clone(), .. });
```

Frontend listens for `script-console-line` events, matches on `run_id`, and appends to the corresponding console. On **Run**, the frontend generates a UUID and passes it implicitly (or as a session context). When execution completes, the final `RunResult` also includes all lines keyed by `run_id` for replay.

---

## File Structure

```
src-tauri/
├── src/
│   ├── plugins/
│   │   ├── rune_loader.rs      # load_rune_engine() — existing
│   │   └── ...
│   ├── scripts/                 # NEW: script runtime
│   │   ├── mod.rs
│   │   ├── engine.rs            # RuneEngine singleton, run()
│   │   └── commands.rs          # Tauri commands: list/read/save/run
│   ├── commands.rs              # existing plugin commands
│   └── main.rs                 # initialize RUNE_ENGINE at startup
├── resources/
│   └── scripts/                 # BUNDLED example scripts
│       └── hello.rn             # user data dir created at runtime
└── Cargo.toml
```

**User script directory** is created at runtime at `{app_data_dir}/scripts/`. On first save, if the directory does not exist, it is created. On Windows this resolves to e.g. `C:\Users\{user}\AppData\Roaming\gent\scripts\`.

```
src/
├── components/
│   ├── right_panel.rs           # Tabbed right panel (Trace/Scripts)
│   ├── script_editor.rs         # NEW: Scripts tab component
│   ├── script_console.rs        # NEW: Console output component
│   └── ...
```

---

## Bundled Scripts

Initial set of example scripts in `resources/scripts/`:

- `hello.rn` — basic hello world with `println!`
- `context_demo.rn` — demonstrates context/print with variables

---

## Script Tab Interactions

**Interaction list:**

- **Select script** — `read_script()` populates the editor
- **Edit script** — local state only, not persisted
- **Click Save** — `save_script()` persists to user script dir
- **Click Run** — `run_script()` streams output to console pane
- **Close tab / switch away** — editor content discarded, not auto-saved
| Click **Save** | `save_script()` → persist to `~/.gent/scripts/` |
| Click **Run** | `run_script()` → stream output to console pane |
| Close/tab away | Editor content discarded (not auto-saved) |

**Note:** Scripts are NOT yet connected to node execution. A "Run Script" node type is a future phase.

---

## Future Phases

- **Phase 2:** `code_execute` node type that calls `run_script()` as part of graph execution
- **Phase 3:** Capability-gated context access from scripts (read Gent context/memory)
- **Phase 4:** Debugger breakpoints in script editor

---

## Dependencies

- `rune = "0.13"` (already in Cargo.toml)
- `wasmtime = "22"` (already in Cargo.toml)
- CodeMirror 6 (frontend, via crate or CDN) — TBD: check existing frontend deps
