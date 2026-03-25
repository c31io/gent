use leptos::prelude::*;

use crate::components::nodes::node::GraphNode;
use crate::components::nodes::connection::Connection;

/// Canvas for rendering nodes with pan/zoom
#[component]
pub fn Canvas() -> impl IntoView {
    // Canvas transform state
    let (zoom, set_zoom) = signal(1.0f64);
    let (pan_x, set_pan_x) = signal(0.0f64);
    let (pan_y, set_pan_y) = signal(0.0f64);

    // Track dragging state
    let (is_panning, set_is_panning) = signal(false);
    let (last_mouse_x, set_last_mouse_x) = signal(0.0f64);
    let (last_mouse_y, set_last_mouse_y) = signal(0.0f64);

    // Connection state
    let (connections, set_connections) = signal(Vec::<ConnectionState>::new());
    let (dragging_connection, set_dragging_connection) = signal(Option::<DraggingConnection>::None);
    let (next_connection_id, set_next_connection_id) = signal(1u32);

    // Sample nodes for demonstration
    let (nodes, _set_nodes) = signal(vec![
        NodeState {
            id: 1,
            x: 100.0,
            y: 100.0,
            node_type: "user_input".to_string(),
            label: "User Input".to_string(),
            selected: false,
        },
        NodeState {
            id: 2,
            x: 350.0,
            y: 80.0,
            node_type: "template".to_string(),
            label: "Template".to_string(),
            selected: false,
        },
        NodeState {
            id: 3,
            x: 350.0,
            y: 220.0,
            node_type: "retrieval".to_string(),
            label: "Retrieval".to_string(),
            selected: true,
        },
    ]);

    // Zoom controls
    let zoom_in = move |_| {
        let current = zoom.get();
        if current < 4.0 {
            set_zoom.set(current + 0.1);
        }
    };

    let zoom_out = move |_| {
        let current = zoom.get();
        if current > 0.25 {
            set_zoom.set(current - 0.1);
        }
    };

    let reset_zoom = move |_| {
        set_zoom.set(1.0);
        set_pan_x.set(0.0);
        set_pan_y.set(0.0);
    };

    // Get port center position relative to canvas
    let get_port_center = move |node_id: u32, port_type: &str| -> (f64, f64) {
        let nodes_snapshot = nodes.get();
        if let Some(node) = nodes_snapshot.iter().find(|n| n.id == node_id) {
            let port_offset_x = if port_type == "output" { 150.0 } else { 0.0 };
            let port_offset_y = 35.0; // Center of full node (header ~37px + body ~42px)
            let x = node.x + port_offset_x;
            let y = node.y + port_offset_y;
            (x, y)
        } else {
            (0.0, 0.0)
        }
    };

    // Port drag handlers
    let handle_output_drag_start = move |node_id: u32, _mouse_x: f64, _mouse_y: f64| {
        let (sx, sy) = get_port_center(node_id, "output");
        set_dragging_connection.set(Some(DraggingConnection {
            source_node_id: node_id,
            current_x: sx,
            current_y: sy,
        }));
    };

    let handle_input_drag_end = move |node_id: u32, _x: f64, _y: f64| {
        if let Some(dc) = dragging_connection.get() {
            // Only connect if target is different from source
            if dc.source_node_id != node_id {
                // Remove any existing connection TO this input port first
                set_connections.update(|c| c.retain(|conn| conn.target_node_id != node_id));
                let new_conn = ConnectionState {
                    id: next_connection_id.get(),
                    source_node_id: dc.source_node_id,
                    target_node_id: node_id,
                    selected: false,
                };
                set_connections.update(|c| c.push(new_conn));
                set_next_connection_id.update(|n| *n += 1);
            }
        }
        set_dragging_connection.set(None);
    };

    // Handle click on input port - remove connection to this node
    let handle_input_click: Callback<(u32,)> = Callback::new(move |args: (u32,)| {
        set_connections.update(|c| c.retain(|conn| conn.target_node_id != args.0));
    });

    // Pan handling
    let handle_mouse_down = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            // Check if we're dragging a connection - if so, don't start panning
            if dragging_connection.get().is_some() {
                return;
            }
            set_is_panning.set(true);
            set_last_mouse_x.set(ev.client_x() as f64);
            set_last_mouse_y.set(ev.client_y() as f64);
        }
    };

    let handle_mouse_move = move |ev: web_sys::MouseEvent| {
        if is_panning.get() {
            let dx = ev.client_x() as f64 - last_mouse_x.get();
            let dy = ev.client_y() as f64 - last_mouse_y.get();
            set_last_mouse_x.set(ev.client_x() as f64);
            set_last_mouse_y.set(ev.client_y() as f64);
            set_pan_x.update(|x| *x += dx);
            set_pan_y.update(|y| *y += dy);
        }
        if dragging_connection.get().is_some() {
            // Update the preview connection endpoint
            // Convert screen coords to canvas SVG coords
            // Canvas container is offset by ~264px (left panel + divider)
            let canvas_offset = 264.0; // Approximate - left panel + divider
            let pan = pan_x.get();
            let pan_y_val = pan_y.get();
            let zoom_val = zoom.get();
            set_dragging_connection.update(|dc| {
                if let Some(ref mut d) = dc {
                    let canvas_x = (ev.client_x() as f64 - canvas_offset - pan) / zoom_val;
                    let canvas_y = (ev.client_y() as f64 - pan_y_val) / zoom_val;
                    d.current_x = canvas_x;
                    d.current_y = canvas_y;
                }
            });
        }
    };

    let handle_mouse_up = move |_ev: web_sys::MouseEvent| {
        set_is_panning.set(false);
        // Cancel any in-progress connection drag
        set_dragging_connection.set(None);
    };

    // Scroll to zoom
    let handle_wheel = move |ev: web_sys::WheelEvent| {
        ev.prevent_default();
        let delta = ev.delta_y();
        let current = zoom.get();
        let new_zoom = if delta < 0.0 {
            (current + 0.1).min(4.0)
        } else {
            (current - 0.1).max(0.25)
        };
        set_zoom.set(new_zoom);
    };

    let zoom_percent = move || format!("{}%", (zoom.get() * 100.0).round() as i32);

    let transform_style = move || {
        format!(
            "translate({}px, {}px) scale({})",
            pan_x.get(),
            pan_y.get(),
            zoom.get()
        )
    };

    view! {
        <div
            class="canvas-container"
            on:mousedown=handle_mouse_down
            on:mousemove=handle_mouse_move
            on:mouseup=handle_mouse_up
            on:mouseleave=handle_mouse_up
            on:wheel=handle_wheel
        >
            <div
                class="canvas"
                style:transform=transform_style
            >
                {/* Connections SVG layer */}
                <svg class="connections-svg" style:position="absolute" style:top="0" style:left="0" style:width="100%" style:height="100%" style:pointer-events="none" style:z_index="0">
                    {/* Render established connections */}
                    {move || connections.get().iter().map(|conn| {
                        let (sx, sy) = get_port_center(conn.source_node_id, "output");
                        let (ex, ey) = get_port_center(conn.target_node_id, "input");
                        view! {
                            <Connection start_x={sx} start_y={sy} end_x={ex} end_y={ey} selected={conn.selected} />
                        }
                    }).collect::<Vec<_>>()}

                    {/* Render preview connection while dragging */}
                    {move || dragging_connection.get().map(|dc| {
                        let (sx, sy) = get_port_center(dc.source_node_id, "output");
                        view! {
                            <Connection start_x={sx} start_y={sy} end_x={dc.current_x} end_y={dc.current_y} selected={false} />
                        }
                    })}
                </svg>

                {/* Grid background pattern would go here */}
                {move || nodes.get().iter().map(|node| {
                    view! {
                        <GraphNode
                            x=node.x
                            y=node.y
                            label={node.label.clone()}
                            selected={node.selected}
                            node_id={node.id}
                            on_output_drag_start={Some(Callback::from(handle_output_drag_start))}
                            on_input_drag_end={Some(Callback::from(handle_input_drag_end))}
                            on_input_click={Some(handle_input_click)}
                        />
                    }
                }).collect::<Vec<_>>()}
            </div>

            {/* Zoom Controls */}
            <div class="zoom-controls">
                <button class="zoom-btn" on:click=zoom_out title="Zoom out">"-"</button>
                <span class="zoom-level">{zoom_percent}</span>
                <button class="zoom-btn" on:click=zoom_in title="Zoom in">"+"</button>
                <button class="zoom-btn" on:click=reset_zoom title="Reset view">"⟲"</button>
            </div>
        </div>
    }
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
    pub current_x: f64,
    pub current_y: f64,
}
