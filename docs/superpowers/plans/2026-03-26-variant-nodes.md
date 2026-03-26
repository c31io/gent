# Variant Nodes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generalized node abstraction with typed ports, visual variants, and instance-level port overrides via a bottom drawer inspector.

**Architecture:** Enum-based node variants (Rust discriminated unions) with typed ports. Most nodes have fixed ports; only IfCondition and Loop allow dynamic port count changes. Bottom drawer inspector for editing node properties.

**Tech Stack:** Leptos 0.7, wasm-bindgen, web-sys, Tauri

---

## File Structure

```
src/components/
  canvas/
    state.rs        — Add PortType, Port, PortDirection, NodeVariant; update NodeState
  nodes/
    node.rs          — Update GraphNode to render typed ports with colors + variant body content
  inspector.rs      — NEW: Bottom drawer inspector component
  left_panel.rs     — Add default port configs per node type
src/
  app_layout.rs     — Wire inspector signals and add to layout
styles.css          — Add port color CSS variables
```

---

## Task 1: Add Typed Port Data Structures to state.rs

**Files:**
- Modify: `src/components/canvas/state.rs:1-41`

- [ ] **Step 1: Add PortType, PortDirection, Port, and NodeVariant enums before NodeState**

```rust
/// Direction for a port
#[derive(Clone, Debug, PartialEq)]
pub enum PortDirection {
    In,
    Out,
}

/// Type of data flowing through a port
#[derive(Clone, Debug, PartialEq)]
pub enum PortType {
    Text,       // blue #3b82f6
    Image,      // green #22c55e
    Audio,      // orange #f97316
    File,       // gray #6b7280
    Embeddings, // purple #a855f7
    Trigger,    // red #ef4444
}

/// A port on a node
#[derive(Clone, Debug)]
pub struct Port {
    pub name: String,
    pub port_type: PortType,
    pub direction: PortDirection,
}

/// Variants for different node types with their specific data
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
    IfCondition { branches: u32 },
    Loop { iterations: u32 },
    ChatOutput { response: String },
    JsonOutput { schema: String },
}
```

- [ ] **Step 2: Add default ports function for each node type**

```rust
/// Returns default ports for a given node_type string
pub fn default_ports_for_type(node_type: &str) -> Vec<Port> {
    match node_type {
        "user_input" => vec![Port { name: "output".into(), port_type: PortType::Text, direction: PortDirection::Out }],
        "file_input" => vec![Port { name: "output".into(), port_type: PortType::File, direction: PortDirection::Out }],
        "trigger" => vec![Port { name: "output".into(), port_type: PortType::Trigger, direction: PortDirection::Out }],
        "template" => vec![
            Port { name: "input".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "output".into(), port_type: PortType::Text, direction: PortDirection::Out },
        ],
        "retrieval" => vec![
            Port { name: "query".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "result".into(), port_type: PortType::Embeddings, direction: PortDirection::Out },
        ],
        "summarizer" => vec![
            Port { name: "input".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "output".into(), port_type: PortType::Text, direction: PortDirection::Out },
        ],
        "planner_agent" => vec![
            Port { name: "goal".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "context".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "plan".into(), port_type: PortType::Text, direction: PortDirection::Out },
            Port { name: "next".into(), port_type: PortType::Trigger, direction: PortDirection::Out },
        ],
        "executor_agent" => vec![
            Port { name: "task".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "context".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "result".into(), port_type: PortType::Text, direction: PortDirection::Out },
            Port { name: "done".into(), port_type: PortType::Trigger, direction: PortDirection::Out },
        ],
        "web_search" => vec![
            Port { name: "query".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "results".into(), port_type: PortType::Text, direction: PortDirection::Out },
        ],
        "code_execute" => vec![
            Port { name: "code".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "output".into(), port_type: PortType::Text, direction: PortDirection::Out },
            Port { name: "error".into(), port_type: PortType::Text, direction: PortDirection::Out },
        ],
        "if_condition" => vec![
            Port { name: "condition".into(), port_type: PortType::Text, direction: PortDirection::In },
            // Outputs added dynamically based on branches count
        ],
        "loop" => vec![
            Port { name: "input".into(), port_type: PortType::Text, direction: PortDirection::In },
            Port { name: "iteration".into(), port_type: PortType::Text, direction: PortDirection::Out },
            Port { name: "done".into(), port_type: PortType::Trigger, direction: PortDirection::Out },
        ],
        "chat_output" => vec![Port { name: "response".into(), port_type: PortType::Text, direction: PortDirection::In }],
        "json_output" => vec![Port { name: "data".into(), port_type: PortType::Text, direction: PortDirection::In }],
        _ => vec![],
    }
}
```

- [ ] **Step 3: Add default variant function**

```rust
/// Returns default NodeVariant for a given node_type string
pub fn default_variant_for_type(node_type: &str) -> NodeVariant {
    match node_type {
        "user_input" => NodeVariant::UserInput { text: String::new() },
        "file_input" => NodeVariant::FileInput { path: String::new() },
        "trigger" => NodeVariant::Trigger,
        "template" => NodeVariant::Template { template: String::new() },
        "retrieval" => NodeVariant::Retrieval { query: String::new() },
        "summarizer" => NodeVariant::Summarizer { max_length: 500 },
        "planner_agent" => NodeVariant::PlannerAgent { goal: String::new() },
        "executor_agent" => NodeVariant::ExecutorAgent { task: String::new() },
        "web_search" => NodeVariant::WebSearch { query: String::new(), num_results: 5 },
        "code_execute" => NodeVariant::CodeExecute { code: String::new(), language: "python".into() },
        "if_condition" => NodeVariant::IfCondition { branches: 2 },
        "loop" => NodeVariant::Loop { iterations: 3 },
        "chat_output" => NodeVariant::ChatOutput { response: String::new() },
        "json_output" => NodeVariant::JsonOutput { schema: String::new() },
        _ => NodeVariant::Trigger,
    }
}
```

- [ ] **Step 4: Update NodeState struct**

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
    pub ports: Vec<Port>,
}
```

- [ ] **Step 5: Run cargo check to verify**

Run: `cargo check`
Expected: PASS (errors if syntax issues)

- [ ] **Step 6: Commit**

```bash
git add src/components/canvas/state.rs
git commit -m "feat: add typed port data structures (PortType, Port, NodeVariant)"
```

---

## Task 2: Add Port Color CSS Variables

**Files:**
- Modify: `styles.css:1-614`

- [ ] **Step 1: Add port color CSS variables to :root**

Add after existing color variables (around line 24):

```css
/* Port Colors */
--port-text: #3b82f6;       /* blue */
--port-image: #22c55e;      /* green */
--port-audio: #f97316;       /* orange */
--port-file: #6b7280;        /* gray */
--port-embeddings: #a855f7;  /* purple */
--port-trigger: #ef4444;     /* red */
```

- [ ] **Step 2: Add port color utility classes**

Add at end of styles.css:

```css
/* Port color indicators */
.port-indicator {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
}

.port-indicator.text { background-color: var(--port-text); }
.port-indicator.image { background-color: var(--port-image); }
.port-indicator.audio { background-color: var(--port-audio); }
.port-indicator.file { background-color: var(--port-file); }
.port-indicator.embeddings { background-color: var(--port-embeddings); }
.port-indicator.trigger { background-color: var(--port-trigger); }

/* Port type indicator dots on node ports */
.node-port.text { border-color: var(--port-text); }
.node-port.image { border-color: var(--port-image); }
.node-port.audio { border-color: var(--port-audio); }
.node-port.file { border-color: var(--port-file); }
.node-port.embeddings { border-color: var(--port-embeddings); }
.node-port.trigger { border-color: var(--port-trigger); }
```

- [ ] **Step 3: Run trunk serve to verify CSS loads**

Run: `trunk serve` (in background)
Expected: No CSS errors in console

- [ ] **Step 4: Commit**

```bash
git add styles.css
git commit -m "feat: add port color CSS variables"
```

---

## Task 3: Update GraphNode to Render Typed Ports

**Files:**
- Modify: `src/components/nodes/node.rs:1-141`

- [ ] **Step 1: Add imports for Port and PortType**

Add after existing imports:

```rust
use crate::components::canvas::state::{Port, PortDirection, PortType};
```

- [ ] **Step 2: Update GraphNode props to accept ports and variant**

Replace the component signature with:

```rust
#[component]
pub fn GraphNode(
    x: f64,
    y: f64,
    label: String,
    selected: bool,
    node_id: u32,
    variant: NodeVariant,
    ports: Vec<Port>,
    has_input_connection: bool,
    #[prop(default = false)] is_deleting: bool,
    on_output_drag_start: Option<Callback<(u32, f64, f64)>>,
    on_input_drag_end: Option<Callback<(u32, f64, f64)>>,
    on_input_click: Option<Callback<(u32,)>>,
    on_input_reroute_start: Option<Callback<(u32,)>>,
    cancel_connection_drag: Option<Callback<(), ()>>,
    on_trigger: Option<Callback<u32>>,
    on_variant_change: Option<Callback<NodeVariant>>,
) -> impl IntoView {
```

- [ ] **Step 3: Add helper to get port CSS class**

Add before the `view!` macro:

```rust
let port_class = |port_type: &PortType| -> String {
    match port_type {
        PortType::Text => "node-port text".into(),
        PortType::Image => "node-port image".into(),
        PortType::Audio => "node-port audio".into(),
        PortType::File => "node-port file".into(),
        PortType::Embeddings => "node-port embeddings".into(),
        PortType::Trigger => "node-port trigger".into(),
    }
};
```

- [ ] **Step 4: Add variant-specific body content rendering**

Add helper function before component:

```rust
fn render_variant_body(variant: &NodeVariant) -> View {
    match variant {
        NodeVariant::UserInput { text } => view! {
            <input
                type="text"
                class="node-variant-input"
                value={text.clone()}
                placeholder="Enter text..."
            />
        }.into_any(),
        NodeVariant::FileInput { path } => view! {
            <input
                type="text"
                class="node-variant-input"
                value={path.clone()}
                placeholder="File path..."
            />
        }.into_any(),
        NodeVariant::Trigger => view! {
            <button
                class="trigger-btn"
                on:mousedown={move |ev| {
                    ev.prevent_default();
                }}
            >
                "Run"
            </button>
        }.into_any(),
        NodeVariant::Template { template } => view! {
            <textarea
                class="node-variant-textarea"
                value={template.clone()}
                placeholder="Template..."
                rows="3"
            />
        }.into_any(),
        NodeVariant::Retrieval { query } => view! {
            <input
                type="text"
                class="node-variant-input"
                value={query.clone()}
                placeholder="Search query..."
            />
        }.into_any(),
        NodeVariant::Summarizer { max_length } => view! {
            <div class="node-variant-field">
                <label>"Max Length"</label>
                <input
                    type="number"
                    class="node-variant-input"
                    value={*max_length as f64}
                    min="50"
                    max="2000"
                />
            </div>
        }.into_any(),
        NodeVariant::PlannerAgent { goal } => view! {
            <textarea
                class="node-variant-textarea"
                value={goal.clone()}
                placeholder="Agent goal..."
                rows="2"
            />
        }.into_any(),
        NodeVariant::ExecutorAgent { task } => view! {
            <textarea
                class="node-variant-textarea"
                value={task.clone()}
                placeholder="Task description..."
                rows="2"
            />
        }.into_any(),
        NodeVariant::WebSearch { query, num_results } => view! {
            <div class="node-variant-fields">
                <input
                    type="text"
                    class="node-variant-input"
                    value={query.clone()}
                    placeholder="Search query..."
                />
                <div class="node-variant-field">
                    <label>"Results"</label>
                    <input
                        type="number"
                        class="node-variant-input small"
                        value={*num_results as f64}
                        min="1"
                        max="20"
                    />
                </div>
            </div>
        }.into_any(),
        NodeVariant::CodeExecute { code, language } => view! {
            <div class="node-variant-fields">
                <textarea
                    class="node-variant-textarea code"
                    value={code.clone()}
                    placeholder="Code..."
                    rows="2"
                />
                <input
                    type="text"
                    class="node-variant-input small"
                    value={language.clone()}
                    placeholder="Language..."
                />
            </div>
        }.into_any(),
        NodeVariant::IfCondition { branches } => view! {
            <div class="node-variant-field">
                <label>"Branches"</label>
                <input
                    type="number"
                    class="node-variant-input"
                    value={*branches as f64}
                    min="2"
                    max="10"
                />
            </div>
        }.into_any(),
        NodeVariant::Loop { iterations } => view! {
            <div class="node-variant-field">
                <label>"Iterations"</label>
                <input
                    type="number"
                    class="node-variant-input"
                    value={*iterations as f64}
                    min="1"
                    max="100"
                />
            </div>
        }.into_any(),
        NodeVariant::ChatOutput { response } => view! {
            <textarea
                class="node-variant-textarea"
                value={response.clone()}
                placeholder="Response..."
                rows="2"
            />
        }.into_any(),
        NodeVariant::JsonOutput { schema } => view! {
            <textarea
                class="node-variant-textarea code"
                value={schema.clone()}
                placeholder="JSON Schema..."
                rows="2"
            />
        }.into_any(),
    }
}
```

- [ ] **Step 5: Add node variant CSS classes to styles.css**

Add to styles.css:

```css
/* Node variant body elements */
.node-variant-input {
    width: 100%;
    padding: 6px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-primary);
    outline: none;
}

.node-variant-input:focus {
    border-color: var(--accent-color);
}

.node-variant-textarea {
    width: 100%;
    padding: 6px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-primary);
    outline: none;
    resize: vertical;
    font-family: inherit;
}

.node-variant-textarea:focus {
    border-color: var(--accent-color);
}

.node-variant-textarea.code {
    font-family: monospace;
}

.node-variant-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.node-variant-field label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
}

.node-variant-input.small {
    width: 80px;
}

.node-variant-fields {
    display: flex;
    flex-direction: column;
    gap: 8px;
}
```

- [ ] **Step 6: Update GraphNode view to render ports dynamically**

Replace the single input/output port divs with port iteration. The node body should render `render_variant_body(&variant)`. The node should show:
- Input ports on the left (vertical stack if multiple)
- Output ports on the right (vertical stack if multiple)

```rust
// In view! block, replace static ports with:
{ports.iter().filter(|p| p.direction == PortDirection::In).map(|port| {
    view! {
        <div
            class={port_class(&port.port_type)}
            data-port="input"
            data-node-id={node_id}
            data-port-name={port.name.clone()}
            title={port.name.clone()}
            on:mousedown=handle_input_mousedown
            on:mouseup=handle_input_mouseup
        >
            <span class="port-label">{port.name.clone()}</span>
        </div>
    }
}).collect::<Vec<_>>()}

{ports.iter().filter(|p| p.direction == PortDirection::Out).map(|port| {
    view! {
        <div
            class={port_class(&port.port_type)}
            data-port="output"
            data-node-id={node_id}
            data-port-name={port.name.clone()}
            title={port.name.clone()}
            on:mousedown=handle_output_mousedown
        >
            <span class="port-label">{port.name.clone()}</span>
        </div>
    }
}).collect::<Vec<_>>()}
```

Add port label CSS:
```css
.port-label {
    position: absolute;
    font-size: 10px;
    color: var(--text-secondary);
    white-space: nowrap;
}

.node-port.input .port-label {
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
}

.node-port.output .port-label {
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
}
```

- [ ] **Step 7: Update node body to use variant**

Replace `{if is_trigger {...}}` section with `{render_variant_body(&variant)}`

- [ ] **Step 8: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add src/components/nodes/node.rs styles.css
git commit -m "feat: update GraphNode to render typed ports and variant body"
```

---

## Task 4: Update Canvas to Use New NodeState with Ports/Variant

**Files:**
- Modify: `src/components/canvas/canvas.rs:1-546`

- [ ] **Step 1: Update imports to include new types**

```rust
use crate::components::canvas::state::{ConnectionState, DraggingConnection, NodeState, NodeVariant, Port, PortDirection, PortType, default_ports_for_type, default_variant_for_type};
```

- [ ] **Step 2: Update node rendering in view to pass new props**

Replace the GraphNode invocation with:

```rust
<GraphNode
    x={node.x}
    y={node.y}
    label={node.label.clone()}
    selected={is_selected}
    node_id={node.id}
    variant={node.variant.clone()}
    ports={node.ports.clone()}
    has_input_connection={has_connection}
    is_deleting={is_deleting}
    on_output_drag_start={Some(Callback::from(handle_output_drag_start))}
    on_input_drag_end={Some(Callback::from(handle_input_drag_end))}
    on_input_click={Some(handle_input_click)}
    on_input_reroute_start={Some(Callback::from(handle_input_reroute_start))}
    cancel_connection_drag={Some(cancel_connection_drag)}
    on_trigger={on_trigger}
/>
```

- [ ] **Step 3: Update `on_node_drop` callback to create nodes with ports and variant**

The `on_node_drop` callback is passed into Canvas from app_layout. When a palette item is dropped, Canvas invokes the callback with `(node_type, canvas_x, canvas_y)`. The parent (app_layout) receives this and creates a new `NodeState` with proper `variant` and `ports`. This is implemented in Task 6 - no changes needed in canvas.rs for node creation.

- [ ] **Step 4: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/canvas/canvas.rs
git commit -m "feat: update canvas to pass ports and variant to GraphNode"
```

---

## Task 5: Create Bottom Drawer Inspector Component

**Files:**
- Create: `src/components/inspector.rs`

- [ ] **Step 1: Create inspector.rs with basic structure**

```rust
use leptos::prelude::*;

#[component]
pub fn NodeInspector(
    /// The selected node (None = inspector hidden)
    selected_node: Option<NodeState>,
    /// Callback when a node property changes
    on_node_update: Option<Callback<NodeState>>,
    /// Callback when delete is clicked
    on_node_delete: Option<Callback<u32>>,
    /// Callback when close is clicked
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    let is_visible = move || selected_node.is_some();

    view! {
        <div class={format!("node-inspector {}", if is_visible() { "visible" } else { "" })}>
            {move || {
                if let Some(node) = selected_node() {
                    view! {
                        <div class="inspector-content">
                            <div class="inspector-header">
                                <div class="inspector-node-info">
                                    <span class="node-type-badge">{node.node_type.clone()}</span>
                                    <span class="node-label">{node.label.clone()}</span>
                                </div>
                                <div class="inspector-actions">
                                    <button
                                        class="delete-btn"
                                        on:click={move |_| {
                                            if let Some(cb) = &on_node_delete {
                                                cb.run(node.id);
                                            }
                                        }}
                                        title="Delete node"
                                    >
                                        "🗑"
                                    </button>
                                    <button
                                        class="close-btn"
                                        on:click={move |_| {
                                            if let Some(cb) = &on_close {
                                                cb.run(());
                                            }
                                        }}
                                        title="Close"
                                    >
                                        "✕"
                                    </button>
                                </div>
                            </div>
                            <div class="inspector-body">
                                <InspectorProperties node={node} />
                            </div>
                        </div>
                    }
                } else {
                    view! { <></> }
                }
            }}
        </div>
    }
}

#[component]
pub fn InspectorProperties(
    node: NodeState,
) -> impl IntoView {
    // Render variant-specific property editors
    match node.variant {
        NodeVariant::UserInput { text } => view! {
            <div class="property-group">
                <label class="property-label">"Text"</label>
                <textarea
                    class="property-textarea"
                    value={text}
                    rows="3"
                />
            </div>
        }.into_any(),
        NodeVariant::FileInput { path } => view! {
            <div class="property-group">
                <label class="property-label">"File Path"</label>
                <input type="text" class="property-input" value={path} />
            </div>
        }.into_any(),
        NodeVariant::Trigger => view! {
            <div class="property-group">
                <span class="property-readonly">"Trigger nodes start execution"</span>
            </div>
        }.into_any(),
        NodeVariant::Template { template } => view! {
            <div class="property-group">
                <label class="property-label">"Template"</label>
                <textarea class="property-textarea" value={template} rows="4" />
            </div>
        }.into_any(),
        NodeVariant::Retrieval { query } => view! {
            <div class="property-group">
                <label class="property-label">"Query"</label>
                <input type="text" class="property-input" value={query} />
            </div>
        }.into_any(),
        NodeVariant::Summarizer { max_length } => view! {
            <div class="property-group">
                <label class="property-label">"Max Length"</label>
                <input type="number" class="property-input" value={*max_length as f64} min="50" max="2000" />
            </div>
        }.into_any(),
        NodeVariant::PlannerAgent { goal } => view! {
            <div class="property-group">
                <label class="property-label">"Goal"</label>
                <textarea class="property-textarea" value={goal} rows="2" />
            </div>
        }.into_any(),
        NodeVariant::ExecutorAgent { task } => view! {
            <div class="property-group">
                <label class="property-label">"Task"</label>
                <textarea class="property-textarea" value={task} rows="2" />
            </div>
        }.into_any(),
        NodeVariant::WebSearch { query, num_results } => view! {
            <div class="property-groups">
                <div class="property-group">
                    <label class="property-label">"Query"</label>
                    <input type="text" class="property-input" value={query} />
                </div>
                <div class="property-group">
                    <label class="property-label">"Number of Results"</label>
                    <input type="number" class="property-input" value={*num_results as f64} min="1" max="20" />
                </div>
            </div>
        }.into_any(),
        NodeVariant::CodeExecute { code, language } => view! {
            <div class="property-groups">
                <div class="property-group">
                    <label class="property-label">"Language"</label>
                    <input type="text" class="property-input" value={language} />
                </div>
                <div class="property-group">
                    <label class="property-label">"Code"</label>
                    <textarea class="property-textarea code" value={code} rows="4" />
                </div>
            </div>
        }.into_any(),
        NodeVariant::IfCondition { branches } => view! {
            <div class="property-group">
                <label class="property-label">"Branches"</label>
                <input type="number" class="property-input" value={*branches as f64} min="2" max="10" />
            </div>
        }.into_any(),
        NodeVariant::Loop { iterations } => view! {
            <div class="property-group">
                <label class="property-label">"Iterations"</label>
                <input type="number" class="property-input" value={*iterations as f64} min="1" max="100" />
            </div>
        }.into_any(),
        NodeVariant::ChatOutput { response } => view! {
            <div class="property-group">
                <label class="property-label">"Response"</label>
                <textarea class="property-textarea" value={response} rows="3" />
            </div>
        }.into_any(),
        NodeVariant::JsonOutput { schema } => view! {
            <div class="property-group">
                <label class="property-label">"JSON Schema"</label>
                <textarea class="property-textarea code" value={schema} rows="4" />
            </div>
        }.into_any(),
    }
}
```

- [ ] **Step 2: Add inspector CSS to styles.css**

```css
/* Inspector drawer styles */
.node-inspector {
    position: fixed;
    bottom: 0;
    left: var(--panel-width-left);
    right: var(--panel-width-right);
    height: 0;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    overflow: hidden;
    transition: height 0.2s ease;
    z-index: 100;
}

.node-inspector.visible {
    height: 200px;
}

.inspector-content {
    padding: 16px 20px;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.inspector-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.inspector-node-info {
    display: flex;
    align-items: center;
    gap: 12px;
}

.inspector-actions {
    display: flex;
    gap: 8px;
}

.close-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.15s;
}

.close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
}

/* Property groups */
.property-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.property-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
}

.property-input {
    padding: 8px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    font-size: 13px;
    color: var(--text-primary);
    outline: none;
}

.property-input:focus {
    border-color: var(--accent-color);
}

.property-textarea {
    padding: 8px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    font-size: 13px;
    color: var(--text-primary);
    outline: none;
    resize: vertical;
    font-family: inherit;
}

.property-textarea:focus {
    border-color: var(--accent-color);
}

.property-textarea.code {
    font-family: monospace;
}

.property-readonly {
    font-size: 13px;
    color: var(--text-secondary);
    font-style: italic;
}

.property-groups {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

/* Port list in inspector */
.port-list {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 0;
}

.port-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-size: 12px;
}

.port-item .port-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
}

.port-direction {
    font-size: 10px;
    color: var(--text-secondary);
    text-transform: uppercase;
}
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/components/inspector.rs styles.css
git commit -m "feat: add bottom drawer inspector component"
```

---

## Task 6: Wire Inspector to Selected Node's WriteSignal

**Files:**
- Modify: `src/app_layout.rs`

- [ ] **Step 1: Add signals for inspector state**

Add to app_layout:

```rust
let (inspector_node, set_inspector_node) = signal(Option::<NodeState>::None);
```

- [ ] **Step 2: Update Canvas to pass nodes with ports/variant**

Find where nodes are created and ensure they use the new structure:

```rust
// When creating a new node from palette drop
let new_node = NodeState {
    id: next_node_id.get(),
    x: canvas_x,
    y: canvas_y,
    node_type: node_type.clone(),
    label: node_type.replace("_", " ").replace(" ", "-").into(),
    selected: false,
    status: NodeStatus::Pending,
    variant: default_variant_for_type(&node_type),
    ports: default_ports_for_type(&node_type),
};
```

- [ ] **Step 3: Update AppLayout to include inspector**

Add to the view:

```rust
<NodeInspector
    selected_node={inspector_node}
    on_node_delete={Some(Callback::new(move |node_id| {
        set_nodes.update(|nodes| {
            nodes.retain(|n| n.id != node_id);
        });
        set_connections.update(|conns| {
            conns.retain(|c| c.source_node_id != node_id && c.target_node_id != node_id);
        });
        set_inspector_node.set(None);
    }))}
    on_close={Some(Callback::new(move |_| {
        set_inspector_node.set(None);
    }))}
/>
```

- [ ] **Step 4: Update selection to open inspector**

When a node is selected, populate the inspector:

```rust
on_selection_change={Some(Callback::new(move |node_id| {
    if let Some(id) = node_id {
        let nodes_snapshot = nodes.get();
        if let Some(node) = nodes_snapshot.iter().find(|n| n.id == id) {
            set_inspector_node.set(Some(node.clone()));
        }
    } else {
        set_inspector_node.set(None);
    }
}))}
```

- [ ] **Step 5: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/app_layout.rs
git commit -m "feat: wire inspector to selected node signals"
```

---

## Task 7: Add Connection Validation Based on PortType

**Files:**
- Modify: `src/components/canvas/canvas.rs`

- [ ] **Step 1: Add port type compatibility check**

Add helper function:

```rust
fn ports_compatible(source: &Port, target: &Port) -> bool {
    // Trigger ports only connect to other trigger ports
    if source.port_type == PortType::Trigger || target.port_type == PortType::Trigger {
        return source.port_type == target.port_type;
    }
    // All other port types can connect to each other
    true
}
```

- [ ] **Step 2: Update handle_input_drag_end to validate connection**

Modify the callback to check port compatibility before creating connection. Since nodes have multiple ports but we don't track which specific port was dragged from, we validate by checking if ANY output port of the source is compatible with ANY input port of the target:

```rust
let handle_input_drag_end = move |node_id: u32, _x: f64, _y: f64| {
    if let Some(dc) = dragging_connection.get() {
        if dc.source_node_id != node_id {
            // Get source and target nodes
            let source_node = nodes.get()
                .iter()
                .find(|n| n.id == dc.source_node_id);

            let target_node = nodes.get()
                .iter()
                .find(|n| n.id == node_id);

            // Validate port compatibility - at least one output must be compatible with one input
            let is_compatible = source_node.and_then(|s| {
                target_node.map(|t| {
                    s.ports.iter().filter(|p| p.direction == PortDirection::Out)
                        .any(|src_port| {
                            t.ports.iter().filter(|p| p.direction == PortDirection::In)
                                .any(|tgt_port| ports_compatible(src_port, tgt_port))
                        })
                })
            }).unwrap_or(false);

            if !is_compatible {
                // Invalid connection - cancel the drag
                set_dragging_connection.set(None);
                set_rerouting_from.set(None);
                return;
            }

            if let Some(src_input) = dc.source_input_node_id {
                set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn|
                    !(conn.source_node_id == dc.source_node_id && conn.target_node_id == src_input)
                ));
            }
            set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn| conn.target_node_id != node_id));
            let new_conn = ConnectionState {
                id: next_connection_id.get(),
                source_node_id: dc.source_node_id,
                target_node_id: node_id,
                selected: false,
            };
            set_connections.update(|c: &mut Vec<ConnectionState>| c.push(new_conn));
            set_next_connection_id.update(|n| *n += 1);
        }
    }
    set_dragging_connection.set(None);
    set_rerouting_from.set(None);
};
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/components/canvas/canvas.rs
git commit -m "feat: add port type connection validation"
```

---

## Task 8: Add Dynamic Port Support for IfCondition/Loop

**Files:**
- Modify: `src/components/canvas/state.rs`

- [ ] **Step 1: Add function to get output ports with dynamic count**

```rust
/// Returns output ports including dynamic ones based on variant state
pub fn get_output_ports(node_type: &str, variant: &NodeVariant) -> Vec<Port> {
    let mut ports = default_ports_for_type(node_type)
        .into_iter()
        .filter(|p| p.direction == PortDirection::Out)
        .collect::<Vec<_>>();

    // Add dynamic output ports for IfCondition based on branches
    if let NodeVariant::IfCondition { branches } = variant {
        for i in 0..*branches {
            ports.push(Port {
                name: format!("branch_{}", i + 1),
                port_type: PortType::Trigger,
                direction: PortDirection::Out,
            });
        }
    }

    ports
}
```

- [ ] **Step 2: Update canvas.rs to use dynamic output ports**

In `canvas.rs`, where GraphNode is rendered with ports, use `get_output_ports(node.node_type, &node.variant)` instead of `node.ports` when rendering output ports. Input ports still come from `node.ports`. This ensures IfCondition shows the correct number of branch outputs based on the `branches` field.

Example change in the GraphNode rendering:

```rust
// Get output ports dynamically based on variant
let output_ports = get_output_ports(&node.node_type, &node.variant);
let input_ports = node.ports.clone(); // Input ports are static
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/components/canvas/state.rs
git commit -m "feat: add dynamic port support for IfCondition"
```

---

## Task 9: Final Integration and Testing

- [ ] **Step 1: Run full dev server**

Run: `trunk serve`
Expected: Application loads without errors

- [ ] **Step 2: Test node creation and port rendering**

- Drag a Template node to canvas → should show input and output ports with labels
- Drag a IfCondition node → should show condition input and branch_1, branch_2 outputs
- Ports should have colored indicators based on type

- [ ] **Step 3: Test inspector**

- Click a node → inspector should open at bottom
- Inspector should show node type and properties

- [ ] **Step 4: Test connection creation**

- Drag from output port to input port → wire should connect
- Drag from trigger output to text input → should NOT connect (validation)

- [ ] **Step 5: Run cargo check --all-features**

Run: `cargo check --all-features`
Expected: PASS

- [ ] **Step 6: Final commit if everything works**

```bash
git add -A
git commit -m "feat: complete variant nodes implementation"
```

---

## Summary of Commits

1. `feat: add typed port data structures (PortType, Port, NodeVariant)`
2. `feat: add port color CSS variables`
3. `feat: update GraphNode to render typed ports and variant body`
4. `feat: update canvas to pass ports and variant to GraphNode`
5. `feat: add bottom drawer inspector component`
6. `feat: wire inspector to selected node signals`
7. `feat: add port type connection validation`
8. `feat: add dynamic port support for IfCondition`
9. `feat: complete variant nodes implementation`
