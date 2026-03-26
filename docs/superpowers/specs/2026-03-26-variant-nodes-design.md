# Variant Nodes Design

## Overview

Add a generalized node abstraction allowing heterogeneous node types with visual variants, typed ports, and instance-level port overrides.

## Current State

- 13 predefined node types in `left_panel.rs` via `NODE_TYPES` const
- Uniform `GraphNode` component renders all nodes identically (label + "Node content" or trigger button)
- `NodeState` holds: id, x, y, node_type (String), label, selected, status

## Goals

1. Per-type visual body content (text fields, spinners, etc.)
2. Typed ports with color indicators
3. Instance-level port overrides for specific node types
4. Editable node properties via bottom drawer inspector

## Data Structures

### PortType Enum

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum PortType {
    Text,      // blue
    Image,     // green
    Audio,     // orange
    File,      // gray
    Embeddings,// purple
    Trigger,   // red
}
```

### Port Struct

```rust
#[derive(Clone, Debug)]
pub struct Port {
    pub name: String,
    pub port_type: PortType,
    pub direction: PortDirection, // In or Out
}
```

### NodeVariant Enum

```rust
#[derive(Clone, Debug)]
pub enum NodeVariant {
    UserInput { text: String },
    FileInput { path: String },
    Trigger,
    Template { template: String },
    Retrieval { query: String },
    Summarizer { max_length: u32 },
    PlannerAgent { goal: String },
    ExecutorAgent { task: String },
    WebSearch { query: String, num_results: u32 },
    CodeExecute { code: String, language: String },
    IfCondition { branches: u32 },       // instance-level port count
    Loop { iterations: u32 },            // instance-level port count
    ChatOutput { response: String },
    JsonOutput { schema: String },
}
```

### Updated NodeState

```rust
#[derive(Clone, Debug)]
pub struct NodeState {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub node_type: String,
    pub label: String,
    pub selected: bool,
    pub status: NodeStatus,
    pub variant: NodeVariant,
    pub ports: Vec<Port>, // instance-level overrides
}
```

### Default Port Configurations

Each `NodeVariant` defines default ports. For nodes with instance-level port overrides (IfCondition, Loop), the port list is mutable via inspector. For others, ports are fixed per type.

| Node Type | Inputs | Outputs |
|-----------|--------|---------|
| UserInput | — | [("output", Text)] |
| FileInput | — | [("output", File)] |
| Trigger | — | [("output", Trigger)] |
| Template | [("input", Text)] | [("output", Text)] |
| Retrieval | [("query", Text)] | [("result", Embeddings)] |
| Summarizer | [("input", Text)] | [("output", Text)] |
| PlannerAgent | [("goal", Text), ("context", Text)] | [("plan", Text), ("next", Trigger)] |
| ExecutorAgent | [("task", Text), ("context", Text)] | [("result", Text), ("done", Trigger)] |
| WebSearch | [("query", Text)] | [("results", Text)] |
| CodeExecute | [("code", Text)] | [("output", Text), ("error", Text)] |
| IfCondition | [("condition", Text)] | Dynamic (branches count) |
| Loop | [("input", Text)] | [("iteration", Text), ("done", Trigger)] |
| ChatOutput | [("response", Text)] | — |
| JsonOutput | [("data", Text)] | — |

## Port Colors (CSS Variables)

```css
--port-text: #3b82f6;      /* blue */
--port-image: #22c55e;     /* green */
--port-audio: #f97316;     /* orange */
--port-file: #6b7280;      /* gray */
--port-embeddings: #a855f7; /* purple */
--port-trigger: #ef4444;   /* red */
```

## Inspector (Bottom Drawer)

- Single drawer spanning full width, slides up from bottom
- Opens when a node is selected
- Shows node label (editable) and variant-specific properties
- For nodes with instance-level port overrides (IfCondition, Loop): shows input count spinner
- Port list displays all ports with colored type indicators and editable names
- "Delete Node" button at bottom

### Inspector Sections

1. **Header**: Node type label + close button
2. **Properties**: Variant-specific fields (text inputs, number spinners)
3. **Ports**: Scrollable list of input/output ports
4. **Actions**: Delete node button

## GraphNode Rendering

### Input Ports
- Rendered at left edge of node body
- Colored circle indicator based on `Port.port_type`
- Port name label beside circle
- Click to disconnect existing wire

### Output Ports
- Rendered at right edge of node body
- Same colored indicator as inputs
- Drag from port to create wire connection

### Node Body
- Rendered via match on `NodeVariant`
- Each variant has its own view expression
- Edits write directly to `WriteSignal<NodeVariant>` in `NodeState`

### Variant-Specific Body Views

```rust
match variant {
    NodeVariant::UserInput { text } => text_input_component(text),
    NodeVariant::Template { template } => textarea_component(template),
    NodeVariant::IfCondition { branches } => branch_count_spinner(branches),
    // ...
}
```

## Connection Validation

- Ports with matching `PortType` can connect
- Exception: Trigger ports only connect to Trigger inputs
- Visual feedback: invalid drop targets dim during drag

## Implementation Phases

1. Add `PortType`, `Port`, `PortDirection` types
2. Update `NodeState` with `variant: NodeVariant` and `ports: Vec<Port>`
3. Add default port configs per variant
4. Update `GraphNode` to render ports from `ports` list with color indicators
5. Add variant-specific body views in `GraphNode`
6. Create bottom drawer inspector component
7. Wire inspector to selected node's `WriteSignal`
8. Add connection validation based on `PortType`
9. Add port override support for IfCondition/Loop

## Files to Modify

- `src/components/nodes/node.rs` — GraphNode with variant rendering
- `src/components/canvas/state.rs` — NodeState, NodeVariant, PortType, Port
- `src/components/left_panel.rs` — NODE_TYPES gets default port configs
- `src/components/canvas/canvas.rs` — Connection validation
- `src/components/inspector.rs` (new) — Bottom drawer inspector
- `src/app_layout.rs` — Add inspector to layout
- CSS variables for port colors

## Port Color Implementation

```css
.port-text { color: var(--port-text); }
.port-image { color: var(--port-image); }
.port-audio { color: var(--port-audio); }
.port-file { color: var(--port-file); }
.port-embeddings { color: var(--port-embeddings); }
.port-trigger { color: var(--port-trigger); }
```

Port indicator is a small circle (8px) with background-color set via CSS variable matching port type.
