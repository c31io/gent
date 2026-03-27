# Implement Unused Warnings Features Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all 11 remaining `#[warn(dead_code)]` warnings by implementing the scaffolded features they represent.

**Architecture:** The work is grouped into 4 independent deliverables: (1) media port types and input nodes, (2) visual node status display, (3) connection identity for rerouting, (4) execution engine completion with graph traversal and code execution stubs.

**Tech Stack:** Leptos 0.7, WASM, Tauri, web-sys

---

## Files Overview

| File | Role |
|------|------|
| `src/components/canvas/state.rs` | `PortType` (add Image/Audio), `NodeStatus` (add variants), `NodeState` (add selected/status usage) |
| `src/components/canvas/canvas.rs` | Render node status visuals (colored border, spinner) |
| `src/components/nodes/node.rs` | Wire `selected`/`status` to DOM classes |
| `src/components/execution_engine.rs` | Implement `get_downstream/upstream_nodes`, `call_execute_code`, wire `Task` fields |
| `src/components/left_panel.rs` | Add Image Input and Audio Input node types |

---

## Group 1: Media Port Types and Input Nodes

### Task 1.1: Confirm Image and Audio port type variants (Already Done)

**Files:**
- Verify: `src/components/canvas/state.rs:10-17`

- [ ] **Step 1: Verify PortType already has Image and Audio**

Check `state.rs` — the `PortType` enum already contains `Image` and `Audio` variants. This task is complete. No changes needed.

Run: `cargo check 2>&1 | grep "PortType"`
Expected: Only the "never constructed" warning (will be eliminated by Tasks 1.2/1.3)

- [ ] **Step 2: Commit note**

```bash
# No changes needed - already implemented
git status
```

---

### Task 1.2: Add Image Input node type to left panel

**Files:**
- Modify: `src/components/left_panel.rs` (find `NODE_TYPES` const, add `image_input` entry)

- [ ] **Step 1: Add Image Input to NODE_TYPES**

Find the `NODE_TYPES` const and add:

```rust
{
    id: "image_input",
    name: "Image Input",
    category: "Input",
    description: "Provides an image file path as output",
},
```

- [ ] **Step 2: Add default ports for image_input in state.rs**

Add to `default_ports_for_type()`:

```rust
"image_input" => vec![
    Port { name: "output".into(), port_type: PortType::Image, direction: PortDirection::Out },
],
```

- [ ] **Step 3: Add default variant for image_input in state.rs**

Add to `default_variant_for_type()`:

```rust
"image_input" => NodeVariant::FileInput { path: String::new() },
```

- [ ] **Step 4: Handle image_input in get_output_ports in state.rs**

Add branch in `get_output_ports()`:

```rust
"image_input" => ports,
```

- [ ] **Step 5: Handle image_input in execute_node_sync in execution_engine.rs**

Add branch:

```rust
"image_input" => {
    if let NodeVariant::FileInput { path } = &node.variant {
        task.add_message(&format!("Image: {}", path), TraceLevel::Info);
        result.clone()
    } else {
        task.add_message("Image Input (no path)", TraceLevel::Warn);
        String::new()
    }
}
```

- [ ] **Step 6: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add src/components/left_panel.rs src/components/canvas/state.rs src/components/execution_engine.rs
git commit -m "feat(nodes): add Image Input node type"
```

---

### Task 1.3: Add Audio Input node type to left panel

**Files:**
- Modify: `src/components/left_panel.rs`, `src/components/canvas/state.rs`, `src/components/execution_engine.rs`

- [ ] **Step 1: Add Audio Input to NODE_TYPES in left_panel.rs**

```rust
{
    id: "audio_input",
    name: "Audio Input",
    category: "Input",
    description: "Provides an audio file path as output",
},
```

- [ ] **Step 2: Add default ports for audio_input in state.rs**

```rust
"audio_input" => vec![
    Port { name: "output".into(), port_type: PortType::Audio, direction: PortDirection::Out },
],
```

- [ ] **Step 3: Add default variant for audio_input in state.rs**

```rust
"audio_input" => NodeVariant::FileInput { path: String::new() },
```

- [ ] **Step 4: Add branch in get_output_ports in state.rs**

```rust
"audio_input" => ports,
```

- [ ] **Step 5: Add branch in execute_node_sync in execution_engine.rs**

```rust
"audio_input" => {
    if let NodeVariant::FileInput { path } = &node.variant {
        task.add_message(&format!("Audio: {}", path), TraceLevel::Info);
        result.clone()
    } else {
        task.add_message("Audio Input (no path)", TraceLevel::Warn);
        String::new()
    }
}
```

- [ ] **Step 6: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; Image/Audio warnings gone

- [ ] **Step 7: Commit**

```bash
git add src/components/left_panel.rs src/components/canvas/state.rs src/components/execution_engine.rs
git commit -m "feat(nodes): add Audio Input node type"
```

---

## Group 2: Visual Node Status Display

### Task 2.1: Wire NodeState.selected and status to DOM class rendering

**Files:**
- Modify: `src/components/nodes/node.rs` — add `status: NodeStatus` prop, use it in class attribute
- Modify: `src/components/canvas/canvas.rs` — pass `status={node.status}` to GraphNode

- [ ] **Step 1: Add status prop to GraphNode in node.rs**

Find the GraphNode function signature (around line 154) and add `status: NodeStatus` after `selected: bool`:

```rust
pub fn GraphNode(
    x: f64,
    y: f64,
    label: String,
    selected: bool,
    status: NodeStatus,  // ADD THIS
    node_id: u32,
    ...
)
```

Also add import at top of file: `use crate::components::canvas::state::NodeStatus;`

- [ ] **Step 2: Update class computation to include status class in node.rs**

Replace the class computation (around line 173) with:

```rust
let status_class = match status {
    NodeStatus::Pending => "",
    NodeStatus::Running => "node-running",
    NodeStatus::Waiting => "node-waiting",
    NodeStatus::Complete => "node-complete",
    NodeStatus::Error => "node-error",
};
let class = if selected {
    format!("node selected {}", status_class)
} else {
    format!("node {}", status_class)
};
let class = if is_deleting {
    format!("{} deleting", class)
} else {
    class
};
```

- [ ] **Step 3: Pass status to GraphNode in canvas.rs**

In `canvas.rs` around line 634, add `status={node.status}`:

```rust
<GraphNode
    x={node.x}
    y={node.y}
    label={node.label.clone()}
    selected={is_selected}
    status={node.status}  // ADD THIS
    node_id={node.id}
    ...
```

- [ ] **Step 4: Add CSS for node status classes**

Find the CSS (likely in `src/` as inline or `index.html`) and add:

```css
.node-running { border-color: #f59e0b; }
.node-waiting { border-color: #8b5cf6; }
.node-complete { border-color: #22c55e; }
.node-error { border-color: #ef4444; }
```

- [ ] **Step 5: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; NodeState.selected and status field warnings gone

- [ ] **Step 6: Commit**

```bash
git add src/components/nodes/node.rs src/components/canvas/canvas.rs
git commit -m "feat(canvas): wire node selected/status to DOM classes for visual display"
```

---

### Task 2.2: Update Execution Engine to set NodeStatus during execution

**Files:**
- Modify: `src/components/app_layout.rs:handle_trigger` — set `NodeStatus::Running`, `NodeStatus::Complete`, `NodeStatus::Error` on nodes as they execute

- [ ] **Step 1: In handle_trigger, update node status during execution**

Find where tasks are created in `handle_trigger` and update node statuses:

```rust
// At start of node execution:
set_nodes.update(|nodes: &mut Vec<NodeState>| {
    if let Some(n) = nodes.iter_mut().find(|n| n.id == exec_node_id) {
        n.status = crate::components::canvas::state::NodeStatus::Running;
    }
});

// On success:
set_nodes.update(|nodes: &mut Vec<NodeState>| {
    if let Some(n) = nodes.iter_mut().find(|n| n.id == exec_node_id) {
        n.status = crate::components::canvas::state::NodeStatus::Complete;
    }
});

// On error (add error handling in node execution):
set_nodes.update(|nodes: &mut Vec<NodeState>| {
    if let Some(n) = nodes.iter_mut().find(|n| n.id == exec_node_id) {
        n.status = crate::components::canvas::state::NodeStatus::Error;
    }
});
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; NodeStatus variants no longer produce warnings

- [ ] **Step 3: Commit**

```bash
git add src/components/app_layout.rs
git commit -m "feat(execution): update node status during execution for visual feedback"
```

---

## Group 3: Connection Identity for Rerouting

### Task 3.1: Use ConnectionState.id in rerouting logic

**Files:**
- Modify: `src/components/canvas/canvas.rs` — use `connection.id` when storing/using connections

- [ ] **Step 1: Find where connection id should be used**

In `canvas.rs`, find where `ConnectionState` is created and ensure `id` is set. It should already be set in `app_layout.rs` when connections are created (the `id` field exists). The warning is that the `id` field is never *read* — find where it should be used.

Search for rerouting logic in canvas.rs — when a connection is picked up for rerouting, it should store the connection `id`. Currently `DraggingConnection` doesn't track which connection is being rerouted — that's where `id` should be used.

- [ ] **Step 2: Add connection_id to DraggingConnection in state.rs**

In `state.rs`, update `DraggingConnection`:

```rust
#[derive(Clone, Debug)]
pub struct DraggingConnection {
    pub connection_id: Option<u32>,  // None = new connection, Some = reroute
    pub source_node_id: u32,
    pub source_port_name: String,
    pub source_input_node_id: Option<u32>,
    pub current_x: f64,
    pub current_y: f64,
    pub is_dragging: bool,
}
```

- [ ] **Step 3: Update DraggingConnection creation in canvas.rs**

When starting a reroute from an existing connection, store its `id`:

```rust
// In reroute handler
DraggingConnection {
    connection_id: Some(connection.id),
    source_node_id: connection.source_node_id,
    source_port_name: connection.source_port_name.clone(),
    source_input_node_id: Some(connection.target_node_id),
    current_x: /* ... */,
    current_y: /* ... */,
    is_dragging: true,
}
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; ConnectionState.id warning gone

- [ ] **Step 5: Commit**

```bash
git add src/components/canvas/state.rs src/components/canvas/canvas.rs
git commit -m "feat(connections): track connection id for rerouting"
```

---

## Group 4: Execution Engine Completion

### Task 4.1: Wire get_downstream_nodes and get_upstream_nodes into execution

**Files:**
- Modify: `src/components/app_layout.rs` — use `get_downstream_nodes` and `get_upstream_nodes` from execution_engine

- [ ] **Step 1: Replace inline adjacency computation with get_downstream_nodes**

In `handle_trigger`, the `execute_downstream_order` function has inline adjacency computation. Replace it with calls to `get_downstream_nodes` and `get_upstream_nodes` from execution_engine.

```rust
// Instead of building adj HashMap inline, use:
let downstream = crate::components::execution_engine::get_downstream_nodes(&connections_snapshot, node_id);
let upstream = crate::components::execution_engine::get_upstream_nodes(&connections_snapshot, node_id);
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; get_downstream/upstream_nodes warnings gone

- [ ] **Step 3: Commit**

```bash
git add src/components/app_layout.rs
git commit -m "feat(execution): wire graph traversal helpers into execution engine"
```

---

### Task 4.2: Wire Task fields (id, node_id, parent_id, waiting_on)

**Files:**
- Modify: `src/components/execution_engine.rs` — use Task fields in execute_node_sync
- Modify: `src/components/app_layout.rs` — set Task.id, Task.parent_id, Task.waiting_on when creating tasks

- [ ] **Step 1: In handle_trigger, populate Task fields**

When creating Tasks in `handle_trigger`:

```rust
let mut task = crate::components::execution_engine::Task::new(exec_node_id, &node.node_type, parent_id.clone());
// Now fields like id, node_id, parent_id are set by Task::new()

// Set waiting_on if waiting on upstream:
if upstream.is_empty() {
    task.waiting_on = None;
} else {
    task.waiting_on = upstream.keys().next().copied();
}
```

- [ ] **Step 2: In execute_node_sync, use Task fields**

The `execute_node_sync` function receives a `node` and creates a `Task`. The `Task.id` and `Task.node_id` are already set by `Task::new`. Ensure the function uses them in logging/messages.

- [ ] **Step 3: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; Task field warnings gone

- [ ] **Step 4: Commit**

```bash
git add src/components/app_layout.rs src/components/execution_engine.rs
git commit -m "feat(execution): populate and use Task tracking fields"
```

---

### Task 4.3: Implement TraceLevel::Debug and Error usage

**Files:**
- Modify: `src/components/execution_engine.rs` — use `TraceLevel::Debug` and `TraceLevel::Error` in appropriate places
- Modify: `src/components/app_layout.rs` — use `TraceLevel::Error` for error cases

- [ ] **Step 1: Add TraceLevel::Debug for verbose execution logging**

In `execute_node_sync` (or `handle_trigger`), add debug-level messages:

```rust
task.add_message(&format!("Starting {} node execution", node.node_type), TraceLevel::Debug);
```

- [ ] **Step 2: Add TraceLevel::Error for error cases**

In error branches of node execution:

```rust
task.add_message("Node execution failed", TraceLevel::Error);
task.status = TaskStatus::Error;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; TraceLevel::Debug and Error warnings gone

- [ ] **Step 4: Commit**

```bash
git add src/components/execution_engine.rs src/components/app_layout.rs
git commit -m "feat(execution): add debug and error trace levels"
```

---

### Task 4.4: Implement TaskStatus::Waiting and Error usage

**Files:**
- Modify: `src/components/execution_engine.rs` — use `TaskStatus::Waiting` and `TaskStatus::Error` in execute_node_sync
- Modify: `src/components/app_layout.rs` — use `TaskStatus::Waiting` when waiting for upstream

- [ ] **Step 1: Set TaskStatus::Waiting when waiting on upstream results**

In `handle_trigger`, when a node can't execute because upstream results aren't ready:

```rust
// If we need results from nodes that haven't run yet:
task.status = TaskStatus::Waiting;
task.waiting_on = Some(waiting_node_id);
```

- [ ] **Step 2: Set TaskStatus::Error on execution failure**

```rust
task.status = TaskStatus::Error;
task.finished_at = Some(Timestamp::now());
```

- [ ] **Step 3: Update execute_node_sync to set TaskStatus::Error on failure**

In `execute_node_sync`, add:

```rust
_ => {
    task.add_message(&format!("Unknown node type: {}", node.node_type), TraceLevel::Error);
    task.status = TaskStatus::Error;
    task.finished_at = Some(Timestamp::now());
    (task, None)
}
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; TaskStatus::Waiting and Error warnings gone

- [ ] **Step 5: Commit**

```bash
git add src/components/execution_engine.rs src/components/app_layout.rs
git commit -m "feat(execution): add task waiting and error states"
```

---

### Task 4.5: Add #[allow(dead_code)] to call_execute_code stub

**Files:**
- Modify: `src/components/execution_engine.rs` — mark `call_execute_code` with explicit allow

- [ ] **Step 1: Add #[allow(dead_code)] to call_execute_code**

Find `call_execute_code` (around line 131) and add `#[allow(dead_code)]` with a TODO:

```rust
/// Execute code via Tauri backend (TODO: wire to Tauri invoke)
#[allow(dead_code)]
pub async fn call_execute_code(_code: &str) -> Result<String, String> {
    Ok("code execution stubbed".to_string())
}
```

This is a stub — the actual Tauri WASM invoke wiring is a separate future story.

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; call_execute_code warning gone (it's now explicitly allowed)

- [ ] **Step 3: Commit**

```bash
git add src/components/execution_engine.rs
git commit -m "feat(execution): mark call_execute_code stub with #[allow(dead_code)]"
```

---

### Task 4.6: Wire call_execute_code into code_execute node

**Files:**
- Modify: `src/components/app_layout.rs` — in `handle_trigger`, call `call_execute_code` for `code_execute` nodes

- [ ] **Step 1: Call call_execute_code in code_execute node handler**

In `handle_trigger`, replace the `code_execute` stub:

```rust
"code_execute" => {
    let code = if let crate::components::canvas::state::NodeVariant::CodeExecute { code, .. } = &node.variant {
        code.clone()
    } else {
        String::new()
    };
    // Use the helper function
    let result = crate::components::execution_engine::call_execute_code(&code).await;
    match result {
        Ok(output) => {
            task.add_message(&format!("Code executed: {}", output), crate::components::execution_engine::TraceLevel::Info);
            output
        }
        Err(e) => {
            task.add_message(&format!("Code error: {}", e), crate::components::execution_engine::TraceLevel::Error);
            task.status = crate::components::execution_engine::TaskStatus::Error;
            String::new()
        }
    }
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; call_execute_code warning gone

- [ ] **Step 3: Commit**

```bash
git add src/components/app_layout.rs
git commit -m "feat(execution): wire call_execute_code into code_execute nodes"
```

---

### Task 4.7: Remove redundant execute_node_sync

**Files:**
- Delete: `src/components/execution_engine.rs` — remove `execute_node_sync` function (lines 137-193)

- [ ] **Step 1: Delete execute_node_sync function**

The review confirmed `execute_node_sync` is never called — `handle_trigger` does all execution inline. Delete the entire function from `execution_engine.rs`.

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`
Expected: No errors; execute_node_sync warning gone

- [ ] **Step 3: Commit**

```bash
git add src/components/execution_engine.rs
git commit -m "feat(execution): remove redundant execute_node_sync function"
```

---

## Final Verification

- [ ] **Run full cargo check to confirm zero warnings**

Run: `cargo check 2>&1`
Expected: Zero warnings (or only intentional `#[allow(dead_code)]` warnings)

---

## Notes

- Tasks can be executed in any order within and across groups
- Each task is self-contained — changes don't cascade between tasks (except within a group)
- CSS for node status classes may be in `index.html` inline styles, `src/app.css`, or similar — check existing CSS structure first
