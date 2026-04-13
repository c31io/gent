use std::collections::HashSet;

use crate::components::canvas::state::{ConnectionState, NodeState};

/// A full snapshot of undoable graph state.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphSnapshot {
    pub nodes: Vec<NodeState>,
    pub connections: Vec<ConnectionState>,
    pub selected_node_ids: HashSet<u32>,
    pub next_node_id: u32,
    pub next_connection_id: u32,
}

pub struct UndoManager {
    undo_stack: Vec<GraphSnapshot>,
    redo_stack: Vec<GraphSnapshot>,
    max_size: usize,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 50,
        }
    }

    pub fn push(&mut self, snapshot: GraphSnapshot) {
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, current: GraphSnapshot) -> Option<GraphSnapshot> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: GraphSnapshot) -> Option<GraphSnapshot> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}
