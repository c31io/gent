# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Gent is a visual node editor for context engineering and agent orchestration — a mix of Node-RED, Unreal Engine Blueprints, and LangChain purpose-built for building reasoning workflows. The UI is a Figma-like three-panel layout with a canvas in the center.

## Build Commands

- **Dev server**: `trunk serve` (runs on http://localhost:1420)
- **Production build**: `trunk build` (outputs to `dist/`)
- **Tauri build**: `cargo tauri build`
- **Check (frontend)**: `cargo check` (in project root - checks WASM/Leptos)
- **Check (backend)**: `cd src-tauri && cargo check` (checks Tauri Rust code)

## Architecture

### Three-Panel Layout
- **Left Panel** (`src/components/left_panel.rs`): Node palette with 13 predefined node types across 6 categories (Input, Context, Agent, Tool, Control, Output)
- **Canvas** (`src/components/canvas.rs`): DOM-based node rendering with pan/zoom via CSS transforms
- **Right Panel** (`src/components/right_panel.rs`): Execution trace display

### Supporting Components
- `save_load.rs`: File export/import, clipboard copy/paste, localStorage persistence
- `toast.rs`: Toast notification system (`ToastContainer`, `Toast`, `ToastType`)
- `modal.rs`: Modal dialogs (`ConfirmModal`, `CredentialPromptModal`)
- `graph_section.rs`: Left panel UI for saved selections

### Core Stack

- **Leptos 0.8** with CSR (client-side rendering) via `wasm-bindgen`
- **web-sys** for DOM event handling (mouse events for pan/resize)
- **Tauri** for desktop shell

### Tauri API Browser/Desktop Detection

- Check `__TAURI__` via `web_sys::window()` and test with `is_undefined()` before invoking. Return user-friendly errors instead of cryptic TypeErrors in browser dev. See `plugin_manager.rs` for the pattern.

### WASM Compatibility
- `std::time::Instant` doesn't work in WASM - use `js_sys::Date::now()` for timestamps
- `Timestamp` struct in execution_engine.rs wraps this pattern

### Leptos 0.8 Lifecycle / Mount Hooks

**`on_mount` does not exist in Leptos 0.8.** There is no `componentDidMount` equivalent.

For deferred initialization, call `spawn_local` at component top level — it runs when the component is instantiated:
```rust
#[component]
pub fn MyComponent() -> impl IntoView {
    spawn_local(async {
        // runs after component is created
        do_something().await;
    });
    view! { ... }
}
```

For cleanup, use `leptos::prelude::on_cleanup` (runs when component is destroyed).

### Key Components
- `AppLayout` (`app_layout.rs`): Manages panel sizes via Leptos signals, handles divider drag-to-resize
- `Canvas` (`canvas.rs`): Renders `GraphNode` components, handles pan (mouse drag) and zoom (scroll wheel)
- `GraphNode` (`nodes/node.rs`): Individual DOM-based nodes with input/output ports
- `wires-canvas` element: Canvas 2D bezier wires rendered via `draw_connections()` in canvas.rs

### Wire/Connection State
- `ConnectionState`: Persistent wire between source_node_id (output) and target_node_id (input)
- `DraggingConnection`: In-progress wire drag with `source_input_node_id` for reroute tracking
- `rerouting_from` signal: Tracks which input port is being rerouted from (dims original wire)

### Multi-Select State
- Selection uses `HashSet<u32>` of node IDs (not `Option<u32>`)
- Keyboard shortcuts: Ctrl+C (copy), Ctrl+V (paste), Ctrl+S (save), Ctrl+E (export), Ctrl+I (import), Delete, Escape, Ctrl+A (select all)
- `SavedSelection` struct in `state.rs` for persistent saves
- `load_selection()` in `save_load.rs` remaps IDs when loading

### Canvas Interaction Patterns

- Click vs drag: 5px movement threshold (`dx < 5.0 && dy < 5.0` in node.rs)
- Port events: NO stop_propagation() - let events bubble to canvas for reliable handling
- Trigger button: `.trigger-btn` class on node headers triggers execution
- Node Interaction Guards: When handling node clicks, check `is_trigger_button()` and `is_text_input()` before setting selection/drag state. See `canvas.rs` click handler and `geometry.rs` helpers.

### Canvas Geometry

- `src/components/canvas/geometry.rs` - DOM-based hit testing via `element_from_point()` and `data-*` attributes
- `find_input_port_at()`, `is_port()`, `is_trigger_button()`, `get_node_id_from_event()` helpers
- `NODE_WIDTH` in `state.rs` must stay in sync with CSS `.graph-node { width: 160px }`. Changing one without the other breaks port positions.

### Port Type Validation

- Connections validate port type compatibility before allowing (port types: Trigger, Data, Control)
- `ConnectionState` stores `source_port_name` and `target_port_name` for precise wire endpoints

### Node Port Layout

- Dynamic port positioning: ports stack vertically per side based on port index
- Port colors via CSS variables (`--port-trigger-color`, `--port-data-color`, `--port-control-color`)
- Output ports: right side; Input ports: left side

### Execution Engine

- `src/components/execution_engine.rs` - Task execution, timestamps, trace logging
- `src/components/execution_trace.rs` - Right panel display of execution traces
- `Task::new()`, `TaskStatus::Running/Complete`, `TraceEntry` for logging

### Styling
- CSS custom properties for theming (`--bg-primary`, `--accent-color`, etc.)
- Dark/light mode via `prefers-color-scheme`
- Panel variables: `--panel-width-left`, `--panel-width-right`, `--divider-width`

## Adding New Node Types

1. Add to `NODE_TYPES` const in `left_panel.rs` with `id`, `name`, `category`, `description`
2. Add rendering logic in `canvas.rs` (future: node type registry)
3. Add configuration UI in a new inspector panel (planned)

## Extensibility Points

- **New panels**: Add to `AppLayout` view in `app_layout.rs`
- **New node components**: Create in `src/components/nodes/`
- **Wire routing**: Modify `draw_connections()` and `draw_bezier()` in canvas.rs
