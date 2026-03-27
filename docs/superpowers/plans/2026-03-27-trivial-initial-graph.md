# Trivial Initial Graph Execution Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After clicking Trigger node's Run button, the text entered in Text Input node displays in Text Output node.

**Architecture:** Execute nodes in topological order (data-flow order). Each node reads its input connections' source node outputs, executes, and passes results downstream. The `user_input` node must return its actual `text` variant field. The `chat_output` node must update its `response` variant field.

**Tech Stack:** Leptos 0.7, wasm-bindgen, Tauri, web-sys

---

## File Structure

**Files to Modify:**
- `src/components/app_layout.rs:94-147` - `handle_trigger` function needs topological execution order and data flow
- `src/components/execution_engine.rs:137-193` - `execute_node_sync` needs to read `user_input` text and update `chat_output` response
- `src/components/nodes/node.rs` - Add `on_text_change` callback to `UserInput` variant to sync typed text to node state
- `src/components/canvas/canvas.rs` - Pass `on_text_change` callback from app_layout through to GraphNode

---

## Initial State (Already Wired)

```
Trigger (id=1) --trigger port--> Text Input (id=2) --output port--> Text Output (id=3)
```

- Trigger node output `trigger` port → Text Input node `trigger` input port
- Text Input node `output` port → Text Output node `response` input port

---

## Task 0: Sync Text Input State

**Files:**
- Modify: `src/components/nodes/node.rs` - Add `on_text_change` callback prop and wire to `UserInput` input
- Modify: `src/components/canvas/canvas.rs` - Add `on_text_change` prop and pass to GraphNode
- Modify: `src/components/app_layout.rs` - Provide callback to update node's `UserInput { text }` variant

- [ ] **Step 1: Add `on_text_change` prop to GraphNode in node.rs**

**Note:** This step must be done FIRST since Tasks 1 and 2 depend on it.

In `node.rs`, add a new callback prop to the `GraphNode` component:

```rust
on_text_change: Option<Callback<(u32, String)>>,
```

Then in the `UserInput` variant rendering, add an `on:input` handler:

```rust
NodeVariant::UserInput { text } => view! {
    <input
        type="text"
        class="node-variant-input"
        value={text.clone()}
        placeholder="Enter text..."
        on:input={move |ev| {
            let new_text = event_target_value(&ev);
            if let Some(cb) = &on_text_change {
                cb.run((node_id, new_text));
            }
        }}
    />
}.into_any(),
```

Note: Use `event_target_value` from leptos::event to get the input value.

- [ ] **Step 2: Add `on_text_change` prop to Canvas in canvas.rs**

In `canvas.rs`, add the prop to the `Canvas` component:

```rust
/// Callback when text input changes in a node
#[prop(default = None)] on_text_change: Option<Callback<(u32, String)>>,
```

Then when rendering `GraphNode` (around line 628), pass the callback:

```rust
<GraphNode
    ...
    on_text_change={on_text_change}
    ...
/>
```

- [ ] **Step 3: Provide `on_text_change` callback in app_layout.rs**

In `app_layout.rs`, define a handler that updates the node's text variant:

```rust
let handle_text_change = move |node_id: u32, new_text: String| {
    set_nodes.update(|nodes: &mut Vec<NodeState>| {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == node_id) {
            if let crate::components::canvas::state::NodeVariant::UserInput { text } = &mut node.variant {
                *text = new_text;
            }
        }
    });
};
```

Then pass it to Canvas:

```rust
<Canvas
    ...
    on_text_change={Some(Callback::new(handle_text_change))}
    ...
/>
```

- [ ] **Step 4: Verify imports**

Ensure `event_target_value` is imported from `leptos::event` or `leptos::html`.

- [ ] **Step 5: Test the build**

Run: `cargo check`
Expected: No errors

---

## Task 1: Topological Execution Order in `handle_trigger`

**Files:**
- Modify: `src/components/app_layout.rs:94-147`

- [ ] **Step 1: Add topological sort helper function**

Add a helper function before `handle_trigger` to sort nodes by dependencies:

```rust
/// Execute nodes in topological order (BFS from trigger), collecting upstream results
fn execute_downstream_order(
    nodes: &[NodeState],
    connections: &[ConnectionState],
    trigger_id: u32,
) -> Vec<(u32, HashMap<u32, String>)> {
    use std::collections::{HashMap, VecDeque};

    let mut in_degree: HashMap<u32, usize> = HashMap::new();
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();

    for node in nodes {
        in_degree.insert(node.id, 0);
        adj.insert(node.id, vec![]);
    }

    for conn in connections {
        if let Some(list) = adj.get_mut(&conn.source_node_id) {
            list.push(conn.target_node_id);
        }
        *in_degree.entry(conn.target_node_id).or_insert(0) += 1;
    }

    // BFS from trigger
    let mut queue: VecDeque<u32> = VecDeque::new();
    queue.push_back(trigger_id);

    let mut execution_order: Vec<(u32, HashMap<u32, String>)> = vec![];
    let mut upstream_results: HashMap<u32, String> = HashMap::new();

    while let Some(node_id) = queue.pop_front() {
        execution_order.push((node_id, upstream_results.clone()));

        if let Some(downstream_ids) = adj.get(&node_id) {
            for &downstream_id in downstream_ids {
                *in_degree.entry(downstream_id).or_insert(0) -= 1;
                if in_degree[&downstream_id] == 0 {
                    queue.push_back(downstream_id);
                }
            }
        }
    }

    execution_order
}
```

- [ ] **Step 2: Rewrite `handle_trigger` to use topological order and propagate data**

Replace the `handle_trigger` function body (lines 94-147) with:

```rust
let handle_trigger = move |node_id: u32| {
    let nodes_snapshot = nodes.get();
    let connections_snapshot = connections.get();

    if nodes_snapshot.iter().find(|n| n.id == node_id && n.node_type == "trigger").is_none() {
        return;
    }

    // Get execution order with upstream results
    let execution_plan = execute_downstream_order(&nodes_snapshot, &connections_snapshot, node_id);

    let mut exec = ExecutionState::new();
    exec.running = true;

    let mut node_results: HashMap<u32, String> = HashMap::new();

    for (exec_node_id, upstream) in execution_plan {
        if exec_node_id == node_id {
            // Trigger node itself
            let mut task = crate::components::execution_engine::Task::new(exec_node_id, "trigger", None);
            task.status = crate::components::execution_engine::TaskStatus::Running;
            task.started_at = Some(crate::components::execution_engine::Timestamp::now());
            task.add_message("Trigger fired", crate::components::execution_engine::TraceLevel::Info);
            task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
            task.status = crate::components::execution_engine::TaskStatus::Complete;
            exec.tasks.push(task);
        } else {
            // Find the node state
            if let Some(node) = nodes_snapshot.iter().find(|n| n.id == exec_node_id) {
                let mut task = crate::components::execution_engine::Task::new(exec_node_id, &node.node_type, None);
                task.status = crate::components::execution_engine::TaskStatus::Running;
                task.started_at = Some(crate::components::execution_engine::Timestamp::now());

                // Execute node with upstream results
                let result = match node.node_type.as_str() {
                    "user_input" => {
                        if let crate::components::canvas::state::NodeVariant::UserInput { text } = &node.variant {
                            task.add_message(&format!("Text Input: {}", text), crate::components::execution_engine::TraceLevel::Info);
                            text.clone()
                        } else {
                            task.add_message("Text Input (no text)", crate::components::execution_engine::TraceLevel::Warn);
                            String::new()
                        }
                    }
                    "chat_output" => {
                        // Get input from upstream (user_input node)
                        let input = upstream.values().next().cloned().unwrap_or_default();
                        task.add_message(&format!("Text Output received: {}", input), crate::components::execution_engine::TraceLevel::Info);
                        // Update the chat_output node's variant response
                        set_nodes.update(|nodes: &mut Vec<NodeState>| {
                            if let Some(n) = nodes.iter_mut().find(|n| n.id == exec_node_id) {
                                if let crate::components::canvas::state::NodeVariant::ChatOutput { response } = &mut n.variant {
                                    *response = input.clone();
                                }
                            }
                        });
                        input
                    }
                    "web_search" => {
                        task.add_message("Web Search → { mock results }", crate::components::execution_engine::TraceLevel::Info);
                        r#"{"query":"mock","results":[]}"#.to_string()
                    }
                    "code_execute" => {
                        task.add_message("Code Execute → (TBD)", crate::components::execution_engine::TraceLevel::Info);
                        "code executed".to_string()
                    }
                    _ => {
                        task.add_message(&format!("{} executed", node.label), crate::components::execution_engine::TraceLevel::Info);
                        upstream.values().next().cloned().unwrap_or_default()
                    }
                };

                task.result = Some(result.clone());
                node_results.insert(exec_node_id, result.clone());

                task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                task.status = crate::components::execution_engine::TaskStatus::Complete;
                exec.tasks.push(task);
            }
        }
    }

    exec.running = false;
    set_execution_state.set(exec);
};
```

- [ ] **Step 3: Add HashMap import**

At the top of `app_layout.rs`, ensure `use std::collections::HashMap;` is present (it may already be imported via other modules, but add it explicitly if needed for the new code).

- [ ] **Step 4: Test the build**

Run: `cargo check`
Expected: No errors

---

## Task 2: Verify Data Display in Text Output Node

**Files:**
- Read: `src/components/nodes/node.rs` - Verify `ChatOutput` variant body rendering
- Read: `src/components/canvas/canvas.rs:614-647` - Verify GraphNode receives updated variant

- [ ] **Step 1: Check ChatOutput rendering**

In `node.rs:124-130`, `ChatOutput` renders a textarea with `{response.clone()}`. Since Leptos reactive signals update the DOM when values change, and we're calling `set_nodes.update()` with the new response, the UI should update automatically.

- [ ] **Step 2: Verify GraphNode re-renders on variant change**

In `canvas.rs:617-646`, the GraphNode is rendered inside a `move || { ... }` closure that depends on `nodes.get()`. When `set_nodes.update()` is called, this should trigger a re-render with the new variant.

---

## Task 3: Manual Test

- [ ] **Step 1: Start dev server**

Run: `trunk serve`
Open: http://localhost:1420

- [ ] **Step 2: Enter text in Text Input node**

Click on the Text Input node's text field and type "Hello World"

- [ ] **Step 3: Click Trigger node's Run button**

The trigger button should be red and say "Run"

- [ ] **Step 4: Verify text appears in Text Output node**

The Text Output node's textarea should now contain "Hello World"

- [ ] **Step 5: Change text and re-run**

Modify the text in Text Input, click Run again, verify updated text in Text Output

---

## Summary

**Root Cause:** `handle_trigger` executed all downstream nodes in parallel without data propagation. `user_input` returned a stub string, and `chat_output` never received or displayed the actual input.

**Fix:**
1. Topological sort ensures `user_input` executes before `chat_output`
2. `user_input` returns its actual `text` variant field
3. `chat_output` receives upstream data via `upstream_results` map and updates its variant

**Files Changed:**
- `src/components/app_layout.rs`: Rewrote `handle_trigger` with topological sort and data flow
