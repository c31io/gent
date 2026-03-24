use leptos::prelude::*;

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

    // Pan handling
    let handle_mouse_down = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
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
    };

    let handle_mouse_up = move |_ev: web_sys::MouseEvent| {
        set_is_panning.set(false);
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
                {/* Grid background pattern would go here */}
                {nodes.get().iter().map(|node| {
                    view! {
                        <GraphNode
                            x=node.x
                            y=node.y
                            label={node.label.clone()}
                            selected={node.selected}
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
