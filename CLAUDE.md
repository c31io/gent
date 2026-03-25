# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Gent is a visual node editor for context engineering and agent orchestration — a mix of Node-RED, Unreal Engine Blueprints, and LangChain purpose-built for building reasoning workflows. The UI is a Figma-like three-panel layout with a canvas in the center.

## Build Commands

- **Dev server**: `trunk serve` (runs on http://localhost:1420)
- **Production build**: `trunk build` (outputs to `dist/`)
- **Tauri build**: `cargo tauri build`
- **Check**: `cargo check`

## Architecture

### Three-Panel Layout
- **Left Panel** (`src/components/left_panel.rs`): Node palette with 13 predefined node types across 6 categories (Input, Context, Agent, Tool, Control, Output)
- **Canvas** (`src/components/canvas.rs`): DOM-based node rendering with pan/zoom via CSS transforms
- **Right Panel** (`src/components/right_panel.rs`): Execution trace display

### Core Stack
- **Leptos 0.7** with CSR (client-side rendering) via `wasm-bindgen`
- **web-sys** for DOM event handling (mouse events for pan/resize)
- **Tauri** for desktop shell

### Key Components
- `AppLayout` (`app_layout.rs`): Manages panel sizes via Leptos signals, handles divider drag-to-resize
- `Canvas` (`canvas.rs`): Renders `GraphNode` components, handles pan (mouse drag) and zoom (scroll wheel)
- `GraphNode` (`nodes/node.rs`): Individual DOM-based nodes with input/output ports
- `Connection` (`nodes/connection.rs`): SVG bezier curve wires between nodes

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
- **Connection logic**: Modify `Connection` component for different wire routing
