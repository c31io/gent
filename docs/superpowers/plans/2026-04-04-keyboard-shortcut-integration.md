# Keyboard Shortcut Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add keyboard shortcuts (Ctrl+C/V/S, Delete, Escape, Ctrl+A) to activate existing multi-select features without breaking canvas mouse events.

**Architecture:** The core problem is that `window.addEventListener("keydown", ...)` breaks canvas mouse events in Leptos/WASM, while `resize` listeners work fine with identical patterns. We'll systematically debug this by trying alternative event listener approaches: document-level listeners, passive option, and capture phase. Keyboard actions will dispatch to existing `save_load.rs` functions.

**Tech Stack:** Leptos 0.8, web-sys, wasm-bindgen, wasm_bindgen_futures

---

## Problem Analysis

From `memory/keyboard-shortcut-window-listener-issue.md`:
- Canvas click/drag works correctly
- Adding ANY `window.addEventListener("keydown", ...)` breaks canvas mouse events
- The `resize` listener in canvas.rs (lines 676-689) works fine with identical pattern

**Working resize pattern:**
```rust
static RESIZE_LISTENER_ADDED: std::sync::Once = std::sync::Once::new();
let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_ev: web_sys::Event| {
    set_resize_gen_clone.update(|g| *g += 1);
}) as Box<dyn Fn(_)>);
RESIZE_LISTENER_ADDED.call_once(|| {
    if let Some(w) = web_sys::window() {
        w.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).ok();
    }
});
closure.forget();
```

**Failed approaches documented:**
1. `on:keydown` on app-layout div with `tabindex="0"` - broke canvas
2. Window-level listener with identical pattern to resize - broke canvas
3. Tabindex removal still left canvas broken (state was polluted)

**Possible root causes:**
- `KeyboardEvent` vs `Event` type handling in wasm-bindgen
- Event phase differences between keydown and resize
- Some interaction with leptos event system or WASM memory

---

## File Map

- `src/components/app_layout.rs` - Main layout, keyboard shortcut integration goes here
- `src/components/canvas/canvas.rs` - Has working resize listener (lines 676-689), reference for pattern
- `src/components/save_load.rs` - Already implemented: `copy_to_clipboard`, `paste_from_clipboard`, `load_selection`, `strip_credentials`
- `src/components/toast.rs` - Already implemented: `ToastContainer`
- `src/components/modal.rs` - Already implemented: `ConfirmModal`, `CredentialPromptModal`
- `src/components/graph_section.rs` - Already implemented: `GraphSection` (not yet integrated into LeftPanel)
- `src/components/canvas/state.rs` - `SavedSelection`, `BundledGroup`, `NodeState`, `ConnectionState` types
- `src/components/canvas/geometry.rs` - `is_text_input_keyboard()` helper

---

## Tasks

### Task 1: Debug window keydown listener issue systematically

**Files:**
- Modify: `src/components/app_layout.rs` - Add keyboard listener attempts
- Read: `src/components/canvas/canvas.rs:676-689` - Reference for working resize pattern

- [ ] **Step 1: Create isolated test component `KeyboardTest.rs`**

Create `src/components/keyboard_test.rs` with a minimal test that adds a keydown listener and reports whether canvas mouse events work:

```rust
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn KeyboardTest(
    set_keyboard_test_result: WriteSignal<Option<String>>,
) -> impl IntoView {
    // Test 1: Try document-level keydown listener
    std::sync::Once::new();
    static DOC_LISTENER_ADDED: std::sync::Once = std::sync::Once::new();

    on_mount(move || {
        let set_result = set_keyboard_test_result;
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            let key = ev.key();
            gloo_timers::future::TimeoutFuture::new(100).await;
            set_result.set(Some(format!("Key pressed: {}", key)));
        }) as Box<dyn Fn(_)>);

        DOC_LISTENER_ADDED.call_once(|| {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                doc.add_event_listener_with_event_listener("keydown", closure.as_ref().unchecked_ref()).ok();
            }
        });
        closure.forget();
    });

    view! { <div id="keyboard-test">"Test"</div> }
}
```

- [ ] **Step 2: Create a more comprehensive listener test in canvas.rs**

Add multiple test listener approaches at the bottom of `Canvas` component, each with a unique `Once` static:

```rust
// APPROACH A: document-level listener
static DOC_KEYDOWN_ADDED: std::sync::Once = std::sync::Once::new();
let doc_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
    web_sys::console::log_1(&format!("DOC keydown: {}", ev.key()).into());
}) as Box<dyn Fn(_)>);
DOC_KEYDOWN_ADDED.call_once(|| {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        let _ = doc.add_event_listener_with_event_listener("keydown", doc_closure.as_ref().unchecked_ref());
    }
});
doc_closure.forget();

// APPROACH B: window-level with passive: true option
static PASSIVE_KEYDOWN_ADDED: std::sync::Once = std::sync::Once::new();
let passive_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
    web_sys::console::log_1(&format!("PASSIVE keydown: {}", ev.key()).into());
}) as Box<dyn Fn(_)>);
PASSIVE_KEYDOWN_ADDED.call_once(|| {
    if let Some(w) = web_sys::window() {
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(true);
        let _ = w.add_event_listener_with_options("keydown", passive_closure.as_ref().unchecked_ref(), &options);
    }
});
passive_closure.forget();

// APPROACH C: capture phase
static CAPTURE_KEYDOWN_ADDED: std::sync::Once = std::sync::Once::new();
let capture_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
    web_sys::console::log_1(&format!("CAPTURE keydown: {}", ev.key()).into());
}) as Box<dyn Fn(_)>);
CAPTURE_KEYDOWN_ADDED.call_once(|| {
    if let Some(w) = web_sys::window() {
        let options = web_sys::AddEventListenerOptions::new();
        options.set_capture(true);
        let _ = w.add_event_listener_with_options("keydown", capture_closure.as_ref().unchecked_ref(), &options);
    }
});
capture_closure.forget();
```

- [ ] **Step 3: Test each approach and identify which works**

Run `trunk serve` and check console for "DOC keydown", "PASSIVE keydown", and "CAPTURE keydown" messages while also testing canvas click/drag.

- [ ] **Step 4: Identify the root cause**

Based on test results:
- If only one approach works, use that
- If none work, the issue is deeper in the event system
- Document findings

- [ ] **Step 5: Clean up test approaches**

Keep only the working approach, remove test code.

---

### Task 2: Integrate keyboard shortcuts into AppLayout

**Files:**
- Modify: `src/components/app_layout.rs` - Add keyboard shortcut handler

- [ ] **Step 1: Add keyboard event state signals**

Add to AppLayout signals section:
```rust
// Keyboard shortcut state
let (saved_selections, set_saved_selections) = signal(Vec::<SavedSelection>::new());
let (toasts, set_toasts) = signal(Vec::<Toast>::new());
let (next_toast_id, set_next_toast_id) = signal(0u32);
let (confirm_modal_visible, set_confirm_modal_visible) = signal(false);
let (confirm_action, set_confirm_action) = signal(Option::<Callback<()>>::None);
```

- [ ] **Step 2: Add toast helper functions**

```rust
let add_toast = move |message: String, toast_type: ToastType| {
    let id = next_toast_id.get();
    set_next_toast_id.update(|n| *n += 1);
    set_toasts.update(|t| t.push(Toast { id, message, toast_type }));
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(3000).await;
        set_toasts.update(|t| t.retain(|toast| toast.id != id));
    });
};
```

- [ ] **Step 3: Wire up GraphSection to LeftPanel**

In `LeftPanel`, add `GraphSection` component with callbacks for load/delete selection.

- [ ] **Step 4: Add the working keyboard listener to AppLayout**

Based on Task 1 results, add the working keydown listener approach.

- [ ] **Step 5: Implement shortcut dispatch logic**

```rust
let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    let ctrl = ev.ctrl_key() || ev.meta_key();
    let key = ev.key();

    // Ignore if focus is in text input
    if is_text_input_keyboard(&ev) {
        return;
    }

    match (ctrl, key.as_str()) {
        (true, "c") => {
            // Copy selection to clipboard
            ev.prevent_default();
            let selection = /* build SavedSelection from selected_node_ids */;
            spawn_local(async move {
                match copy_to_clipboard(selection, true).await {
                    Ok(_) => add_toast("Copied to clipboard".to_string(), ToastType::Success),
                    Err(e) => add_toast(format!("Copy failed: {}", e), ToastType::Error),
                }
            });
        }
        (true, "v") => {
            // Paste from clipboard
            ev.prevent_default();
            spawn_local(async move {
                match paste_from_clipboard().await {
                    Ok(selection) => {
                        let (nodes, conns, next_id, next_conn) = load_selection(selection, next_node_id.get(), next_connection_id.get());
                        set_nodes.update(|n| n.extend(nodes));
                        set_connections.update(|c| c.extend(conns));
                        set_next_node_id.set(next_id);
                        set_next_connection_id.set(next_conn);
                        add_toast("Pasted from clipboard".to_string(), ToastType::Success);
                    }
                    Err(e) => add_toast(format!("Paste failed: {}", e), ToastType::Error),
                }
            });
        }
        (true, "s") => {
            // Save selection
            ev.prevent_default();
            if selected_node_ids.get().is_empty() {
                add_toast("No selection to save".to_string(), ToastType::Info);
                return;
            }
            // Show save dialog (or auto-save with generated name)
            let selection = /* build SavedSelection */;
            let mut selections = saved_selections.get();
            selections.push(selection);
            save_saved_selections_to_storage(&selections);
            set_saved_selections.set(selections);
            add_toast("Selection saved".to_string(), ToastType::Success);
        }
        (true, "a") => {
            // Select all
            ev.prevent_default();
            let all_ids: HashSet<u32> = nodes.get().iter().map(|n| n.id).collect();
            set_selected_node_ids.set(all_ids);
        }
        (_, "Delete") | (_, "Backspace") => {
            // Delete selected nodes
            ev.prevent_default();
            let to_delete = selected_node_ids.get();
            if to_delete.is_empty() { return; }
            // Animate and delete
            if let Some(first_id) = to_delete.iter().next().copied() {
                set_deleting_node_id.set(Some(first_id));
            }
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(300).await;
                set_nodes.update(|n| n.retain(|node| !to_delete.contains(&node.id)));
                set_connections.update(|c| c.retain(|conn|
                    !to_delete.contains(&conn.source_node_id) && !to_delete.contains(&conn.target_node_id)
                ));
                set_deleting_node_id.set(None);
                set_selected_node_ids.update(|ids| ids.clear());
            });
        }
        (_, "Escape") => {
            // Clear selection
            set_selected_node_ids.update(|ids| ids.clear());
            if let Some(callback) = on_selection_change {
                callback.run(None);
            }
        }
        _ => {}
    }
};
```

- [ ] **Step 6: Add ToastContainer and modals to AppLayout view**

```rust
<ToastContainer toasts={toasts.into()} on_dismiss={Callback::new(move |id| {
    set_toasts.update(|t| t.retain(|toast| toast.id != id));
})} />

<ConfirmModal
    visible={confirm_modal_visible.get()}
    title="Confirm Delete".to_string()
    message="Delete selected nodes?".to_string()
    on_confirm={Callback::new(move |_| {
        set_confirm_modal_visible.set(false);
        // Perform delete
    })}
    on_cancel={Callback::new(move |_| {
        set_confirm_modal_visible.set(false);
    })}
/>
```

---

### Task 3: Wire up GraphSection integration

**Files:**
- Modify: `src/components/left_panel.rs` - Add GraphSection component
- Modify: `src/components/app_layout.rs` - Pass saved selections signal

- [ ] **Step 1: Import GraphSection in left_panel.rs**

Add `use crate::components::graph_section::GraphSection;` and wire it into LeftPanel view.

- [ ] **Step 2: Pass saved_selections signal from AppLayout to LeftPanel to GraphSection**

- [ ] **Step 3: Implement on_load_selection callback**

When user clicks a saved selection in GraphSection, load it into canvas.

- [ ] **Step 4: Implement on_delete_selection callback**

When user clicks × on a saved selection, remove it from saved_selections and localStorage.

- [ ] **Step 5: Load saved selections on mount**

In AppLayout's on_mount or component initialization, call `load_saved_selections()` and set the signal.

---

### Task 4: Test all keyboard shortcuts

**Files:**
- Test manually with `trunk serve`

- [ ] **Step 1: Test Ctrl+C with selection**

Select nodes, press Ctrl+C, verify toast "Copied to clipboard"

- [ ] **Step 2: Test Ctrl+V**

Press Ctrl+V, verify pasted nodes appear on canvas

- [ ] **Step 3: Test Ctrl+S**

Select nodes, press Ctrl+S, verify saved to localStorage and appears in GraphSection

- [ ] **Step 4: Test Delete**

Select nodes, press Delete, verify nodes removed with animation

- [ ] **Step 5: Test Escape**

Press Escape, verify selection cleared

- [ ] **Step 6: Test Ctrl+A**

Press Ctrl+A, verify all nodes selected

- [ ] **Step 7: Verify canvas still works**

After all shortcut tests, verify click/drag on canvas still works

---

## Success Criteria

1. All keyboard shortcuts (Ctrl+C/V/S, Delete, Escape, Ctrl+A) work
2. Canvas click/drag remains fully functional
3. Toast notifications appear for clipboard operations
4. Saved selections appear in GraphSection and persist across page reload
5. Copy/paste works across browser tabs (clipboard API)

## Notes

- The clipboard API requires secure context (HTTPS or localhost)
- `copy_to_clipboard` and `paste_from_clipboard` are async and need `spawn_local`
- Delete key conflicts with browser back - use `ev.prevent_default()`
- Escape should clear selection but not interfere with other behaviors
