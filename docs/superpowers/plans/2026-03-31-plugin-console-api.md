# Plugin Console API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement shared `ConsoleLine` type accessible by both Rune scripting engine and WASM plugins via wasmtime host imports.

**Architecture:** Console output flows through a shared `plugins/console.rs` module. WASM plugins call `log::println(msg)` via wasmtime linker host imports, captured into a `Mutex<Vec<ConsoleLine>>` per plugin instance. Rune scripts use the same `log::println` function that populates a thread-local buffer later drained into `ConsoleLine` entries.

**Tech Stack:** Rust, wasmtime 22, wasmtime-wasi 22, rune 0.22

---

## File Changes

- Create: `src-tauri/src/plugins/console.rs` — shared `ConsoleLine` struct (no `run_id`)
- Modify: `src-tauri/src/plugins/rust_loader.rs` — add `console_lines` field and `log::println` host import
- Modify: `src-tauri/src/plugins/mod.rs` — export `console` module
- Modify: `src-tauri/src/scripts/engine.rs` — remove local `ConsoleLine`, use shared module, drain `LOG_OUTPUT` as `ConsoleLine` entries

---

## Task 1: Create shared ConsoleLine module

**Files:**
- Create: `src-tauri/src/plugins/console.rs`

- [ ] **Step 1: Create plugins/console.rs with ConsoleLine struct**

```rust
use serde::{Deserialize, Serialize};

/// Shared console line structure for both Rune scripts and WASM plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
}

impl ConsoleLine {
    /// Create an output-level console line
    pub fn output(message: impl Into<String>) -> Self {
        Self { level: "output".into(), message: message.into() }
    }

    /// Create an error-level console line
    pub fn error(message: impl Into<String>) -> Self {
        Self { level: "error".into(), message: message.into() }
    }
}
```

- [ ] **Step 2: Run cargo check to verify compilation**

Run: `cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri" && cargo check 2>&1`
Expected: Compiles successfully (new file with no dependencies beyond serde)

- [ ] **Step 3: Commit**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent"
git add src-tauri/src/plugins/console.rs
git commit -m "feat(plugins): add shared ConsoleLine module"
```

---

## Task 2: Export console module from plugins

**Files:**
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Add console module export**

Add after line 7:
```rust
pub mod console;
```

Add to `pub use` exports at line 14:
```rust
pub use console::ConsoleLine;
```

- [ ] **Step 2: Run cargo check to verify**

Run: `cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri" && cargo check 2>&1`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent"
git add src-tauri/src/plugins/mod.rs
git commit -m "feat(plugins): export console module"
```

---

## Task 3: Add console capture to RustWasmPlugin

**Files:**
- Modify: `src-tauri/src/plugins/rust_loader.rs`

- [ ] **Step 1: Add Mutex import and ConsoleLine import**

Add to existing imports at top:
```rust
use crate::plugins::console::ConsoleLine;
use std::sync::Mutex;
```

- [ ] **Step 2: Add console_lines field to RustWasmPlugin struct**

Replace the `RustWasmPlugin` struct (lines 109-114):
```rust
/// Inner plugin instance for Rust WASM
struct RustWasmPlugin {
    engine: Engine,
    module: Module,
    manifest: Manifest,
    capabilities: Vec<Capability>,
    console_lines: Mutex<Vec<ConsoleLine>>,
}
```

- [ ] **Step 3: Update RustWasmPlugin construction in load()**

Replace lines 97-104 with:
```rust
let plugin = RustWasmPlugin {
    engine: self.engine.clone(),
    module,
    manifest: Manifest::default(),
    capabilities: capabilities.to_vec(),
    console_lines: Mutex::new(Vec::new()),
};
```

- [ ] **Step 4: Add log::println host function in process()**

Find the linker setup section (after line 131 `let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);`) and add before the `instantiate` call:

```rust
// Set up log::println host import for console capture
let console_lines = self.console_lines.clone();
linker.func_wrap("log", "println", move |msg: &str| {
    console_lines.lock().unwrap().push(ConsoleLine::output(msg));
})?;
```

- [ ] **Step 5: Drain console_lines into result after execution**

After `start.call()` succeeds and before `parse_output()`, add:
```rust
// Collect console lines from the plugin execution
let mut console_lines = self.console_lines.lock().unwrap();
let captured_lines: Vec<ConsoleLine> = console_lines.drain(..).collect();
drop(console_lines);
```

Note: The `captured_lines` should be returned alongside `Output`. However, the current `process()` returns `Result<Output, PluginError>` and `Output` is just `Output(pub serde_json::Value)`. For this initial implementation, we'll capture but not yet return them (future task will integrate with execution result). Log lines are still captured for future use.

- [ ] **Step 6: Run cargo check to verify compilation**

Run: `cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri" && cargo check 2>&1`
Expected: Compiles successfully

- [ ] **Step 7: Commit**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent"
git add src-tauri/src/plugins/rust_loader.rs
git commit -m "feat(plugins): add console capture for WASM plugins via log::println host import"
```

---

## Task 4: Update Rune engine to use shared ConsoleLine

**Files:**
- Modify: `src-tauri/src/scripts/engine.rs`

- [ ] **Step 1: Update imports and remove local ConsoleLine definition**

Replace lines 1-26 with:
```rust
use crate::plugins::console::ConsoleLine;
use crate::plugins::errors::PluginError;
use rune::diagnostics::Diagnostic;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Module, Source, Sources, Vm};
use serde::Serialize;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::OnceLock;

/// Log output buffer populated by log::println in Rune, drained after execution
thread_local! {
    static LOG_OUTPUT: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
```

Remove the entire `ConsoleLine` struct definition (lines 10-16) — it's now in `plugins/console.rs`.

- [ ] **Step 2: Update log_println to use ConsoleLine-compatible approach**

The current `log_println` pushes raw strings. Keep it as-is for now since it pushes to `LOG_OUTPUT` which is drained later. The change is in how we consume `LOG_OUTPUT`.

- [ ] **Step 3: Update run() method signature to remove run_id parameter**

Update the `run()` method signature at line 62. Change:
```rust
pub fn run(
    &self,
    source: &str,
    input: serde_json::Value,
    run_id: &str,
) -> Result<Vec<ConsoleLine>, PluginError> {
```

To:
```rust
pub fn run(
    &self,
    source: &str,
    input: serde_json::Value,
) -> Result<Vec<ConsoleLine>, PluginError> {
```

- [ ] **Step 4: Update all ConsoleLine constructions in run() to remove run_id**

Replace lines 87-91 (first ConsoleLine in diagnostics handling):
```rust
lines.push(ConsoleLine {
    level: "error".into(),
    message: format!("diagnostic emit failed: {}", e),
});
```

Replace lines 96-100 (Diagnostic::Fatal):
```rust
lines.push(ConsoleLine {
    level: "error".into(),
    message: f.to_string(),
});
```

Replace lines 103-107 (Diagnostic::Warning - preserves existing behavior):
```rust
lines.push(ConsoleLine {
    level: "warning".into(),
    message: w.to_string(),
});
```
Note: The "warning" level preserves existing behavior from before the shared module refactor.

Replace lines 134-138 (LOG_OUTPUT drain):
```rust
lines.push(ConsoleLine {
    level: "output".into(),
    message: msg,
});
```

Replace lines 143-147 (runtime error):
```rust
lines.push(ConsoleLine {
    level: "error".into(),
    message: format!("runtime error: {}", e),
});
```

- [ ] **Step 5: Find and update callers of `engine.run()`**

Search for callers of `RuneEngine::run` that pass `run_id`:

Run: `cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri" && grep -rn "run_id" --include="*.rs"`
Expected: List files that reference `run_id`

Check each file found:
- If caller is in `src-tauri/src/`: update it to remove `run_id` argument
- If caller is in UI layer (e.g., `src/`): update it to remove `run_id` argument

For each caller found, edit to remove the `run_id` parameter from the `.run()` call.

- [ ] **Step 6: Run cargo check to verify compilation**

Run: `cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri" && cargo check 2>&1`
Expected: Compiles successfully

- [ ] **Step 7: Commit**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent"
git add src-tauri/src/scripts/engine.rs
git commit -m "refactor(script_engine): use shared ConsoleLine, remove run_id"
```

---

## Task 5: Verify no TODO warnings from plugins code

**Files:**
- None (verification only)

- [ ] **Step 1: Run cargo check and confirm no TODO warnings in plugins**

Run: `cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri" && cargo check 2>&1 | grep -i "TODO.*console\|TODO.*plugin"`
Expected: No TODO warnings related to plugins or console

- [ ] **Step 2: Commit final state**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent"
git add -A
git commit -m "feat(plugins): complete plugin console API implementation"
```
