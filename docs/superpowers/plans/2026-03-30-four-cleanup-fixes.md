# Four Cleanup Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix four independent code quality issues in the frontend: deduplicate Tauri invoke boilerplate, replace a polling loop with callback-based signaling, cap the unbounded console buffer, and convert a stringly-typed field to an enum.

**Architecture:** Four independent changes. A new shared `src/tauri_invoke.rs` module holds the deduplicated invoke helper. The polling loop is eliminated by restructuring `init_editor` to be invoked directly after content loads. Console buffer caps at 10000 lines. `ConsoleLine.level` becomes a `ConsoleLevel` enum.

**Tech Stack:** Leptos 0.8, wasm-bindgen, wasm-bindgen-futures, gloo-timers.

---

## File Map

| File | Role |
|------|------|
| `src/tauri_invoke.rs` | **Create** — shared `tauri::invoke` helper |
| `src/main.rs` | **Modify** — register `tauri_invoke` module |
| `src/components/script_editor.rs` | **Modify** — use `tauri_invoke`, fix polling + console + enum |
| `src/components/plugin_manager.rs` | **Modify** — use `tauri_invoke` |

---

## Task 1: Extract shared `tauri::invoke` helper

**Files:**
- Create: `src/tauri_invoke.rs`
- Modify: `src/app.rs` (register module)
- Modify: `src/components/script_editor.rs:31-165` (replace body)
- Modify: `src/components/plugin_manager.rs:7-51` (replace body)

- [ ] **Step 1: Create `src/tauri_invoke.rs`**

```rust
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// Invoke a Tauri command and return the raw JsValue.
/// Returns Err(String) on any failure.
pub async fn invoke(js_cmd: String, args: &js_sys::Object) -> Result<JsValue, String> {
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;

    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;

    if tauri.is_undefined() {
        return Err("Only available in Tauri desktop app".to_string());
    }

    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke_fn = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args_arr = js_sys::Array::new();
    args_arr.push(&js_cmd.into());
    args_arr.push(args);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke_fn.into(), &JsValue::UNDEFINED, &args_arr)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    JsFuture::from(promise).await.map_err(|e| format!("promise error: {:?}", e))
}
```

- [ ] **Step 2: Register module in `src/main.rs`**

Find the module declarations near the top of `src/main.rs` (lines 1–2) and add:
```rust
mod tauri_invoke;
```
above `mod app;`. The module is accessible to all crates via `tauri_invoke::invoke`.

- [ ] **Step 3: Replace `list_scripts` body in `script_editor.rs`**

Replace lines 31–59 with:
```rust
async fn list_scripts() -> Result<Vec<ScriptInfo>, String> {
    let opts = js_sys::Object::new();
    let js_value = tauri_invoke::invoke("list_scripts".into(), &opts).await?;
    let scripts: Vec<ScriptInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(scripts)
}
```

- [ ] **Step 4: Replace `read_script` body in `script_editor.rs`**

Replace lines 61–96 with:
```rust
async fn read_script(id: String) -> Result<String, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    let js_value = tauri_invoke::invoke("read_script".into(), &opts).await?;

    #[derive(serde::Deserialize)]
    struct Content { source: String }
    let content: Content = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(content.source)
}
```

- [ ] **Step 5: Replace `save_script` body in `script_editor.rs`**

Replace lines 98–129 with:
```rust
async fn save_script(id: String, content: String) -> Result<(), String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    js_sys::Reflect::set(&opts, &"content".into(), &content.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    tauri_invoke::invoke("save_script".into(), &opts).await?;
    Ok(())
}
```

- [ ] **Step 6: Replace `run_script` body in `script_editor.rs`**

Replace lines 131–165 with:
```rust
async fn run_script(id: String) -> Result<RunResult, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    let empty_input = JsValue::from(js_sys::Object::new());
    js_sys::Reflect::set(&opts, &"input".into(), &empty_input)
        .map_err(|e| format!("set error: {:?}", e))?;
    let js_value = tauri_invoke::invoke("run_script".into(), &opts).await?;
    let result: RunResult = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(result)
}
```

- [ ] **Step 7: Replace `list_plugins` body in `plugin_manager.rs`**

Replace lines 6–51 with:
```rust
async fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    let opts = js_sys::Object::new();
    let js_value = tauri_invoke::invoke("list_plugins".into(), &opts).await?;
    let plugins: Vec<PluginInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deserialization failed: {:?}", e))?;
    Ok(plugins)
}
```

- [ ] **Step 8: Verify `trunk serve` compiles without errors**

Run: `trunk serve 2>&1 | head -50`
Expected: Compiles successfully, no warnings about unused imports in `script_editor.rs` or `plugin_manager.rs`.

- [ ] **Step 9: Commit**

```bash
git add src/tauri_invoke.rs src/app.rs src/components/script_editor.rs src/components/plugin_manager.rs
git commit -m "refactor: extract shared tauri_invoke helper
Consolidates ~60 lines of duplicate boilerplate across list_scripts,
read_script, save_script, run_script, and list_plugins into a single
tauri_invoke::invoke() function."
```

---

## Task 2: Replace polling loop with callback handoff

**Files:**
- Modify: `src/components/script_editor.rs:387-470` (restructure `init_editor`)

The polling (lines 397–404) waits for `pending_content` to be set. The fix: call `init_editor()` directly from the `spawn_local` closure that sets `pending_content`, instead of calling it unconditionally at line 472.

- [ ] **Step 1: Modify `spawn_local` in mount closure to call `init_editor()` after setting pending content**

In the `spawn_local` block starting at line 187, find where `set_pending_content.set(Some(source.clone()))` is called (line 198). After that line, add a call to `init_editor()`:

```rust
// Inside the spawn_local after set_pending_content.set:
init_editor();
```

- [ ] **Step 2: Remove unconditional `init_editor()` call at line 472**

Delete the standalone `init_editor();` call at the bottom of the component body.

- [ ] **Step 3: Remove the polling for loop and 100ms delays in `init_editor`**

In `init_editor` (lines 394–404), replace the `for _ in 0..50` polling with a direct read of `pending_content.get()`:

```rust
// Before: for _ in 0..50 { if let Some(c) = pending_content.get() { ... } gloo_timers::future::TimeoutFuture::new(100).await; }
// After:
let initial_content = pending_content.get().unwrap_or_default();
```

- [ ] **Step 4: Verify `trunk serve` compiles**

Run: `trunk serve 2>&1 | head -50`
Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add src/components/script_editor.rs
git commit -m "fix(script_editor): replace polling loop with direct init_editor call
Eliminates 50x100ms polling by invoking init_editor directly after
pending_content is set, rather than polling on mount."
```

---

## Task 3: Cap console buffer at 10000 lines

**Files:**
- Modify: `src/components/script_editor.rs:247-254` (add truncate)

- [ ] **Step 1: Add truncation after pushing to console_lines**

In the `set_console_lines.update` closure (lines 249–252), add a truncate. Replace the closure body with:

```rust
set_console_lines.update(|lines| {
    if lines.len() >= 10000 {
        lines.drain(0..lines.len() - 10000);
    }
    lines.push(cl);
});
```

Also update the init console line push (line 256–262) to use the same truncation.

- [ ] **Step 2: Verify `trunk serve` compiles**

Run: `trunk serve 2>&1 | head -50`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add src/components/script_editor.rs
git commit -m "fix(script_editor): cap console_lines at 10000 entries
Prevents unbounded memory growth from accumulated console output."
```

---

## Task 4: Convert `ConsoleLine.level` to `ConsoleLevel` enum

**Files:**
- Modify: `src/components/script_editor.rs` (enum + usage)

- [ ] **Step 1: Add `ConsoleLevel` enum and update `ConsoleLine`**

After the `ScriptInfo` struct (around line 14), add:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConsoleLevel {
    Info,
    Warn,
    Error,
}

impl Default for ConsoleLevel {
    fn default() -> Self {
        ConsoleLevel::Info
    }
}
```

Then update `ConsoleLine` (lines 17–22) to:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsoleLine {
    pub level: ConsoleLevel,
    pub message: String,
    pub run_id: String,
}
```

- [ ] **Step 2: Update the hardcoded init console line**

Find `level: "info".into()` (line 258) and replace with `level: ConsoleLevel::Info`.

- [ ] **Step 3: Update the view rendering**

In the view (around line 555), replace:
```rust
let cls = if line.level == "error" { "console-error" } else { "console-info" };
```
with:
```rust
let cls = match line.level {
    ConsoleLevel::Error => "console-error",
    ConsoleLevel::Warn => "console-warn",
    ConsoleLevel::Info => "console-info",
};
```

And replace `format!("[{}]", line.level)` with a match that produces `"info"`, `"warn"`, `"error"`.

- [ ] **Step 4: Verify `trunk serve` compiles**

Run: `trunk serve 2>&1 | head -50`
Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add src/components/script_editor.rs
git commit -m "fix(script_editor): replace stringly-typed ConsoleLevel with enum
Changes level from String to ConsoleLevel::{Info, Warn, Error}.
Wire format change — no version compat needed per user guidance."
```

---

## Final Verification

- [ ] Run `trunk serve` — exit code 0, no errors, no warnings
- [ ] Run `cargo check` — exit code 0, no errors
- [ ] `git log --oneline -5` — four clean commits on `fix/cleanup-four-issues`
