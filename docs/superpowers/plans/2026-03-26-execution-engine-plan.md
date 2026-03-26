# Execution Engine MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable parallel async execution of node graphs via Trigger nodes, with lazy request/response between nodes, and a threaded Discord-like trace in the right panel.

**Architecture:** Task queue model in frontend (Leptos signals). Each node becomes a task with status (Pending/Running/Waiting/Complete/Error). Tauri backend handles Code Execute only. Web Search is a stub.

**Tech Stack:** Leptos 0.7, wasm-bindgen, Tauri 2.x, web-sys, gloo-timers

> **Note on Tauri invoke:** The WASM frontend cannot call Tauri commands directly via `super::tauri::`. We use the `tauri` Rust crate with `invoke()` which wraps the JS API. Add `tauri = "2"` to `Cargo.toml` [dependencies].

---

## File Structure

```
src/
├── components/
│   ├── execution_engine.rs   [CREATE] Core engine: ExecutionState, Task, node executors
│   ├── execution_trace.rs     [CREATE] Threaded trace renderer for right panel
│   ├── mod.rs                 [MODIFY] Add execution_engine, execution_trace modules
│   ├── app_layout.rs          [MODIFY] Add ExecutionState signals, pass to panels
│   ├── right_panel.rs          [MODIFY] Replace placeholder with execution_trace
│   ├── left_panel.rs           [MODIFY] Add "trigger" node type
│   ├── canvas/state.rs        [MODIFY] Add status field to NodeState
│   └── canvas/canvas.rs       [MODIFY] Add run button per trigger node
src-tauri/src/
└── lib.rs                     [MODIFY] Add execute_code command
```

---

## Task 1: Add Trigger Node Type to Palette

**Files:**
- Modify: `src/components/left_panel.rs:12-97` (add to NODE_TYPES const)
- Modify: `src/components/mod.rs` (no change needed - module already exists)

- [ ] **Step 1: Add trigger node to NODE_TYPES**

In `left_panel.rs`, add to `NODE_TYPES` const array:

```rust
// Input
NodeType {
    id: "trigger",
    name: "Trigger",
    category: "Input",
    description: "Click to start execution",
},
```

- [ ] **Step 2: Commit**

```bash
git add src/components/left_panel.rs
git commit -m "feat: add trigger node type to palette"
```

---

## Task 2: Add NodeStatus to NodeState

**Files:**
- Modify: `src/components/canvas/state.rs`

- [ ] **Step 1: Add NodeStatus enum and status field to NodeState**

In `state.rs`, add:

```rust
/// Execution status of a node
#[derive(Clone, Debug, PartialEq)]
pub enum NodeStatus {
    Pending,
    Running,
    Waiting,
    Complete,
    Error,
}

/// Minimal node state for rendering
#[derive(Clone, Debug)]
pub struct NodeState {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub node_type: String,
    pub label: String,
    pub selected: bool,
    pub status: NodeStatus,  // ADD THIS FIELD
}
```

- [ ] **Step 2: Update app_layout.rs NodeState initializers**

In `app_layout.rs:23-48`, add `.status: NodeStatus::Pending` to each initial node.

- [ ] **Step 3: Update handle_node_drop closure**

In `app_layout.rs:75-87`, add `.status: NodeStatus::Pending` to new_node.

- [ ] **Step 4: Commit**

```bash
git add src/components/canvas/state.rs src/components/app_layout.rs
git commit -m "feat: add NodeStatus enum to NodeState"
```

---

## Task 3: Create Execution Engine Module

**Files:**
- Create: `src/components/execution_engine.rs`
- Modify: `src/components/mod.rs`
- Modify: `Cargo.toml` (add tauri dependency for invoke)

- [ ] **Step 1: Add tauri dependency to Cargo.toml**

Add to `Cargo.toml` [dependencies]:

```toml
tauri = "2"
```

- [ ] **Step 2: Create execution_engine.rs**

```rust
use leptos::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

/// Trace level for styling
#[derive(Clone, Debug)]
pub enum TraceLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A single entry in the execution trace
#[derive(Clone, Debug)]
pub struct TraceEntry {
    pub timestamp: Instant,
    pub message: String,
    pub level: TraceLevel,
}

impl TraceEntry {
    pub fn new(message: &str, level: TraceLevel) -> Self {
        Self {
            timestamp: Instant::now(),
            message: message.to_string(),
            level,
        }
    }
}

/// Execution status of a task
#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Waiting,
    Complete,
    Error,
}

/// A single task in the execution queue
#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub node_id: u32,
    pub node_type: String,
    pub status: TaskStatus,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub parent_id: Option<String>,
    pub messages: Vec<TraceEntry>,
    pub result: Option<String>,
    pub waiting_on: Option<u32>,  // node_id we're waiting for
}

impl Task {
    pub fn new(node_id: u32, node_type: &str, parent_id: Option<String>) -> Self {
        Self {
            id: format!("{}-{}", node_type, node_id),
            node_id,
            node_type: node_type.to_string(),
            status: TaskStatus::Pending,
            started_at: None,
            finished_at: None,
            parent_id,
            messages: Vec::new(),
            result: None,
            waiting_on: None,
        }
    }

    pub fn add_message(&mut self, msg: &str, level: TraceLevel) {
        self.messages.push(TraceEntry::new(msg, level));
    }
}

/// Execution engine state
#[derive(Clone, Debug)]
pub struct ExecutionState {
    pub tasks: Vec<Task>,
    pub running: bool,
}

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            running: false,
        }
    }
}

/// Find downstream node IDs connected to a node's output
pub fn get_downstream_nodes(connections: &[super::canvas::state::ConnectionState], node_id: u32) -> Vec<u32> {
    connections
        .iter()
        .filter(|c| c.source_node_id == node_id)
        .map(|c| c.target_node_id)
        .collect()
}

/// Find upstream node IDs connected to a node's input
pub fn get_upstream_nodes(connections: &[super::canvas::state::ConnectionState], node_id: u32) -> Vec<u32> {
    connections
        .iter()
        .filter(|c| c.target_node_id == node_id)
        .map(|c| c.source_node_id)
        .collect()
}

/// Call Tauri backend to execute code
pub async fn call_execute_code(code: &str) -> Result<String, String> {
    tauri::invoke("execute_code", &code.to_string())
        .await
        .map_err(|e| e.to_string())
}

/// Execute a node based on its type (non-async version for MVP)
pub fn execute_node_sync(
    node: &super::canvas::state::NodeState,
    upstream_results: &HashMap<u32, String>,
    parent_id: Option<String>,
) -> (Task, Option<String>) {
    let mut task = Task::new(node.id, &node.node_type, parent_id);
    task.status = TaskStatus::Running;
    task.started_at = Some(Instant::now());

    let result = match node.node_type.as_str() {
        "trigger" => {
            task.add_message("Trigger fired", TraceLevel::Info);
            None  // Trigger doesn't produce output itself
        }
        "web_search" => {
            task.add_message("Web Search → { query: 'mock results', results: [] }", TraceLevel::Info);
            Some(r#"{"query":"mock results","results":[]}"#.to_string())
        }
        "code_execute" => {
            // For MVP: stub - actual async call handled separately in app_layout
            task.add_message("Code Execute → (stubbed in MVP)", TraceLevel::Info);
            Some("code stubbed".to_string())
        }
        "user_input" => {
            task.add_message("User Input node", TraceLevel::Info);
            Some("user input value".to_string())
        }
        "template" => {
            task.add_message("Template node", TraceLevel::Info);
            Some("template output".to_string())
        }
        "planner_agent" | "executor_agent" => {
            task.add_message("Agent processing...", TraceLevel::Info);
            upstream_results.values().next().cloned()
        }
        "if_condition" | "loop" => {
            task.add_message("Control flow stub - taking first branch", TraceLevel::Warn);
            upstream_results.values().next().cloned()
        }
        "chat_output" | "json_output" => {
            let input = upstream_results.values().next().cloned().unwrap_or_default();
            task.add_message(&format!("Output: {}", input), TraceLevel::Info);
            Some(input)
        }
        _ => {
            task.add_message(&format!("Unknown node type: {}", node.node_type), TraceLevel::Warn);
            upstream_results.values().next().cloned()
        }
    };

    task.status = TaskStatus::Complete;
    task.finished_at = Some(Instant::now());
    if let Some(ref r) = result {
        task.result = Some(r.clone());
    }

    (task, result)
}
```

- [ ] **Step 2: Update mod.rs**

Add to `src/components/mod.rs`:

```rust
pub mod execution_engine;
pub mod execution_trace;
```

- [ ] **Step 3: Commit**

```bash
git add src/components/execution_engine.rs src/components/mod.rs
git commit -m "feat: create execution engine module with task queue"
```

---

## Task 4: Add Tauri execute_code Command

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add execute_code command**

Replace the contents of `src-tauri/src/lib.rs`:

```rust
use std::process::Command;

#[tauri::command]
fn execute_code(code: String) -> Result<String, String> {
    // Run via sh on mac/linux, cmd on windows
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &code])
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(["-c", &code])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![execute_code])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add execute_code Tauri command for code execution"
```

---

## Task 5: Create Execution Trace Renderer

**Files:**
- Create: `src/components/execution_trace.rs`

- [ ] **Step 1: Create execution_trace.rs**

```rust
use leptos::prelude::*;
use crate::components::execution_engine::{ExecutionState, Task, TraceLevel};

/// Format duration in milliseconds
fn format_duration(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

/// Format timestamp as HH:MM:SS.mmm
fn format_timestamp(instant: std::time::Instant) -> String {
    let elapsed = instant.elapsed().as_millis();
    let secs = (elapsed / 1000) % 60;
    let mins = (elapsed / 60000) % 60;
    let hours = elapsed / 3600000;
    let ms = elapsed % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms)
}

/// Get emoji for trace level
fn level_emoji(level: &TraceLevel) -> &'static str {
    match level {
        TraceLevel::Debug => "⚪",
        TraceLevel::Info => "🟢",
        TraceLevel::Warn => "🟡",
        TraceLevel::Error => "🔴",
    }
}

/// Get color class for task status
fn status_color(status: &crate::components::execution_engine::TaskStatus) -> &'static str {
    match status {
        crate::components::execution_engine::TaskStatus::Pending => "status-pending",
        crate::components::execution_engine::TaskStatus::Running => "status-running",
        crate::components::execution_engine::TaskStatus::Waiting => "status-waiting",
        crate::components::execution_engine::TaskStatus::Complete => "status-complete",
        crate::components::execution_engine::TaskStatus::Error => "status-error",
    }
}

#[component]
pub fn ExecutionTrace(
    execution: Signal<ExecutionState>,
) -> impl IntoView {
    view! {
        <div class="execution-trace">
            <div class="panel-header">"Execution Trace"</div>
            <div class="panel-content trace-content">
                {move || {
                    let exec = execution.get();
                    if exec.tasks.is_empty() {
                        view! {
                            <div class="trace-empty">
                                "Click a Trigger node to start execution"
                            </div>
                        }
                    } else {
                        exec.tasks.iter().map(|task| {
                            let status_class = status_color(&task.status);
                            let duration = task.started_at.map(|started| {
                                let end = task.finished_at.unwrap_or_else(std::time::Instant::now);
                                format_duration(end.duration_since(started).as_millis())
                            }).unwrap_or_default();

                            view! {
                                <div class="trace-thread">
                                    <div class="trace-task-header" class:status_class>
                                        <span class="trace-status-dot"></span>
                                        <span class="trace-node-type">{task.node_type.clone()}</span>
                                        <span class="trace-duration">{duration}</span>
                                    </div>
                                    <div class="trace-messages">
                                        {task.messages.iter().map(|msg| {
                                            let emoji = level_emoji(&msg.level);
                                            let ts = format_timestamp(msg.timestamp);
                                            view! {
                                                <div class="trace-message">
                                                    <span class="trace-ts">{ts}</span>
                                                    <span class="trace-emoji">{emoji}</span>
                                                    <span class="trace-msg-text">{msg.message.clone()}</span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }
                }}
            </div>
        </div>
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/execution_trace.rs
git commit -m "feat: create execution trace renderer"
```

---

## Task 6: Integrate Execution Engine into AppLayout

**Files:**
- Modify: `src/components/app_layout.rs`
- Modify: `src/components/right_panel.rs` (replace with ExecutionTrace)

- [ ] **Step 1: Add ExecutionState signal to AppLayout**

In `app_layout.rs`, add after line 58:

```rust
use crate::components::execution_engine::{ExecutionState, get_downstream_nodes};

let (execution_state, set_execution_state) = signal(ExecutionState::new());
```

- [ ] **Step 2: Add trigger handler function**

In `app_layout.rs`, add after line 58:

```rust
let handle_trigger = move |node_id: u32| {
    let nodes_snapshot = nodes.get();
    let connections_snapshot = connections.get();

    // Find the trigger node
    if let Some(trigger_node) = nodes_snapshot.iter().find(|n| n.id == node_id && n.node_type == "trigger") {
        // Get downstream nodes
        let downstream = get_downstream_nodes(&connections_snapshot, node_id);

        // Create execution state
        let mut exec = ExecutionState::new();
        exec.running = true;

        // Add trigger task
        let mut trigger_task = crate::components::execution_engine::Task::new(node_id, "trigger", None);
        trigger_task.status = crate::components::execution_engine::TaskStatus::Running;
        trigger_task.started_at = Some(std::time::Instant::now());
        trigger_task.add_message("Trigger fired", crate::components::execution_engine::TraceLevel::Info);
        trigger_task.finished_at = Some(std::time::Instant::now());
        trigger_task.status = crate::components::execution_engine::TaskStatus::Complete;
        exec.tasks.push(trigger_task);

        // Queue downstream tasks
        for downstream_id in downstream {
            if let Some(node) = nodes_snapshot.iter().find(|n| n.id == downstream_id) {
                let mut task = crate::components::execution_engine::Task::new(downstream_id, &node.node_type, None);
                task.status = crate::components::execution_engine::TaskStatus::Running;
                task.started_at = Some(std::time::Instant::now());
                task.add_message(&format!("Executing {}...", node.label), crate::components::execution_engine::TraceLevel::Info);

                // Simple synchronous execution for MVP (no actual async)
                // Web search stub
                let result = if node.node_type == "web_search" {
                    task.add_message("Web Search → { mock results }", crate::components::execution_engine::TraceLevel::Info);
                    Some(r#"{"query":"mock","results":[]}"#.to_string())
                } else if node.node_type == "code_execute" {
                    task.add_message("Code Execute → (TBD)", crate::components::execution_engine::TraceLevel::Info);
                    Some("code executed".to_string())
                } else {
                    task.add_message(&format!("{} complete", node.label), crate::components::execution_engine::TraceLevel::Info);
                    Some("ok".to_string())
                };

                task.finished_at = Some(std::time::Instant::now());
                task.status = crate::components::execution_engine::TaskStatus::Complete;
                task.result = result;
                exec.tasks.push(task);
            }
        }

        exec.running = false;
        set_execution_state.set(exec);
    }
};
```

- [ ] **Step 3: Replace RightPanel with ExecutionTrace**

In `app_layout.rs:200`, change:

```rust
<RightPanel />
```

to:

```rust
<ExecutionTrace execution={execution_state.into()} />
```

- [ ] **Step 4: Add ExecutionTrace import**

In `app_layout.rs:1-9`, add:

```rust
use crate::components::execution_trace::ExecutionTrace;
```

- [ ] **Step 5: Pass handle_trigger to Canvas**

In `app_layout.rs:177-187`, modify Canvas to pass trigger handler:

```rust
<Canvas
    nodes={nodes.into()}
    connections={connections.into()}
    selected_node_id={selected_node_id.into()}
    set_selected_node_id={set_selected_node_id}
    set_nodes={set_nodes}
    set_connections={set_connections}
    deleting_node_id={Some(deleting_node_id.into())}
    on_node_drop={Some(Callback::from(handle_node_drop))}
    left_width={Some(left_width.into())}
    on_trigger={Some(Callback::from(handle_trigger))}
/>
```

- [ ] **Step 6: Update Canvas to accept on_trigger prop**

In `canvas/canvas.rs`, add to Canvas component props:

```rust
/// Callback when trigger node is clicked
#[prop(default = None)] on_trigger: Option<Callback<u32>>,
```

And add to handle_mouse_down, BEFORE the dragging block (before line 175). When a trigger node is clicked, call the callback and return early without starting drag:

```rust
// Check if this is a trigger node - fire and don't drag
if let Some(node) = nodes_snapshot.iter().find(|n| n.id == node_id) {
    if node.node_type == "trigger" {
        if let Some(callback) = &on_trigger {
            callback.run(node_id);
        }
        return;  // Don't start dragging for trigger nodes
    }
}
```

- [ ] **Step 7: Commit**

```bash
git add src/components/app_layout.rs src/components/canvas/canvas.rs
git commit -m "feat: integrate execution engine into AppLayout"
```

---

## Task 7: Add CSS Styles for Trace

**Files:**
- Modify: `styles.css`

- [ ] **Step 1: Add trace styles**

Add to your CSS file:

```css
.execution-trace {
    display: flex;
    flex-direction: column;
    height: 100%;
}

.trace-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
}

.trace-empty {
    text-align: center;
    padding: 32px 16px;
    color: var(--text-secondary);
    font-size: 12px;
}

.trace-thread {
    margin-bottom: 12px;
    border-left: 2px solid var(--border-color);
    padding-left: 8px;
}

.trace-task-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    padding: 4px 0;
}

.trace-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
}

.status-pending .trace-status-dot { background: #888; }
.status-running .trace-status-dot { background: #3498db; animation: pulse 1s infinite; }
.status-waiting .trace-status-dot { background: #f39c12; }
.status-complete .trace-status-dot { background: #2ecc71; }
.status-error .trace-status-dot { background: #e74c3c; }

.status-running .trace-status-dot {
    animation: pulse 1s infinite;
}

@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}

.trace-node-type {
    text-transform: capitalize;
}

.trace-duration {
    margin-left: auto;
    color: var(--text-secondary);
    font-size: 11px;
}

.trace-messages {
    margin-left: 16px;
    border-left: 1px solid var(--border-subtle);
    padding-left: 8px;
}

.trace-message {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 11px;
    padding: 2px 0;
    color: var(--text-secondary);
}

.trace-ts {
    color: var(--text-tertiary);
    font-family: monospace;
    font-size: 10px;
}

.trace-emoji {
    font-size: 10px;
}

.trace-msg-text {
    color: var(--text-primary);
    word-break: break-word;
}
```

- [ ] **Step 2: Commit**

```bash
git add styles.css
git commit -m "feat: add CSS styles for execution trace"
```

---

## Task 8: Build and Test

- [ ] **Step 1: Run dev server**

```bash
trunk serve
```

- [ ] **Step 2: Verify build succeeds**

Expected: No errors, application loads at http://localhost:1420

- [ ] **Step 3: Test execution flow**

1. Drag a "Trigger" node from Input category to canvas
2. Drag an Agent node (e.g., "Planner Agent") and connect Trigger → Agent
3. Drag a Tool node (e.g., "Web Search") and connect Agent → Tool
4. Click the Trigger node
5. Right panel should show threaded trace with timestamps

---

## Verification

1. Trigger node appears in left panel palette under Input category
2. Clicking trigger node starts execution
3. Downstream nodes execute in parallel
4. Right panel shows threaded trace with colored status dots
5. Tauri backend logs code execution (visible in terminal)
6. Multiple triggers can fire independently