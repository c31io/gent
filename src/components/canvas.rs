use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::UnwrapThrowExt;

use crate::components::nodes::node::GraphNode;

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

    // Node dragging state
    let (dragging_node_id, set_dragging_node_id) = signal(Option::<u32>::None);
    let (drag_offset_x, set_drag_offset_x) = signal(0.0f64);
    let (drag_offset_y, set_drag_offset_y) = signal(0.0f64);

    // Connection state
    let (connections, set_connections) = signal(Vec::<ConnectionState>::new());
    let (dragging_connection, set_dragging_connection) = signal(Option::<DraggingConnection>::None);
    let (next_connection_id, set_next_connection_id) = signal(1u32);
    let (rerouting_from, set_rerouting_from) = signal(Option::<u32>::None); // Input node being rerouted from

    // Sample nodes for demonstration
    let (nodes, set_nodes) = signal(vec![
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
            selected: false,
        },
    ]);

    // Selection state
    let (selected_node_id, set_selected_node_id) = signal(Option::<u32>::None);

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
            source_input_node_id: None, // Not a reroute
            current_x: sx,
            current_y: sy,
            is_dragging: false, // Not dragging yet - will be set to true on mouse movement
        }));
    };

    let handle_input_drag_end = move |node_id: u32, _x: f64, _y: f64| {
        if let Some(dc) = dragging_connection.get() {
            // Only connect if target is different from source
            if dc.source_node_id != node_id {
                // If rerouting, remove OLD (dimmed) connection first
                if let Some(src_input) = dc.source_input_node_id {
                    set_connections.update(|c| c.retain(|conn|
                        !(conn.source_node_id == dc.source_node_id && conn.target_node_id == src_input)
                    ));
                }
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
            // Same target = no-op, do nothing
        }
        set_dragging_connection.set(None);
        set_rerouting_from.set(None);
    };

    // Handle click on input port - remove connection to this node
    let handle_input_click: Callback<(u32,)> = Callback::new(move |args: (u32,)| {
        set_connections.update(|c| c.retain(|conn| conn.target_node_id != args.0));
    });

    // Handle reroute start from input port - pick up existing wire
    let handle_input_reroute_start: Callback<(u32,)> = Callback::new(move |args: (u32,)| {
        // Find the source node that this input is connected from
        let source_node_id = connections.get()
            .iter()
            .find(|c| c.target_node_id == args.0)
            .map(|c| c.source_node_id);

        if let Some(src_id) = source_node_id {
            let (sx, sy) = get_port_center(src_id, "output");

            // Mark this input as being rerouted from (dims the original connection)
            set_rerouting_from.set(Some(args.0));

            set_dragging_connection.set(Some(DraggingConnection {
                source_node_id: src_id,
                source_input_node_id: Some(args.0), // Mark as reroute
                current_x: sx,
                current_y: sy,
                is_dragging: false, // Will be set to true on first mouse movement
            }));
        }
    });

    // Cancel connection drag (used when click is detected on input port)
    let cancel_connection_drag: Callback<(), ()> = Callback::new(move |_args: ()| {
        set_dragging_connection.set(None);
        set_rerouting_from.set(None);
    });

    // Pan handling
    let handle_mouse_down = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            // Ignore if mouse is over ANY port - let the port handler deal with it
            if is_port(&ev) {
                return;
            }

            // Check if this is a node body mousedown (not port) - start node drag
            if let Some(node_id) = get_node_id_from_event(&ev) {
                let canvas_offset_x = 264.0;
                let canvas_offset_y = 0.0;
                let pan = pan_x.get();
                let pan_y_val = pan_y.get();
                let zoom_val = zoom.get();

                let canvas_x = (ev.client_x() as f64 - canvas_offset_x - pan) / zoom_val;
                let canvas_y = (ev.client_y() as f64 - canvas_offset_y - pan_y_val) / zoom_val;

                // Select this node
                set_selected_node_id.set(Some(node_id));

                // Find node's current position
                let nodes_snapshot = nodes.get();
                if let Some(node) = nodes_snapshot.iter().find(|n| n.id == node_id) {
                    set_drag_offset_x.set(canvas_x - node.x);
                    set_drag_offset_y.set(canvas_y - node.y);
                    set_dragging_node_id.set(Some(node_id));
                    set_is_panning.set(false);
                    return;
                }
            } else {
                // Clicked on empty canvas - clear selection
                set_selected_node_id.set(None);
            }

            // Cancel only if an actual drag is in progress (not just started)
            // This prevents output port clicks from being cancelled when they bubble here
            if let Some(dc) = dragging_connection.get() {
                if dc.is_dragging {
                    set_dragging_connection.set(None);
                    return;
                }
            }
            set_is_panning.set(true);
            set_last_mouse_x.set(ev.client_x() as f64);
            set_last_mouse_y.set(ev.client_y() as f64);
        }
    };

    let handle_mouse_move = move |ev: web_sys::MouseEvent| {
        // Handle node dragging
        if let Some(node_id) = dragging_node_id.get() {
            let canvas_offset_x = 264.0;
            let canvas_offset_y = 0.0;
            let pan = pan_x.get();
            let pan_y_val = pan_y.get();
            let zoom_val = zoom.get();

            let canvas_x = (ev.client_x() as f64 - canvas_offset_x - pan) / zoom_val;
            let canvas_y = (ev.client_y() as f64 - canvas_offset_y - pan_y_val) / zoom_val;

            let new_x = canvas_x - drag_offset_x.get();
            let new_y = canvas_y - drag_offset_y.get();

            set_nodes.update(|nodes: &mut Vec<NodeState>| {
                if let Some(node) = nodes.iter_mut().find(|n| n.id == node_id) {
                    node.x = new_x;
                    node.y = new_y;
                }
            });
            return; // Don't process panning while dragging node
        }

        // Check if we need to start a connection drag (first movement after output click)
        let is_dragging = dragging_connection.get().map(|dc| dc.is_dragging).unwrap_or(false);

        if !is_dragging && dragging_connection.get().is_some() {
            // First movement - this is an actual drag, not a pan
            set_is_panning.set(false);
            set_dragging_connection.update(|d| {
                if let Some(ref mut d) = d {
                    d.is_dragging = true;
                }
            });
        }

        // Handle panning if still active
        if is_panning.get() {
            let dx = ev.client_x() as f64 - last_mouse_x.get();
            let dy = ev.client_y() as f64 - last_mouse_y.get();
            set_last_mouse_x.set(ev.client_x() as f64);
            set_last_mouse_y.set(ev.client_y() as f64);
            set_pan_x.update(|x| *x += dx);
            set_pan_y.update(|y| *y += dy);
        }

        // Update connection preview if dragging
        if dragging_connection.get().is_some() && dragging_connection.get().unwrap().is_dragging {
            let canvas_offset_x = 264.0;
            let canvas_offset_y = 0.0;
            let pan = pan_x.get();
            let pan_y_val = pan_y.get();
            let zoom_val = zoom.get();

            let canvas_x = (ev.client_x() as f64 - canvas_offset_x - pan) / zoom_val;
            let canvas_y = (ev.client_y() as f64 - canvas_offset_y - pan_y_val) / zoom_val;

            set_dragging_connection.update(|dc| {
                if let Some(ref mut d) = dc {
                    d.current_x = canvas_x;
                    d.current_y = canvas_y;
                }
            });
        }
    };

    let handle_mouse_up = move |ev: web_sys::MouseEvent| {
        set_dragging_node_id.set(None);
        set_is_panning.set(false);
        // Only cancel drag if we're NOT over an input port
        // If we ARE over an input port, let its handler complete the connection
        if let Some(dc) = dragging_connection.get() {
            if dc.is_dragging {
                let target = find_input_port_at(ev.client_x() as f64, ev.client_y() as f64);
                if target.is_none() {
                    // Dropped on empty - remove the connection being rerouted
                    if let Some(src_input) = dc.source_input_node_id {
                        set_connections.update(|c| c.retain(|conn|
                            !(conn.source_node_id == dc.source_node_id && conn.target_node_id == src_input)
                        ));
                    }
                    set_dragging_connection.set(None);
                    set_rerouting_from.set(None);
                }
                // If over input port, leave dragging_connection for node's handler to complete
            }
        }
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

    // Set up canvas animation loop using Effect
    // This runs after mount and redraws when signals change
    Effect::new(move |_| {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return,
        };
        let canvas_elem = match document.get_element_by_id("wires-canvas") {
            Some(e) => e,
            None => return,
        };
        let canvas_ref: web_sys::HtmlCanvasElement = match canvas_elem.dyn_into() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Set canvas size to match container's actual pixel dimensions
        if let Some(container) = canvas_ref.parent_element() {
            let container: web_sys::HtmlElement = match container.dyn_into() {
                Ok(c) => c,
                Err(_) => return,
            };
            let width = container.client_width() as u32;
            let height = container.client_height() as u32;
            canvas_ref.set_width(width);
            canvas_ref.set_height(height);
        }

        let ctx: web_sys::CanvasRenderingContext2d = match canvas_ref.get_context("2d") {
            Ok(Some(c)) => c.unchecked_into(),
            _ => return,
        };

        // Draw current state
        let connections = connections.get();
        let dragging = dragging_connection.get();
        let rerouting = rerouting_from.get();
        let nodes = nodes.get();
        draw_connections(
            &ctx,
            &connections,
            &dragging,
            rerouting,
            &nodes,
            pan_x.get(),
            pan_y.get(),
            zoom.get(),
        );
    });

    view! {
        <div
            class="canvas-container"
            on:mousedown=handle_mouse_down
            on:mousemove=handle_mouse_move
            on:mouseup=handle_mouse_up
            on:mouseleave=handle_mouse_up
            on:wheel=handle_wheel
        >
            {/* Wires canvas layer - OUTSIDE transformed div so we control transform via ctx */}
            <canvas
                id="wires-canvas"
                style:position="absolute"
                style:top="0"
                style:left="0"
                style:width="100%"
                style:height="100%"
                style:pointer-events="none"
                style:z_index="0"
            ></canvas>

            {/* Transformed canvas area with nodes */}
            <div
                class="canvas"
                style:transform=transform_style
            >
                {/* Grid background pattern would go here */}
                {move || {
                    let connections_snapshot = connections.get();
                    let selected = selected_node_id.get();
                    nodes.get().iter().map(|node| {
                        let has_connection = connections_snapshot.iter().any(|c| c.target_node_id == node.id);
                        let is_selected = selected == Some(node.id);
                        view! {
                            <GraphNode
                                x=node.x
                                y=node.y
                                label={node.label.clone()}
                                selected={is_selected}
                                node_id={node.id}
                                has_input_connection={has_connection}
                                on_output_drag_start={Some(Callback::from(handle_output_drag_start))}
                                on_input_drag_end={Some(Callback::from(handle_input_drag_end))}
                                on_input_click={Some(handle_input_click)}
                                on_input_reroute_start={Some(Callback::from(handle_input_reroute_start))}
                                cancel_connection_drag={Some(cancel_connection_drag)}
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
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
    pub source_input_node_id: Option<u32>, // Input node we picked up from (for reroute)
    pub current_x: f64,
    pub current_y: f64,
    pub is_dragging: bool,
}

/// Find input port element at given viewport coordinates
fn find_input_port_at(x: f64, y: f64) -> Option<u32> {
    let doc = web_sys::window()?.document()?;
    let element = doc.element_from_point(x as f32, y as f32)?;
    let port_type = element.get_attribute("data-port")?;
    if port_type != "input" {
        return None;
    }
    let node_id = element.get_attribute("data-node-id")?.parse().ok()?;
    Some(node_id)
}

/// Check if the mouse event target is an input port
fn is_input_port(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            if let Some(port_type) = element.get_attribute("data-port") {
                return port_type == "input";
            }
        }
    }
    false
}

/// Check if the mouse event target is any port (input or output)
fn is_port(ev: &web_sys::MouseEvent) -> bool {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            return element.get_attribute("data-port").is_some();
        }
    }
    false
}

/// Get node_id from mouse event - traverses up to find the node div
fn get_node_id_from_event(ev: &web_sys::MouseEvent) -> Option<u32> {
    if let Some(target) = ev.target() {
        if let Ok(element) = target.dyn_into::<web_sys::Element>() {
            // Walk up the DOM tree to find the node div
            let mut current: Option<web_sys::Element> = Some(element);
            while let Some(el) = current {
                // If this element has data-node-id AND it's not a port, it's a node
                if el.get_attribute("data-node-id").is_some()
                    && el.get_attribute("data-port").is_none()
                {
                    return el.get_attribute("data-node-id")?.parse().ok();
                }
                // Move to parent element
                current = el.parent_element();
            }
        }
    }
    None
}

/// Draw a bezier wire on the canvas context
fn draw_bezier(
    ctx: &web_sys::CanvasRenderingContext2d,
    sx: f64,
    sy: f64,
    ex: f64,
    ey: f64,
    selected: bool,
    dimmed: bool,
) {
    let mid_x = (sx + ex) / 2.0;
    ctx.begin_path();
    ctx.move_to(sx, sy);
    ctx.bezier_curve_to(mid_x, sy, mid_x, ey, ex, ey);
    let color = if selected {
        "#6366f1"
    } else if dimmed {
        "#505050"
    } else {
        "#a0a0a0"
    };
    #[allow(deprecated)]
    ctx.set_stroke_style(&JsValue::from_str(color));
    ctx.set_line_width(2.0);
    ctx.stroke();
}

/// Get port center position from a nodes slice (non-reactive version)
fn get_port_center_static(node_id: u32, port_type: &str, nodes: &[NodeState]) -> (f64, f64) {
    if let Some(node) = nodes.iter().find(|n| n.id == node_id) {
        let port_offset_x = if port_type == "output" { 150.0 } else { 0.0 };
        let port_offset_y = 35.0;
        let x = node.x + port_offset_x;
        let y = node.y + port_offset_y;
        (x, y)
    } else {
        (0.0, 0.0)
    }
}

/// Draw all connections on the canvas
fn draw_connections(
    ctx: &web_sys::CanvasRenderingContext2d,
    connections: &[ConnectionState],
    dragging: &Option<DraggingConnection>,
    rerouting_from: Option<u32>,
    nodes: &[NodeState],
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
) {
    let canvas = ctx.canvas().unwrap();
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    ctx.clear_rect(0.0, 0.0, width, height);

    // Apply transform for pan/zoom
    ctx.set_transform(zoom, 0.0, 0.0, zoom, pan_x, pan_y).unwrap_throw();

    // Draw established connections
    for conn in connections {
        let (sx, sy) = get_port_center_static(conn.source_node_id, "output", nodes);
        let (ex, ey) = get_port_center_static(conn.target_node_id, "input", nodes);
        let dimmed = rerouting_from == Some(conn.target_node_id);
        draw_bezier(ctx, sx, sy, ex, ey, conn.selected, dimmed);
    }

    // Draw preview connection while dragging
    if let Some(ref dc) = dragging {
        if dc.is_dragging {
            let (sx, sy) = get_port_center_static(dc.source_node_id, "output", nodes);
            draw_bezier(ctx, sx, sy, dc.current_x, dc.current_y, false, false);
        }
    }

    // Reset transform
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap_throw();
}
