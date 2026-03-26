use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::canvas::geometry::{find_input_port_at, get_node_id_from_event, is_port, is_trigger_button};
use crate::components::canvas::state::{ConnectionState, DraggingConnection, NodeState, NodeVariant, Port, PortDirection, PortType, default_ports_for_type, default_variant_for_type};
use crate::components::canvas::wires::draw_connections;
use crate::components::nodes::node::GraphNode;

/// Canvas for rendering nodes with pan/zoom
#[component]
pub fn Canvas(
    /// Selected node ID
    selected_node_id: Signal<Option<u32>>,
    set_selected_node_id: WriteSignal<Option<u32>>,
    /// All nodes
    nodes: Signal<Vec<NodeState>>,
    set_nodes: WriteSignal<Vec<NodeState>>,
    /// All connections
    connections: Signal<Vec<ConnectionState>>,
    set_connections: WriteSignal<Vec<ConnectionState>>,
    /// Node ID currently being deleted (for shrink animation)
    #[prop(default = None)] deleting_node_id: Option<Signal<Option<u32>>>,
    /// Callback when node selection changes
    #[prop(default = None)] on_selection_change: Option<Callback<Option<u32>>>,
    /// Callback when a node is dropped from the palette (receives node_type, canvas x, y)
    #[prop(default = None)] on_node_drop: Option<Callback<(String, f64, f64)>>,
    /// Left panel width signal (for calculating canvas offset)
    #[prop(default = None)] left_width: Option<Signal<i32>>,
    /// Callback when trigger node is clicked
    #[prop(default = None)] on_trigger: Option<Callback<u32>>,
) -> impl IntoView {
    // Canvas transform state (local to canvas)
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

    // Connection state (local to canvas - wires are drawn here)
    let (dragging_connection, set_dragging_connection) = signal(Option::<DraggingConnection>::None);
    let (rerouting_from, set_rerouting_from) = signal(Option::<u32>::None);
    let (next_connection_id, set_next_connection_id) = signal(1u32);

    // Get canvas offset (left panel width + divider width)
    let get_canvas_offset_x = move || -> f64 {
        left_width.map(|w| w.get() as f64 + 4.0).unwrap_or(264.0)
    };

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
            let port_offset_y = 35.0;
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
            source_input_node_id: None,
            current_x: sx,
            current_y: sy,
            is_dragging: false,
        }));
    };

    let handle_input_drag_end = move |node_id: u32, _x: f64, _y: f64| {
        if let Some(dc) = dragging_connection.get() {
            if dc.source_node_id != node_id {
                if let Some(src_input) = dc.source_input_node_id {
                    set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn|
                        !(conn.source_node_id == dc.source_node_id && conn.target_node_id == src_input)
                    ));
                }
                set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn| conn.target_node_id != node_id));
                let new_conn = ConnectionState {
                    id: next_connection_id.get(),
                    source_node_id: dc.source_node_id,
                    target_node_id: node_id,
                    selected: false,
                };
                set_connections.update(|c: &mut Vec<ConnectionState>| c.push(new_conn));
                set_next_connection_id.update(|n| *n += 1);
            }
        }
        set_dragging_connection.set(None);
        set_rerouting_from.set(None);
    };

    let handle_input_click: Callback<(u32,)> = Callback::new(move |args: (u32,)| {
        set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn| conn.target_node_id != args.0));
    });

    let handle_input_reroute_start: Callback<(u32,)> = Callback::new(move |args: (u32,)| {
        let source_node_id = connections.get()
            .iter()
            .find(|c| c.target_node_id == args.0)
            .map(|c| c.source_node_id);

        if let Some(src_id) = source_node_id {
            let (sx, sy) = get_port_center(src_id, "output");
            set_rerouting_from.set(Some(args.0));
            set_dragging_connection.set(Some(DraggingConnection {
                source_node_id: src_id,
                source_input_node_id: Some(args.0),
                current_x: sx,
                current_y: sy,
                is_dragging: false,
            }));
        }
    });

    let cancel_connection_drag: Callback<(), ()> = Callback::new(move |_args: ()| {
        set_dragging_connection.set(None);
        set_rerouting_from.set(None);
    });

    // Pan handling
    let handle_mouse_down = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            if is_port(&ev) {
                return;
            }

            if let Some(node_id) = get_node_id_from_event(&ev) {
                let canvas_offset_x = get_canvas_offset_x();
                let canvas_offset_y = 0.0;
                let pan = pan_x.get();
                let pan_y_val = pan_y.get();
                let zoom_val = zoom.get();

                let canvas_x = (ev.client_x() as f64 - canvas_offset_x - pan) / zoom_val;
                let canvas_y = (ev.client_y() as f64 - canvas_offset_y - pan_y_val) / zoom_val;

                set_selected_node_id.set(Some(node_id));
                if let Some(callback) = on_selection_change {
                    callback.run(Some(node_id));
                }

                // Check if this is a trigger button click - if so, don't drag (button handles trigger)
                if is_trigger_button(&ev) {
                    return;
                }

                let nodes_snapshot = nodes.get();
                if let Some(node) = nodes_snapshot.iter().find(|n| n.id == node_id) {
                    set_drag_offset_x.set(canvas_x - node.x);
                    set_drag_offset_y.set(canvas_y - node.y);
                    set_dragging_node_id.set(Some(node_id));
                    set_is_panning.set(false);
                    return;
                }
            } else {
                set_selected_node_id.set(None);
                if let Some(callback) = on_selection_change {
                    callback.run(None);
                }
            }

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
        if let Some(node_id) = dragging_node_id.get() {
            let canvas_offset_x = get_canvas_offset_x();
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
            return;
        }

        let is_dragging = dragging_connection.get().map(|dc| dc.is_dragging).unwrap_or(false);

        if !is_dragging && dragging_connection.get().is_some() {
            set_is_panning.set(false);
            set_dragging_connection.update(|d| {
                if let Some(ref mut d) = d {
                    d.is_dragging = true;
                }
            });
        }

        if is_panning.get() {
            let dx = ev.client_x() as f64 - last_mouse_x.get();
            let dy = ev.client_y() as f64 - last_mouse_y.get();
            set_last_mouse_x.set(ev.client_x() as f64);
            set_last_mouse_y.set(ev.client_y() as f64);
            set_pan_x.update(|x| *x += dx);
            set_pan_y.update(|y| *y += dy);
        }

        if dragging_connection.get().is_some() && dragging_connection.get().unwrap().is_dragging {
            let canvas_offset_x = get_canvas_offset_x();
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

    // Helper to get dragged node type from window
    let get_dragged_node_type = || -> Option<String> {
        let window = web_sys::window()?;
        let val = js_sys::Reflect::get(&window, &"draggedNodeType".into()).ok()?;
        if val.is_undefined() || val.is_null() {
            None
        } else {
            val.as_string()
        }
    };

    // Helper to clear dragged node type
    let clear_dragged_node_type = || {
        if let Some(window) = web_sys::window() {
            let _ = js_sys::Reflect::delete_property(&window, &"draggedNodeType".into());
        }
    };

    let handle_mouse_up = move |ev: web_sys::MouseEvent| {
        set_dragging_node_id.set(None);
        set_is_panning.set(false);

        // Check if a palette node is being dropped
        if let Some(callback) = &on_node_drop {
            if let Some(node_type) = get_dragged_node_type() {
                let canvas_offset_x = get_canvas_offset_x();
                let canvas_offset_y = 0.0;
                let pan = pan_x.get();
                let pan_y_val = pan_y.get();
                let zoom_val = zoom.get();

                let canvas_x = (ev.client_x() as f64 - canvas_offset_x - pan) / zoom_val;
                let canvas_y = (ev.client_y() as f64 - canvas_offset_y - pan_y_val) / zoom_val;

                callback.run((node_type, canvas_x, canvas_y));
                clear_dragged_node_type();
            }
        }

        if let Some(dc) = dragging_connection.get() {
            if dc.is_dragging {
                let target = find_input_port_at(ev.client_x() as f64, ev.client_y() as f64);
                if target.is_none() {
                    if let Some(src_input) = dc.source_input_node_id {
                        set_connections.update(|c: &mut Vec<ConnectionState>| c.retain(|conn|
                            !(conn.source_node_id == dc.source_node_id && conn.target_node_id == src_input)
                        ));
                    }
                    set_dragging_connection.set(None);
                    set_rerouting_from.set(None);
                }
            }
        }
    };

    let handle_wheel = move |ev: web_sys::WheelEvent| {
        ev.prevent_default();
        let delta = ev.delta_y();
        let current_zoom = zoom.get();
        let new_zoom = if delta < 0.0 {
            (current_zoom + 0.1).min(4.0)
        } else {
            (current_zoom - 0.1).max(0.25)
        };

        if new_zoom == current_zoom {
            return;
        }

        let canvas_offset_x = get_canvas_offset_x();
        let canvas_offset_y = 0.0;
        let cursor_x = ev.client_x() as f64;
        let cursor_y = ev.client_y() as f64;

        let canvas_x_before = (cursor_x - canvas_offset_x - pan_x.get()) / current_zoom;
        let canvas_y_before = (cursor_y - canvas_offset_y - pan_y.get()) / current_zoom;

        set_zoom.set(new_zoom);

        let new_pan_x = cursor_x - canvas_offset_x - canvas_x_before * new_zoom;
        let new_pan_y = cursor_y - canvas_offset_y - canvas_y_before * new_zoom;

        set_pan_x.set(new_pan_x);
        set_pan_y.set(new_pan_y);
    };

    let handle_canvas_dblclick = move |ev: web_sys::MouseEvent| {
        if get_node_id_from_event(&ev).is_some() {
            return;
        }

        if let Some(target) = ev.target() {
            if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                if let Ok(Some(_)) = element.closest(".zoom-controls") {
                    return;
                }
            }
        }

        let nodes_snapshot = nodes.get();
        if nodes_snapshot.is_empty() {
            return;
        }

        let container = match ev.current_target() {
            Some(c) => c,
            None => return,
        };
        let container: web_sys::HtmlElement = match container.dyn_into() {
            Ok(c) => c,
            Err(_) => return,
        };
        let viewport_width = container.client_width() as f64;
        let viewport_height = container.client_height() as f64;

        let node_width = 160.0;
        let node_height = 100.0;

        let min_x = nodes_snapshot.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
        let min_y = nodes_snapshot.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
        let max_x = nodes_snapshot.iter().map(|n| n.x + node_width).fold(f64::NEG_INFINITY, f64::max);
        let max_y = nodes_snapshot.iter().map(|n| n.y + node_height).fold(f64::NEG_INFINITY, f64::max);

        let content_width = max_x - min_x;
        let content_height = max_y - min_y;

        let padding = 0.1;
        let available_width = viewport_width * (1.0 - 2.0 * padding);
        let available_height = viewport_height * (1.0 - 2.0 * padding);

        let zoom_to_fit = (available_width / content_width)
            .min(available_height / content_height)
            .max(0.25)
            .min(4.0);

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        let viewport_center_x = viewport_width / 2.0;
        let viewport_center_y = viewport_height / 2.0;

        let new_pan_x = viewport_center_x - center_x * zoom_to_fit;
        let new_pan_y = viewport_center_y - center_y * zoom_to_fit;

        set_zoom.set(zoom_to_fit);
        set_pan_x.set(new_pan_x);
        set_pan_y.set(new_pan_y);
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

    // Canvas redraw effect
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
            on:mousedown={handle_mouse_down}
            on:mousemove={handle_mouse_move}
            on:mouseup={handle_mouse_up}
            on:mouseleave={handle_mouse_up}
            on:wheel={handle_wheel}
            on:dblclick={handle_canvas_dblclick}
        >
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

            <div
                class="canvas"
                style:transform={transform_style}
            >
                {move || {
                    let connections_snapshot = connections.get();
                    let selected = selected_node_id.get();
                    let deleting = deleting_node_id.and_then(|s| s.get());
                    nodes.get().iter().map(|node| {
                        let has_connection = connections_snapshot.iter().any(|c| c.target_node_id == node.id);
                        let is_selected = selected == Some(node.id);
                        let is_deleting = deleting == Some(node.id);
                        view! {
                            <GraphNode
                                x={node.x}
                                y={node.y}
                                label={node.label.clone()}
                                selected={is_selected}
                                node_id={node.id}
                                variant={node.variant.clone()}
                                ports={node.ports.clone()}
                                has_input_connection={has_connection}
                                is_deleting={is_deleting}
                                on_output_drag_start={Some(Callback::from(handle_output_drag_start))}
                                on_input_drag_end={Some(Callback::from(handle_input_drag_end))}
                                on_input_click={Some(handle_input_click)}
                                on_input_reroute_start={Some(Callback::from(handle_input_reroute_start))}
                                cancel_connection_drag={Some(cancel_connection_drag)}
                                on_trigger={on_trigger}
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            <div class="zoom-controls">
                <button class="zoom-btn" on:click={zoom_out} title="Zoom out">"-"</button>
                <span class="zoom-level">{zoom_percent}</span>
                <button class="zoom-btn" on:click={zoom_in} title="Zoom in">"+"</button>
                <button class="zoom-btn" on:click={reset_zoom} title="Reset view">"⟲"</button>
            </div>
        </div>
    }
}
