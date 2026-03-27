use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use std::collections::HashMap;

use crate::components::canvas::state::{ConnectionState, NodeState, NodeStatus, default_ports_for_type, default_variant_for_type};
use crate::components::canvas::Canvas;
use crate::components::execution_engine::ExecutionState;
use crate::components::execution_trace::ExecutionTrace;
use crate::components::left_panel::{LeftPanel, NODE_TYPES};
use crate::components::node_inspector::NodeInspector;

/// Main application layout with left panel, canvas, and right panel
#[component]
pub fn AppLayout() -> impl IntoView {
    // Shared state for panel sizes (in pixels)
    let (left_width, set_left_width) = signal(260i32);
    let (right_width, set_right_width) = signal(300i32);

    // Track if dragging divider
    let (dragging_left, set_dragging_left) = signal(false);
    let (dragging_right, set_dragging_right) = signal(false);

    // Lifted state from Canvas for NodeInspector
    let (nodes, set_nodes) = signal(vec![
        NodeState {
            id: 1,
            x: 80.0,
            y: 150.0,
            node_type: "trigger".to_string(),
            label: "Trigger".to_string(),
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type("trigger"),
            ports: default_ports_for_type("trigger"),
        },
        NodeState {
            id: 2,
            x: 300.0,
            y: 150.0,
            node_type: "user_input".to_string(),
            label: "Text Input".to_string(),
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type("user_input"),
            ports: default_ports_for_type("user_input"),
        },
        NodeState {
            id: 3,
            x: 520.0,
            y: 150.0,
            node_type: "chat_output".to_string(),
            label: "Text Output".to_string(),
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type("chat_output"),
            ports: default_ports_for_type("chat_output"),
        },
    ]);

    let (connections, set_connections) = signal(vec![
        ConnectionState {
            id: 1,
            source_node_id: 1,
            source_port_name: "output".to_string(),
            target_node_id: 2,
            target_port_name: "trigger".to_string(),
            selected: false,
        },
        ConnectionState {
            id: 2,
            source_node_id: 2,
            source_port_name: "output".to_string(),
            target_node_id: 3,
            target_port_name: "response".to_string(),
            selected: false,
        },
    ]);
    let (selected_node_id, set_selected_node_id) = signal(Option::<u32>::None);
    let (deleting_node_id, set_deleting_node_id) = signal(Option::<u32>::None);
    let (next_node_id, set_next_node_id) = signal(4u32);

    // Inspector state for selected node
    let (inspector_node, set_inspector_node) = signal(Option::<NodeState>::None);

    // Drag preview state
    let (dragging_node_type, set_dragging_node_type) = signal(Option::<String>::None);
    let (drag_x, set_drag_x) = signal(0.0);
    let (drag_y, set_drag_y) = signal(0.0);

    // Execution state for the execution engine
    let (execution_state, set_execution_state) = signal(ExecutionState::new());

    // Handler for text input changes
    let handle_text_change = move |node_id: u32, new_text: String| {
        set_nodes.update(|nodes: &mut Vec<NodeState>| {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == node_id) {
                if let crate::components::canvas::state::NodeVariant::UserInput { text } = &mut node.variant {
                    *text = new_text;
                }
            }
        });
    };

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

    // Handle trigger node execution
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

        // Extract just the node IDs in order for upstream computation
        let exec_order_ids: Vec<u32> = execution_plan.iter().map(|(id, _)| *id).collect();

        for (exec_idx, (exec_node_id, _upstream)) in execution_plan.into_iter().enumerate() {
            // Compute actual upstream results from previously executed nodes
            let mut upstream: HashMap<u32, String> = HashMap::new();
            for prev_exec_node_id in exec_order_ids.iter().take(exec_idx) {
                if let Some(result) = node_results.get(prev_exec_node_id) {
                    upstream.insert(*prev_exec_node_id, result.clone());
                }
            }

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

    // Callback to start palette drag
    let on_palette_drag_start: Callback<String> = Callback::new(move |node_type: String| {
        set_dragging_node_type.set(Some(node_type));
    });

    // Handle node drop from palette
    let handle_node_drop = move |node_type: String, x: f64, y: f64| {
        let node_id = next_node_id.get();
        let label = NODE_TYPES
            .iter()
            .find(|n| n.id == node_type)
            .map(|n| n.name)
            .unwrap_or(&node_type)
            .to_string();

        let new_node = NodeState {
            id: node_id,
            x: x - 75.0,
            y: y - 50.0,
            node_type: node_type.clone(),
            label,
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type(&node_type),
            ports: default_ports_for_type(&node_type),
        };

        set_nodes.update(|nodes: &mut Vec<NodeState>| nodes.push(new_node));
        set_next_node_id.update(|n| *n += 1);
        set_selected_node_id.set(Some(node_id));
    };

    let handle_left_divider_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_dragging_left.set(true);
    };

    let handle_right_divider_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_dragging_right.set(true);
    };

    let handle_mouse_up = move |_ev: web_sys::MouseEvent| {
        set_dragging_left.set(false);
        set_dragging_right.set(false);
        // Clear drag preview state
        set_dragging_node_type.set(None);
        // Clear window draggedNodeType to prevent stale state on subsequent clicks
        if let Some(window) = web_sys::window() {
            let _ = js_sys::Reflect::delete_property(&window, &"draggedNodeType".into());
        }
    };

    // Global mouse move for drag preview
    let handle_global_mousemove = move |ev: web_sys::MouseEvent| {
        if dragging_node_type.get().is_some() {
            set_drag_x.set(ev.client_x() as f64);
            set_drag_y.set(ev.client_y() as f64);
        }
        if dragging_left.get() {
            let new_width = ev.client_x();
            if new_width >= 180 && new_width <= 500 {
                set_left_width.set(new_width);
            }
        }
        if dragging_right.get() {
            let window = web_sys::window().unwrap();
            let inner_width = window.inner_width().unwrap().as_f64().unwrap() as i32;
            let new_width = inner_width - ev.client_x();
            if new_width >= 180 && new_width <= 500 {
                set_right_width.set(new_width);
            }
        }
    };

    view! {
        <div
            class="app-layout"
            on:mousemove={handle_global_mousemove}
            on:mouseup={handle_mouse_up}
            on:mouseleave={handle_mouse_up}
        >
            <div class="app-layout-main">
                {/* Left Panel */}
                <div
                    class="panel"
                    style:width=move || format!("{}px", left_width.get())
                >
                    <LeftPanel on_drag_start={Some(on_palette_drag_start)} />
                </div>

                {/* Left Divider */}
                <div
                    class="divider"
                    on:mousedown={handle_left_divider_mouse_down}
                ></div>

                {/* Canvas */}
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
                    right_width={Some(right_width.into())}
                    on_trigger={Some(Callback::new(handle_trigger))}
                    on_text_change={Some(Callback::new(move |(node_id, new_text)| handle_text_change(node_id, new_text)))}
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
                />

                {/* Right Divider */}
                <div
                    class="divider"
                    on:mousedown={handle_right_divider_mouse_down}
                ></div>

                {/* Right Panel */}
                <div
                    class="panel panel-right"
                    style:width=move || format!("{}px", right_width.get())
                >
                    <ExecutionTrace execution={execution_state.into()} />
                </div>
            </div>

            {/* Node Inspector Drawer */}
            <NodeInspector
                selected_node={inspector_node.into()}
                on_node_delete={Some(Callback::new(move |node_id| {
                    // Unselect the node and set deleting_node_id to trigger the shrink animation
                    set_selected_node_id.set(None);
                    set_deleting_node_id.set(Some(node_id));
                    // After animation completes (200ms), remove the node
                    spawn_local(async move {
                        TimeoutFuture::new(200).await;
                        set_nodes.update(|nodes| {
                            nodes.retain(|n| n.id != node_id);
                        });
                        set_connections.update(|conns| {
                            conns.retain(|c| c.source_node_id != node_id && c.target_node_id != node_id);
                        });
                        set_deleting_node_id.set(None);
                    });
                    set_inspector_node.set(None);
                }))}
                on_close={Some(Callback::new(move |_| {
                    set_inspector_node.set(None);
                }))}
            />

            {/* Drag Preview */}
            {move || {
                if let Some(node_type) = dragging_node_type.get() {
                    let label = NODE_TYPES
                        .iter()
                        .find(|n| n.id == node_type)
                        .map(|n| n.name.to_string())
                        .unwrap_or_else(|| node_type.clone());
                    Some(view! {
                        <div
                            class="drag-preview"
                            style:left={format!("{}px", drag_x.get())}
                            style:top={format!("{}px", drag_y.get())}
                        >
                            {label}
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}
