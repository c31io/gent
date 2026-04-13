# AGENTS.md

This file provides essential context for AI coding agents working on the Gent codebase.

## Project Overview

**Gent** is a visual node editor for context engineering and agent orchestration — a mix of Node-RED, Unreal Engine Blueprints, and LangChain purpose-built for building reasoning workflows. The UI is a Figma-like three-panel layout with a canvas in the center.

- **Frontend**: Leptos 0.8 (CSR, compiled to WASM via `wasm-bindgen`)
- **Desktop shell**: Tauri v2
- **Build tool**: Trunk
- **Backend runtime**: Rust (Tokio, Reqwest)
- **Plugin runtime**: Wasmtime (WASM) + Rune scripting engine
- **Styling**: Plain CSS with custom properties (dark/light mode via `prefers-color-scheme`)

## Project Structure

```
C:\Users\c31io\Documents\GitHub\gent
├── Cargo.toml              # Frontend package "gent-ui"
├── src/
│   ├── main.rs             # Entry: mounts Leptos App to body
│   ├── app.rs              # Root App component
│   ├── tauri_invoke.rs     # Helper to invoke Tauri commands from WASM
│   ├── components/
│   │   ├── app_layout.rs   # Three-panel layout + global state + keyboard shortcuts
│   │   ├── canvas/         # Canvas module: canvas.rs, geometry.rs, state.rs, wires.rs
│   │   ├── nodes/          # Node rendering: node.rs
│   │   ├── left_panel.rs   # Node palette (13 predefined node types)
│   │   ├── right_panel.rs  # Execution trace display
│   │   ├── inspector_panel.rs  # Bottom inspector panel
│   │   ├── execution_engine.rs # Task execution logic
│   │   ├── execution_trace.rs  # Trace display logic
│   │   ├── save_load.rs    # Persistence, clipboard, import/export
│   │   ├── toast.rs        # Toast notification system
│   │   ├── plugin_manager.rs   # Plugin management UI
│   │   ├── script_editor.rs    # Script editor UI
│   │   └── graph_section.rs    # Saved selections / bundled groups UI
│   └── gent-plugin/        # Plugin SDK crate for external authors
│       ├── Cargo.toml
│       └── src/lib.rs
├── src-tauri/              # Tauri backend
│   ├── Cargo.toml          # Backend package "gent"
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   └── src/
│       ├── lib.rs          # Tauri setup + command registrations
│       ├── main.rs         # Binary entry (calls gent_lib::run)
│       ├── llm.rs          # LLM completion (OpenAI, Anthropic)
│       ├── plugins/        # Plugin system (WASM loader, Rune loader, registry, etc.)
│       └── scripts/        # Rune scripting engine (commands + engine)
├── plugins/hello-world/    # Example plugin project
├── public/                 # Static assets (CodeMirror, bundled plugins, scripts)
├── styles.css              # Main stylesheet
├── index.html              # Trunk entry HTML
├── Trunk.toml              # Trunk configuration
└── docs/superpowers/       # Design docs, specs, and implementation plans
```

## Build and Test Commands

### Development
- **Dev server**: `trunk serve` (serves on http://localhost:1420)
- **Tauri dev**: `cargo tauri dev` (builds frontend via Trunk, then launches Tauri)

### Production / Desktop
- **Production web build**: `trunk build` (outputs to `dist/`)
- **Tauri build**: `cargo tauri build`

### Checking
- **Check frontend**: `cargo check` (run in project root — checks WASM/Leptos code)
- **Check backend**: `cd src-tauri && cargo check` (checks Tauri Rust code)

### Testing
- **There are currently no tests in the project.** If you add tests, use `cargo test` in the relevant crate directory.

## Code Style Guidelines

- **Language**: Rust (frontend + backend + plugins)
- **Formatting**: Use `rustfmt` / `cargo fmt`
- **Comments**: Doc comments (`///`) for public APIs; inline comments for tricky logic
- **Naming**: Follow standard Rust naming (`snake_case` for functions/variables, `PascalCase` for types/components)
- **Error handling**: Backend uses `anyhow`/`thiserror`; Tauri commands return `Result<T, String>` for frontend compatibility

## Key Architecture Conventions

### Leptos 0.8 Lifecycle
- **`on_mount` does not exist in Leptos 0.8.** For deferred initialization, call `spawn_local` at the top level of a component — it runs when the component is instantiated.
- For cleanup, use `leptos::prelude::on_cleanup`.

### WASM Compatibility
- `std::time::Instant` does not work in WASM. Use `js_sys::Date::now()` for timestamps.
- The `Timestamp` struct in `execution_engine.rs` wraps this pattern.

### Tauri Browser/Desktop Detection
- Check `__TAURI__` via `web_sys::window()` and test with `is_undefined()` before invoking Tauri APIs.
- Return user-friendly errors (e.g., "Only available in Tauri desktop app") instead of cryptic JS TypeErrors.
- See `tauri_invoke.rs` and `plugin_manager.rs` for the pattern.

### Canvas & Node System
- **Three-panel layout**: Left (node palette), Center (canvas), Right (execution trace). Bottom inspector panel is collapsible.
- **DOM-based rendering**: Nodes are DOM elements; wires are drawn on a `<canvas>` overlay.
- **Node width**: `NODE_WIDTH` in `state.rs` (160px) must stay in sync with CSS `.graph-node { width: 160px }`. Changing one without the other breaks port positions.
- **Port events**: Do **not** call `stop_propagation()` on port events — let them bubble to the canvas for reliable handling.
- **Click vs drag**: A 5px movement threshold (`dx < 5.0 && dy < 5.0`) distinguishes clicks from drags in `node.rs`.
- **Port type validation**: Connections validate port type compatibility before allowing (types: `Trigger`, `Text`, `Image`, `Audio`, `File`, `Embeddings`).
- **Multi-select**: Selection is a `HashSet<u32>` of node IDs (not `Option<u32>`).

### Keyboard Shortcuts (Global)
Implemented in `app_layout.rs` via a window `keydown` listener:
- `Ctrl+C` — Copy selection to clipboard
- `Ctrl+V` — Paste from clipboard
- `Ctrl+S` — Save selection to localStorage
- `Ctrl+E` — Export selection to file
- `Ctrl+I` — Import from file
- `Ctrl+A` — Select all nodes
- `Ctrl+Z` — Undo last change
- `Ctrl+Shift+Z` — Redo last undone change
- `Delete` / `Backspace` — Delete selected nodes
- `Escape` — Clear selection

Shortcuts are ignored when focus is in a text input.

### Undo/Redo System
- **Snapshot-based history**: A reactive `Effect` in `app_layout.rs` observes all undoable signals (`nodes`, `connections`, `selected_node_ids`, `next_node_id`, `next_connection_id`). When they change, the previous state is pushed onto an `UndoManager` stack (capped at 50 entries).
- **Scope**: Undo covers graph content and selection. It does **not** cover view state (pan/zoom), execution trace, or panel sizes.
- **Implementation**: `src/components/undo.rs` defines `GraphSnapshot` and `UndoManager`. `StoredValue<bool>` is used to suppress snapshot pushes during undo/redo restoration.

### Plugin System
- **WASM plugins**: Loaded via `wasmtime` + `wasmtime-wasi` (see `src-tauri/src/plugins/wasm_loader.rs`)
- **Rune scripts**: `.rn` files executed by the embedded Rune engine (see `src-tauri/src/scripts/`)
- **Plugin SDK**: `src/gent-plugin/` defines `Manifest`, `Input`, `Output`, `Capability`
- **Example plugin**: `plugins/hello-world/` demonstrates building a plugin for Gent

### LLM Integration
- Supports OpenAI and Anthropic APIs.
- API keys can be provided per-node or fall back to environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`).
- Custom base URLs are supported for OpenAI-compatible endpoints.

## Security Considerations

- **Tauri CSP is currently `null`** — no Content Security Policy is configured.
- **`execute_code` Tauri command runs arbitrary shell code** (`cmd /C` on Windows, `sh -c` elsewhere). Treat this as a high-privilege operation.
- **Script ID validation**: Only alphanumeric ASCII, `-`, and `_` are allowed in script IDs to prevent path traversal.
- **Bundled scripts cannot be overwritten** via the `save_script` command.

## Adding New Node Types

1. Add the node definition to `NODE_TYPES` in `src/components/left_panel.rs` (`id`, `name`, `category`, `description`).
2. Add default ports in `src/components/canvas/state.rs` (`default_ports_for_type`).
3. Add default variant in `default_variant_for_type` (same file).
4. Add execution logic in `src/components/execution_engine.rs` if the node needs runtime behavior.
5. Add styling/CSS if the node needs a custom appearance.

## Extensibility Points

- **New panels**: Add to `AppLayout` view in `app_layout.rs`
- **New node components**: Create in `src/components/nodes/`
- **Wire routing**: Modify `draw_connections()` and `draw_bezier()` in `canvas.rs`
- **New Tauri commands**: Register in `src-tauri/src/lib.rs` and expose via `tauri_invoke.rs` on the frontend if needed

## Useful Docs in `docs/superpowers/`

The project keeps design documents and implementation plans under `docs/superpowers/`:
- `specs/` — Design specs (e.g., execution engine, plugin system, scripting engine)
- `plans/` — Dated implementation plans (e.g., LLM node, multi-select, plugin console API)
