/// Direction for a port
#[derive(Clone, Debug, PartialEq, Hash)]
pub enum PortDirection {
    In,
    Out,
}

/// Type of data flowing through a port
#[derive(Clone, Debug, PartialEq, Hash)]
pub enum PortType {
    Text,       // blue #3b82f6
    Image,      // green #22c55e
    Audio,      // orange #f97316
    File,       // gray #6b7280
    Embeddings, // purple #a855f7
    Trigger,    // red #ef4444
}

/// A port on a node with rendering offset calculated
#[derive(Clone, Debug, Hash)]
pub struct Port {
    pub name: String,
    pub port_type: PortType,
    pub direction: PortDirection,
}

/// Port with vertical offset for rendering
#[derive(Clone, Debug)]
pub struct PortWithOffset {
    pub port: Port,
    pub top_offset: f64, // Percentage 0.0 to 1.0
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
    pub status: NodeStatus,
    pub variant: NodeVariant,
    pub ports: Vec<Port>,
}

/// Represents a persistent wire connection between two nodes
#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub id: u32,
    pub source_node_id: u32,
    pub source_port_name: String,
    pub target_node_id: u32,
    pub target_port_name: String,
    pub selected: bool,
}

/// Tracks an in-progress wire being dragged from a port
#[derive(Clone, Debug)]
pub struct DraggingConnection {
    pub source_node_id: u32,
    pub source_port_name: String,
    pub source_input_node_id: Option<u32>, // Input node we picked up from (for reroute)
    pub current_x: f64,
    pub current_y: f64,
    pub is_dragging: bool,
}

/// Returns default ports for a given node_type string
pub fn default_ports_for_type(node_type: &str) -> Vec<Port> {
    match node_type {
        "user_input" => vec![
            Port { name: "trigger".into(), port_type: PortType::Trigger, direction: PortDirection::In },
            Port { name: "output".into(), port_type: PortType::Text, direction: PortDirection::Out },
        ],
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

    // TODO: Add dynamic output ports for Loop based on iterations if needed

    ports
}

/// Fixed port spacing in pixels (distance between consecutive ports)
const PORT_SPACING: f64 = 25.0;

/// First port offset from node top (pixels)
const FIRST_PORT_OFFSET: f64 = 50.0;

/// Compute vertical offsets for ports, stacking them vertically using fixed pixel positions
pub fn compute_port_offsets(ports: &[Port]) -> Vec<PortWithOffset> {
    let in_ports: Vec<_> = ports.iter().filter(|p| p.direction == PortDirection::In).collect();
    let out_ports: Vec<_> = ports.iter().filter(|p| p.direction == PortDirection::Out).collect();

    let mut result = Vec::new();

    // Input ports: stack from FIRST_PORT_OFFSET downward with fixed pixel spacing
    for (i, port) in in_ports.iter().enumerate() {
        let offset = FIRST_PORT_OFFSET + (i as f64 * PORT_SPACING);
        result.push(PortWithOffset {
            port: (*port).clone(),
            top_offset: offset,
        });
    }

    // Output ports: stack from FIRST_PORT_OFFSET downward (same spacing as inputs)
    for (i, port) in out_ports.iter().enumerate() {
        let offset = FIRST_PORT_OFFSET + (i as f64 * PORT_SPACING);
        result.push(PortWithOffset {
            port: (*port).clone(),
            top_offset: offset,
        });
    }

    result
}

/// Reference node width (px) for port position calculations
pub const REFERENCE_NODE_WIDTH: f64 = 160.0;

/// Port element radius (half of 10px width/height)
const PORT_RADIUS: f64 = 5.0;

/// Compute the canvas (x, y) position of a port given node position and top_offset
/// top_offset is now a direct pixel value from node top edge
pub fn get_port_canvas_position(
    node_x: f64,
    node_y: f64,
    direction: PortDirection,
    top_offset: f64,
) -> (f64, f64) {
    let x = match direction {
        PortDirection::In => node_x - PORT_RADIUS,
        PortDirection::Out => node_x + REFERENCE_NODE_WIDTH - PORT_RADIUS,
    };
    // top_offset is already a pixel value from node top, add PORT_RADIUS for center
    let y = node_y + top_offset + PORT_RADIUS;
    (x, y)
}

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