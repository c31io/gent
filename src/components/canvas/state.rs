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
}

/// Represents a persistent wire connection between two nodes
#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub id: u32,
    pub source_node_id: u32,
    pub target_node_id: u32,
    pub selected: bool,
}

/// Tracks an in-progress wire being dragged from a port
#[derive(Clone, Debug)]
pub struct DraggingConnection {
    pub source_node_id: u32,
    pub source_input_node_id: Option<u32>, // Input node we picked up from (for reroute)
    pub current_x: f64,
    pub current_y: f64,
    pub is_dragging: bool,
}
