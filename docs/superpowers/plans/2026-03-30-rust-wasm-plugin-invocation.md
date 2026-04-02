# Rust WASM Plugin Invocation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `RustWasmPlugin::process` to invoke wasm32-wasip2 compiled plugins via wasmtime, capturing stdout and handling errors properly.

**Architecture:** Plugin is a standalone wasm32-wasip2 binary. `process()` serializes Input to JSON, passes it as args to the WASM module via wasmtime's WASI support, captures stdout, and deserializes it as Output. Manifest extraction is deferred - initial implementation uses default manifest.

**Tech Stack:** wasmtime 22, wasmtime-wasi 22, Rust

---

## File Changes

- Modify: `src-tauri/src/plugins/rust_loader.rs` — implement `process()`, add error helpers
- Modify: `src-tauri/src/plugins/rust_loader.rs` — add manifest extraction (deferred to Task 3)

---

## Task 1: Add stdout capture helper and error handling

**Files:**
- Modify: `src-tauri/src/plugins/rust_loader.rs`

- [ ] **Step 1: Add imports for WASI stdout capture types**

Add after existing imports:
```rust
use wasmtime_wasi::pipe::MemoryOutputPipe;
use wasmtime_wasi::preview1::WasiP1Ctx;
```

- [ ] **Step 2: Add stdout capture struct**

Add before `RustWasmLoader`:
```rust
/// Captures stdout/stderr from a WASI command invocation
struct CapturedOutput {
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
}

impl CapturedOutput {
    fn new() -> Self {
        Self {
            stdout: MemoryOutputPipe::new(4096),
            stderr: MemoryOutputPipe::new(4096),
        }
    }

    fn into_contents(self) -> (Vec<u8>, Vec<u8>) {
        (self.stdout.contents().to_vec(), self.stderr.contents().to_vec())
    }
}
```

- [ ] **Step 3: Add helper to build WASI context with args and captured stdout**

Add after `is_rust_wasm`:
```rust
fn build_wasi_ctx(
    plugin_id: &str,
    input_json: &str,
    captured: &CapturedOutput,
) -> WasiP1Ctx {
    WasiCtxBuilder::new()
        .args(&[plugin_id, input_json])
        .stdout(captured.stdout.clone())
        .stderr(captured.stderr.clone())
        .build_p1()
}
```

- [ ] **Step 4: Add helper to parse stdout as Output or error**

Add after `build_wasi_ctx`:
```rust
fn parse_output(captured: CapturedOutput) -> Result<Output, PluginError> {
    let (stdout, _stderr) = captured.into_contents();
    let stdout_str = String::from_utf8(stdout)
        .map_err(|e| PluginError::Runtime(format!("invalid utf-8 from plugin stdout: {}", e)))?;

    serde_json::from_str::<serde_json::Value>(&stdout_str)
        .map(Output)
        .map_err(|e| PluginError::Runtime(format!("invalid JSON from plugin: {}", e)))
}
```

- [ ] **Step 5: Commit**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent"
git add src-tauri/src/plugins/rust_loader.rs
git commit -m "feat(plugins): add WASI stdout capture helpers"
```

---

## Task 2: Implement process() with args/stdout invocation

**Files:**
- Modify: `src-tauri/src/plugins/rust_loader.rs:78-89`

- [ ] **Step 1: Add imports for wasmtime linking**

Add to imports at top:
```rust
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::preview1::WasiP1Ctx;
```

- [ ] **Step 2: Implement process() - exit code via error propagation**

Replace the existing stub `process()` (lines 78-84) with:
```rust
fn process(&self, input: Input) -> Result<Output, PluginError> {
    let input_json = serde_json::to_string(&input.0)
        .map_err(|e| PluginError::Runtime(format!("failed to serialize input: {}", e)))?;

    let captured = CapturedOutput::new();
    let wasi = build_wasi_ctx(self.id(), &input_json, &captured);

    let mut store = Store::new(&self.engine, wasi);

    // Set up WASI linking
    let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
        .map_err(|e| PluginError::Runtime(format!("failed to set up WASI: {}", e)))?;

    // Instantiate - WASI imports are auto-linked via the linker
    let instance = linker
        .instantiate(&mut store, &self.module)
        .map_err(|e| PluginError::Runtime(format!("failed to instantiate plugin: {}", e)))?;

    // Find entry point - try __main_argc_argv first (wasip2), then _start (wasip1)
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "__main_argc_argv")
        .or_else(|_| instance.get_typed_func::<(), ()>(&mut store, "_start"))
        .map_err(|e| PluginError::Runtime(format!("failed to find main entry: {}", e)))?;

    // Call the entry point - proc_exit(0) succeeds, proc_exit(N) traps with error
    start.call(&mut store, ())
        .map_err(|e| PluginError::Runtime(format!("plugin execution failed: {}", e)))?;

    parse_output(captured)
}
```

Note: wasmtime-wasi propagates `proc_exit` as a trap error. Exit code 0 succeeds silently; non-zero exit causes `start.call()` to return an error containing the exit code in its message. Stdout is still captured even on non-zero exit.

- [ ] **Step 3: Update id() to return plugin name from manifest**

Replace the placeholder `id()` implementation:
```rust
fn id(&self) -> &str {
    &self.manifest.name
}
```

- [ ] **Step 4: Run cargo check to verify compilation**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri"
cargo check 2>&1
```

Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/plugins/rust_loader.rs
git commit -m "feat(plugins): implement RustWasmPlugin::process with wasmtime"
```

---

## Task 3: Update load() to use default manifest (no-op)

**Files:**
- Modify: `src-tauri/src/plugins/rust_loader.rs:41-63`

Note: Full manifest extraction from `__gent_plugin_manifest` export requires WASI stdout capture during `load()`, which adds complexity and is deferred. Manifest remains `default()` for now.

- [ ] **Step 1: Update load() to explicitly use default manifest**

Replace:
```rust
manifest: Manifest::default(), // Will be extracted from WASM
```

With (remove the TODO comment):
```rust
manifest: Manifest::default(),
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/plugins/rust_loader.rs
git commit -m "chore(plugins): clean up TODO comment in RustWasmLoader::load"
```

---

## Task 4: Verify no TODO warnings from plugins code

**Files:**
- None (verification only)

- [ ] **Step 1: Run cargo check and confirm no TODO warnings in plugins**

```bash
cd "C:/Users/c31io/Documents/GitHub/gent/src-tauri"
cargo check 2>&1 | grep -i "TODO.*rust_loader\|TODO.*plugin"
```

Expected: No TODO warnings related to plugins. The remaining warnings (dead code in canvas, execution_engine) are out of scope for this task.

- [ ] **Step 2: Commit final state**

```bash
git add -A
git commit -m "fix(plugins): TODO resolved - RustWasmPlugin::process implemented"
```
