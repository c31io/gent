# Reactive Canvas Resize Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make canvas wire rendering reactively update when the canvas container resizes, eliminating the broken imperative resize handler and no-op debounce logic.

**Architecture:** Replace imperative canvas resize + redraw with reactive signals. Track viewport/container dimensions as signals, derive canvas dimensions, and use effects to handle canvas resizing and wire redrawing purely through Leptos reactivity.

**Tech Stack:** Leptos 0.7, wasm-bindgen, web-sys

---

## File Structure

- **Modify:** `src/components/canvas/canvas.rs`
  - Lines 48-50: Add viewport dimension signals
  - Lines 514-568: Replace imperative Effect with reactive canvas resize effect
  - Lines 569-588: Remove broken resize handler

---

## Task 1: Add Viewport Dimension Signals

**Files:**
- Modify: `src/components/canvas/canvas.rs:48-50`

- [ ] **Step 1: Add width/height signals after existing pan/zoom signals**

Current code (lines 48-50):
```rust
let (zoom, set_zoom) = signal(1.0f64);
let (pan_x, set_pan_x) = signal(0.0f64);
let (pan_y, set_pan_y) = signal(0.0f64);
```

Add after line 50:
```rust
let (canvas_width, set_canvas_width) = signal(0u32);
let (canvas_height, set_canvas_height) = signal(0u32);
```

- [ ] **Step 2: Run check to verify compilation**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/components/canvas/canvas.rs
git commit -m "feat(canvas): add viewport dimension signals"
```

---

## Task 2: Replace Imperative Effect with Reactive Canvas Resize

**Files:**
- Modify: `src/components/canvas/canvas.rs:514-568`

- [ ] **Step 1: Read current Effect::new implementation to understand redraw pattern**

The current Effect (lines 514-568):
- Gets canvas element and context
- Resizes canvas to match container
- Calls `draw_connections` with current state
- Has a nested resize listener that does nothing useful

- [ ] **Step 2: Rewrite the Effect to use reactive canvas_width/canvas_height signals**

Current code structure:
```rust
Effect::new(move |_| {
    let _lw = left_w.get();
    let _rw = right_w.get();
    // ... canvas resize and redraw
    // ... nested resize listener (broken)
});
```

Replace with:
```rust
// Canvas resize + redraw effect - runs when canvas dimensions or panel widths change
Effect::new(move |_| {
    // Track panel widths
    let _lw = left_w.get();
    let _rw = right_w.get();

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let canvas_elem = match document.get_element_by_id("wires-canvas") {
        Some(e) => e,
        None => return,
    };
    let canvas_ref: web_sys::HtmlCanvasElement = match canvas_elem.dyn_into() {
        Ok(c) => c,
        Err(_) => return,
    };

    // Get container and resize canvas to match
    if let Some(container) = canvas_ref.parent_element() {
        let container: web_sys::HtmlElement = match container.dyn_into() {
            Ok(c) => c,
            Err(_) => return,
        };
        let width = container.client_width() as u32;
        let height = container.client_height() as u32;

        // Update reactive signals so other parts can track
        set_canvas_width.set(width);
        set_canvas_height.set(height);

        canvas_ref.set_width(width);
        canvas_ref.set_height(height);
    }

    let ctx: web_sys::CanvasRenderingContext2d = match canvas_ref.get_context("2d") {
        Ok(Some(c)) => c.unchecked_into(),
        _ => return,
    };

    let connections = connections.get();
    let dragging = dragging_connection.get();
    let rerouting = rerouting_from.get();
    let nodes = nodes.get();
    let port_pos = port_positions.get();
    draw_connections(
        &ctx,
        &connections,
        &dragging,
        rerouting,
        &nodes,
        &port_pos,
        pan_x.get(),
        pan_y.get(),
        zoom.get(),
    );
});
```

**Key changes:**
- Uses `set_canvas_width` and `set_canvas_height` to update signals reactively
- Removes the broken nested resize listener (lines 569-588)
- Canvas now redraws when panel widths change (existing behavior)

- [ ] **Step 3: Run check to verify compilation**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/components/canvas/canvas.rs
git commit -m "feat(canvas): make canvas resize reactive via signals"
```

---

## Task 3: Move Resize Listener Outside Effect and Fix It

**Files:**
- Modify: `src/components/canvas/canvas.rs`

**Architecture:**
- Canvas Effect (the existing one at lines 514-568) tracks `left_w`, `right_w`, `canvas_width`, `canvas_height` signals
- When any of these change, the Effect re-runs → resizes canvas + redraws
- Resize listener is MOVED OUTSIDE the Effect (in `onmounted`) and just updates `canvas_width`/`canvas_height` signals
- This avoids infinite loop: resize listener sets signals → Effect re-runs → resize listener doesn't re-run (it's outside)

- [ ] **Step 1: Move resize listener outside the Effect block**

Current structure (lines 514-589):
```rust
Effect::new(move |_| {
    // ... canvas resize + redraw (lines 514-568)
    // ... resize listener (lines 569-588) - THIS IS INSIDE THE EFFECT (BUG)
});
```

After changes, structure should be:
```rust
// EFFECT 1: Canvas resize + redraw (tracks signals)
Effect::new(move |_| {
    let _lw = left_w.get();
    let _rw = right_w.get();
    let _cw = canvas_width.get();  // Track new signals
    let _ch = canvas_height.get();
    // ... canvas resize + redraw (lines 514-567)
});

// SEPARATE on_mounted-style setup: resize listener (runs once, outside Effect)
{
    // Resize listener code moved here, updates canvas_width/canvas_height signals
    // NOT inside an Effect that tracks those signals
}
```

The resize listener should be placed AFTER the Effect (before `view!` macro), in a `gloo_timers` setup similar to how `onmounted` works in Leptos. Use `gloo_timers::callback::Timeout` or `Closure::wrap` with `call_once` pattern.

- [ ] **Step 2: Fix the resize listener to update canvas_width/canvas_height signals**

Replace the broken resize handler (lines 569-588) with:

```rust
// Window resize listener - runs once to attach, updates signals on resize
// These signal changes trigger the canvas Effect to redraw
static RESIZE_LISTENER_ADDED: std::sync::Once = std::sync::Once::new();
let pan_x_clone = pan_x.clone();
let pan_y_clone = pan_y.clone();
let set_pan_x_clone = set_pan_x.clone();
let set_pan_y_clone = set_pan_y.clone();
let set_canvas_width_clone = set_canvas_width.clone();
let set_canvas_height_clone = set_canvas_height.clone();

let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_ev: web_sys::Event| {
    if let Some(w) = web_sys::window() {
        if let Some(d) = w.document() {
            if let Some(canvas_elem) = d.get_element_by_id("wires-canvas") {
                if let Ok(canvas_ref) = canvas_elem.dyn_into::<web_sys::HtmlCanvasElement>() {
                    if let Some(container) = canvas_ref.parent_element() {
                        if let Ok(container) = container.dyn_into::<web_sys::HtmlElement>() {
                            set_canvas_width_clone.set(container.client_width() as u32);
                            set_canvas_height_clone.set(container.client_height() as u32);
                        }
                    }
                }
            }
        }
    }
}) as Box<dyn Fn(_)>);

RESIZE_LISTENER_ADDED.call_once(|| {
    window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).ok();
});
closure.forget();
```

- [ ] **Step 3: Run check to verify compilation**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/components/canvas/canvas.rs
git commit -m "feat(canvas): fix resize listener to update dimension signals"
```

---

## Task 4: Remove Broken Resize Handler

**Files:**
- Modify: `src/components/canvas/canvas.rs:569-588`

- [ ] **Step 1: Remove the broken resize handler code**

Delete lines 569-588:
```rust
// Debounced resize handler to avoid excessive redraws
use gloo_timers::callback::Timeout;
use std::cell::RefCell;
static RESIZE_LISTENER_ADDED: std::sync::Once = std::sync::Once::new();
let resize_timeout: RefCell<Option<Timeout>> = RefCell::new(None);
let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_ev: web_sys::Event| {
    // Clear existing timeout
    resize_timeout.borrow_mut().take();
    // Set new timeout to debounce resize events
    let pan_x_clone = pan_x.clone();
    let set_pan_x_clone = set_pan_x.clone();
    *resize_timeout.borrow_mut() = Some(Timeout::new(10, move || {
        let current = pan_x_clone.get_untracked();
        set_pan_x_clone.set(current);
    }));
}) as Box<dyn Fn(_)>);
RESIZE_LISTENER_ADDED.call_once(|| {
    window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).ok();
});
closure.forget();
```

- [ ] **Step 2: Run check to verify compilation**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/components/canvas/canvas.rs
git commit -m "refactor(canvas): remove broken resize handler"
```

---

## Summary

After these changes:
1. Canvas dimensions are tracked as signals (`canvas_width`, `canvas_height`)
2. The main canvas Effect redraws when panel widths OR canvas dimensions change
3. A ResizeObserver (or fixed window resize handler) updates canvas dimensions reactively
4. Wires stay correctly positioned when the canvas resizes
5. The no-op `set_pan_x.set(pan_x.get())` logic is gone
