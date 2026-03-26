use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

use crate::components::canvas::state::{ConnectionState, NodeState, NodeStatus};
use crate::components::canvas::Canvas;
use crate::components::execution_engine::{ExecutionState, get_downstream_nodes};
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
        },
        NodeState {
            id: 2,
            x: 300.0,
            y: 150.0,
            node_type: "planner_agent".to_string(),
            label: "Planner Agent".to_string(),
            selected: false,
            status: NodeStatus::Pending,
        },
        NodeState {
            id: 3,
            x: 520.0,
            y: 150.0,
            node_type: "web_search".to_string(),
            label: "Web Search".to_string(),
            selected: false,
            status: NodeStatus::Pending,
        },
    ]);

    let (connections, set_connections) = signal(vec![
        ConnectionState {
            id: 1,
            source_node_id: 1,
            target_node_id: 2,
            selected: false,
        },
        ConnectionState {
            id: 2,
            source_node_id: 2,
            target_node_id: 3,
            selected: false,
        },
    ]);
    let (selected_node_id, set_selected_node_id) = signal(Option::<u32>::None);
    let (deleting_node_id, set_deleting_node_id) = signal(Option::<u32>::None);
    let (next_node_id, set_next_node_id) = signal(4u32);

    // Drag preview state
    let (dragging_node_type, set_dragging_node_type) = signal(Option::<String>::None);
    let (drag_x, set_drag_x) = signal(0.0);
    let (drag_y, set_drag_y) = signal(0.0);

    // Execution state for the execution engine
    let (execution_state, set_execution_state) = signal(ExecutionState::new());

    // Handle trigger node execution
    let handle_trigger = move |node_id: u32| {
        let nodes_snapshot = nodes.get();
        let connections_snapshot = connections.get();

        // Find the trigger node
        if let Some(_trigger_node) = nodes_snapshot.iter().find(|n| n.id == node_id && n.node_type == "trigger") {
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
        };

        set_nodes.update(|nodes: &mut Vec<NodeState>| nodes.push(new_node));
        set_next_node_id.update(|n| *n += 1);
        set_selected_node_id.set(Some(node_id));
    };

    // Delete node handler - animates shrink then removes node
    let delete_node = move |node_id: u32| {
        // First unselect to prevent flash to next node
        set_selected_node_id.set(None);

        // Then set deleting node to trigger shrink animation
        set_deleting_node_id.set(Some(node_id));

        // After animation completes, remove the node
        let nodes_to_delete = node_id;
        let connections_to_delete = node_id;

        spawn_local(async move {
            TimeoutFuture::new(300).await;
            set_nodes.update(|n: &mut Vec<NodeState>| n.retain(|node| node.id != nodes_to_delete));
            set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn| {
                conn.source_node_id != connections_to_delete && conn.target_node_id != connections_to_delete
            }));
            set_deleting_node_id.set(None);
        });
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
                    on_trigger={Some(Callback::new(handle_trigger))}
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
                nodes={nodes.into()}
                selected_node_id={selected_node_id.into()}
                on_delete={Callback::from(delete_node)}
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
