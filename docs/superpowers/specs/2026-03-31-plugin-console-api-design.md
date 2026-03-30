# Plugin Console API Design

## Status
Approved

## Overview

Refactor logging/console output to use a shared module accessible by both Rune scripting engine and WASM plugins via wasmtime.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  plugins/console.rs                         │
│  ConsoleLine { level: String, message: String }            │
└─────────────────────────────────────────────────────────────┘
                           ▲
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────┴──────┐ ┌──────┴──────┐       │
    │ Rune Engine │ │ WASM Loader │       │
    │ log::println│ │ log::println│       │
    └─────────────┘ └─────────────┘       │
```

## Components

### `ConsoleLine` struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
}

impl ConsoleLine {
    pub fn output(message: impl Into<String>) -> Self {
        Self { level: "output".into(), message: message.into() }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self { level: "error".into(), message: message.into() }
    }
}
```

### WASM Plugin Console Capture

**`RustWasmPlugin` struct** — add `console_lines` field:
```rust
struct RustWasmPlugin {
    engine: Engine,
    module: Module,
    manifest: Manifest,
    capabilities: Vec<Capability>,
    console_lines: Mutex<Vec<ConsoleLine>>,
}
```

**Linker setup in `process()`:**
```rust
let console_lines = self.console_lines.clone();
linker.func_wrap("log", "println", move |msg: &str| {
    console_lines.lock().unwrap().push(ConsoleLine::output(msg));
})?;
```

Console lines are captured during plugin execution and returned alongside the plugin output.

### Rune Integration

Update `scripts/engine.rs` to use shared `ConsoleLine`:
- Move `log_println` to push `ConsoleLine::output(msg)` instead of raw strings
- Drain `LOG_OUTPUT` buffer into result as `ConsoleLine` entries

## Data Flow

```
Plugin Execution
    │
    ├───> WASI stdout ──> parse_output() ──> Output(JSON)
    │
    └───> log::println() ──> ConsoleLine buffer ──> Vec<ConsoleLine>
```

## File Changes

- Create: `src-tauri/src/plugins/console.rs` — shared ConsoleLine
- Modify: `src-tauri/src/plugins/rust_loader.rs` — add console capture for WASM
- Modify: `src-tauri/src/plugins/mod.rs` — export console module
- Modify: `src-tauri/src/scripts/engine.rs` — use shared ConsoleLine

## Notes

- `run_id` removed — not needed for plugin invocations
- Console capture is per-plugin-instance via `Mutex<Vec<ConsoleLine>>`
- WASM plugins call `log::println(msg: &str)` host function
