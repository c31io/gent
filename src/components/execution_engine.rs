use std::collections::HashMap;

/// WASM-compatible timestamp using js_sys::Date
#[derive(Clone, Copy, Debug)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn now() -> Self {
        Self(js_sys::Date::now() as u64)
    }

    pub fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::from_millis(js_sys::Date::now() as u64 - self.0)
    }

    pub fn duration_since(&self, earlier: Timestamp) -> std::time::Duration {
        std::time::Duration::from_millis(self.0 - earlier.0)
    }
}

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
    pub timestamp: Timestamp,
    pub message: String,
    pub level: TraceLevel,
}

impl TraceEntry {
    pub fn new(message: &str, level: TraceLevel) -> Self {
        Self {
            timestamp: Timestamp::now(),
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
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
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

/// Call Tauri backend to execute code (stubbed for WASM - uses JS bindings in practice)
/// For MVP, this is stubbed. Real implementation would use wasm_bindgen_futures
/// to call into JS which then calls window.__TAURI__.core.invoke
pub async fn call_execute_code(_code: &str) -> Result<String, String> {
    // Stubbed for MVP - real implementation would call Tauri via JS bindings
    Ok("code execution stubbed".to_string())
}

/// Execute a node based on its type (non-async version for MVP)
pub fn execute_node_sync(
    node: &super::canvas::state::NodeState,
    upstream_results: &HashMap<u32, String>,
    parent_id: Option<String>,
) -> (Task, Option<String>) {
    let mut task = Task::new(node.id, &node.node_type, parent_id);
    task.status = TaskStatus::Running;
    task.started_at = Some(Timestamp::now());

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
    task.finished_at = Some(Timestamp::now());
    if let Some(ref r) = result {
        task.result = Some(r.clone());
    }

    (task, result)
}