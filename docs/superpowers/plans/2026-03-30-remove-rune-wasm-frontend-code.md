# Remove Rune WASM Frontend Code Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the frontend Rune WASM plugin loader code (`RuneLoader`) left from the original design. The backend Rune scripting engine (`src/scripts/`) and Rune dependencies are kept — they are the current design.

**Architecture:** Remove only the frontend plugin layer that was designed to load Rune-compiled WASM modules into the frontend. This includes `RuneLoader`, the `load_rune_engine` bridge function, and all references to them. The backend `scripts/` module (which runs Rune `.rn` scripts via `RuneEngine`) is retained since that is the current design.

**Tech Stack:** Rust (Tauri), Cargo

---

## What to REMOVE (Frontend Rune WASM loader):
- `src-tauri/src/plugins/rune_loader.rs` — Rune WASM plugin loader (placeholder implementation)
- `src-tauri/src/plugins/mod.rs` — remove `rune_loader` module, `RuneLoader` export, `load_rune_engine` from `pub use loader`
- `src-tauri/src/plugins/loader.rs` — remove `load_rune_engine` function

## What to KEEP (Backend Rune engine):
- `src-tauri/src/scripts/` — backend Rune scripting engine (engine.rs, commands.rs, mod.rs)
- `src-tauri/Cargo.toml` — `rune` and `rune-modules` dependencies (needed by backend scripts)
- `public/scripts/hello.rn` — bundled Rune script (used by backend scripts)
- `src-tauri/src/lib.rs` — Rune engine initialization and script commands (needed by backend)

---

## Task 1: Remove RuneLoader from plugins/mod.rs

**Files:**
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Edit plugins/mod.rs**

Remove line 7 (`pub mod rune_loader;`) and line 19 (`pub use rune_loader::RuneLoader;`).

Change line 20 from:
```rust
pub use loader::{load_rune_engine, PluginLoader};
```
to:
```rust
pub use loader::PluginLoader;
```

Update the `WasmLoader` trait comment (line 22-26):
Old:
```rust
/// WASM loader trait - implemented by RuneLoader and RustWasmLoader
pub trait WasmLoader: Send + Sync {
```
New:
```rust
/// WASM loader trait - implemented by RustWasmLoader
pub trait WasmLoader: Send + Sync {
```

---

## Task 2: Remove load_rune_engine from plugins/loader.rs

**Files:**
- Modify: `src-tauri/src/plugins/loader.rs`

- [ ] **Step 1: Edit plugins/loader.rs**

Remove lines 48-57 (the `load_rune_engine` function entirely):

```rust
// REMOVE THIS:
/// Load the one and only Rune script engine.
///
/// The Rune engine is a singleton embedded in Gent, not a general plugin.
/// Separate from PluginLoader to avoid loader detection ambiguity.
pub fn load_rune_engine(
    wasm: &[u8],
    capabilities: &[Capability],
) -> Result<Box<dyn Plugin>, PluginError> {
    RuneLoader::new()
        .and_then(|loader| loader.load(wasm, capabilities))
}
```

Also remove `RuneLoader` from the import on line 4 if it becomes unused.

---

## Task 3: Delete rune_loader.rs

**Files:**
- Delete: `src-tauri/src/plugins/rune_loader.rs`

- [ ] **Step 1: Delete rune_loader.rs**

Use Bash: `rm src-tauri/src/plugins/rune_loader.rs`

---

## Task 4: Verify build

- [ ] **Step 1: Run cargo check**

Run: `cd src-tauri && cargo check 2>&1`
Expected: PASS with only pre-existing warnings (no Rune-related errors)

---

## Task 5: Full build verification

- [ ] **Step 1: Full Tauri build**

Run: `cd src-tauri && cargo tauri build 2>&1 | tail -20`
Expected: Build succeeds

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test 2>&1`
Expected: All tests pass
