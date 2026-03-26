# Execution Engine MVP — Design

## Context

Gent is a visual node editor for agent orchestration. The canvas, wires, and node palette are built, but nothing actually runs. This spec adds an execution engine so users can trigger node graphs and see results flow through the system.

**Problem**: Nodes exist but don't execute. Right panel shows placeholder data.
**Goal**: Enable parallel async execution with a threaded trace display.

---

## Architecture

### Execution Model: Task Queue + Request/Response

Each node becomes a **task** in a queue. Nodes operate in three modes:

- **Run** — Execute logic and emit results downstream
- **Request** — Ask upstream for data, wait for response
- **Emit** — Send result to downstream tasks

Execution is **lazy** — a node only runs when explicitly requested by a downstream node (not pushed when upstream completes).

**Parallel** — If a trigger connects to multiple downstream nodes, all run concurrently.

### Trigger Model

A **Trigger node** fires when clicked. It starts execution of its downstream graph. Multiple triggers can fire independently, each creating a separate execution trace.

---

## Data Structures

```rust
enum NodeStatus {
    Pending,
    Running,
    Waiting,    // awaiting upstream response
    Complete,
    Error,
}

struct Task {
    id: String,
    node_id: String,
    status: NodeStatus,
    started_at: Option<Instant>,
    finished_at: Option<Instant>,
    parent_id: Option<String>,   // for threading hierarchy
    messages: Vec<TraceEntry>,  // trace messages
    result: Option<String>,
}

struct TraceEntry {
    timestamp: Instant,
    message: String,
    level: TraceLevel,  // Debug, Info, Warn, Error
}

struct ExecutionState {
    tasks: Vec<Task>,
    running: bool,
}
```

---

## Node Execution

### Trigger Node
- Click to fire — starts execution downstream
- Visual pulse when triggered
- No input port, one output port

### Agent Nodes (Planner Agent, Executor Agent)
- Receive task request from trigger or upstream
- Process request (for MVP: just pass through or call a tool)
- Respond with result when done
- Single agent for MVP — no branching/looping

### Tool Nodes

**Web Search (stub)**
- Returns mock JSON immediately when called
- Does not require API key or network

**Code Execute**
- Sends code string to Tauri backend via `invoke("execute_code", { code })`
- Backend runs code via `std::process::Command`
- Returns stdout on success, stderr on error

### Control Flow Nodes (If/Condition, Loop)
- Stub for MVP — execute single branch only
- Full branching/looping deferred to future work

### Output Nodes (Chat Response, JSON Output)
- Receive final result, display in right panel

---

## Tauri Backend Command

```rust
#[tauri::command]
fn execute_code(code: String) -> Result<String, String> {
    // Run via std::process::Command
    // Returns stdout on success, stderr on error
}
```

---

## Right Panel Trace

Threaded history (Discord-like), showing:

```
[12:01:23] 🔵 Trigger fired
  └─ [12:01:24] 🟡 Agent: "Processing..."
     └─ [12:01:25] 🟢 Web Search → { mock results }
     └─ [12:01:26] 🟢 Code Execute → "hello world"
  └─ [12:01:27] 🔵 Complete
```

Each task has a thread showing:
- State transitions (pending → running → waiting → complete/error)
- Timestamps at each step
- Messages/logs from the node
- Nested indentation for child tasks

---

## Component Changes

### New Files
- `src/components/execution_engine.rs` — Core engine: `ExecutionState` signal, task queue, node executor
- `src/components/execution_trace.rs` — Right panel render function for threaded trace
- `src-tauri/src/main.rs` — Add `execute_code` command

### Modified Files
- `src/components/app_layout.rs` — Add `ExecutionState` signal, pass to canvas and right panel
- `src/components/right_panel.rs` — Replace placeholder trace with execution_trace rendering
- `src/components/left_panel.rs` — Add Trigger node type
- `src/components/canvas/canvas.rs` — Add run button per trigger node
- `src/components/nodes/node.rs` — Add `status` field to `NodeState`

### Files Not Modified (MVP scope)
- Wire system (`wires.rs`) — no changes needed
- Node inspector — still works as-is
- Geometry utilities — no changes

---

## Verification

1. Add Trigger node from palette to canvas
2. Connect Trigger → Agent → Tool (Web Search or Code Execute)
3. Click trigger node — execution starts
4. Right panel shows threaded trace with timestamps and state colors
5. Code Execute (with real code) returns actual stdout from Tauri backend
6. Multiple triggers fire independently in parallel

---

## Out of Scope (Future PRs)

- Real Web Search API integration
- Full If/Loop branching logic
- Graph save/load persistence
- Node configuration UI (beyond stubs)
- Meta-agent graph modification
- Undo/redo, multi-select, keyboard shortcuts