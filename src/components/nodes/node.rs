use leptos::prelude::*;

/// A DOM-based graph node component
#[component]
pub fn GraphNode(
    x: f64,
    y: f64,
    label: String,
    selected: bool,
    node_id: u32,
    has_input_connection: bool,
    #[prop(default = false)] is_deleting: bool,
    #[prop(default = false)] is_trigger: bool,
    on_output_drag_start: Option<Callback<(u32, f64, f64)>>,
    on_input_drag_end: Option<Callback<(u32, f64, f64)>>,
    on_input_click: Option<Callback<(u32,)>>,
    on_input_reroute_start: Option<Callback<(u32,)>>,
    cancel_connection_drag: Option<Callback<(), ()>>,
    on_trigger: Option<Callback<u32>>,
) -> impl IntoView {
    let class = if selected { "node selected" } else { "node" };
    let class = if is_deleting {
        format!("{} deleting", class)
    } else {
        class.to_string()
    };

    // Track if mouse moved between mousedown and mouseup on input port
    // Also track if this input has an existing connection (for reroute detection)
    let input_drag_start = std::rc::Rc::new(std::cell::Cell::new(Option::<(f64, f64, bool)>::None));

    // Output port mousedown - start connection drag
    // NO stop_propagation - let events bubble to canvas for reliable handling
    let handle_output_mousedown = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        if let Some(cb) = &on_output_drag_start {
            cb.run((node_id, ev.client_x() as f64, ev.client_y() as f64));
        }
    };

    // Input port mousedown - if has existing connection, start reroute immediately
    let input_drag_start_clone = input_drag_start.clone();
    let on_input_reroute_start_clone = on_input_reroute_start.clone();
    let handle_input_mousedown = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        // NO stop_propagation - let canvas see this for panning decisions
        input_drag_start_clone.set(Some((ev.client_x() as f64, ev.client_y() as f64, has_input_connection)));

        // If this input has an existing connection, start the reroute drag immediately
        // so the canvas shows the preview as we drag
        if has_input_connection {
            if let Some(cb) = &on_input_reroute_start_clone {
                cb.run((node_id,));
            }
        }
    };

    // Input port mouseup - complete connection or handle click
    let input_drag_start_clone2 = input_drag_start.clone();
    let on_input_drag_end_clone = on_input_drag_end.clone();
    let on_input_click_clone = on_input_click.clone();
    let cancel_connection_drag_clone = cancel_connection_drag.clone();
    let handle_input_mouseup = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        // NO stop_propagation - let canvas handle too

        let start_pos = input_drag_start_clone2.get();
        input_drag_start_clone2.set(None);

        // If we have a start_pos and minimal movement, it's a click - remove connection
        let was_click = start_pos.map(|(sx, sy, _)| {
            let dx = (ev.client_x() as f64 - sx).abs();
            let dy = (ev.client_y() as f64 - sy).abs();
            dx < 5.0 && dy < 5.0
        }).unwrap_or(false);

        if was_click && start_pos.is_some() {
            // Click on input - remove connection and cancel any pending drag
            if let Some(cb) = &on_input_click_clone {
                cb.run((node_id,));
            }
            if let Some(cb) = &cancel_connection_drag_clone {
                cb.run(());
            }
        } else {
            // Normal drag from output to this input - complete connection
            if let Some(cb) = &on_input_drag_end_clone {
                cb.run((node_id, ev.client_x() as f64, ev.client_y() as f64));
            }
        }
    };

    view! {
        <div
            class={class}
            data-node-id={node_id}
            style:left={format!("{}px", x)}
            style:top={format!("{}px", y)}
        >
            <div class="node-header">
                <span>{label}</span>
            </div>
            <div class="node-body">
                {if is_trigger {
                    view! {
                        <button
                            class="trigger-btn"
                            on:mousedown={move |ev| {
                                ev.prevent_default();
                                if let Some(cb) = &on_trigger {
                                    cb.run(node_id);
                                }
                            }}
                        >
                            "Run"
                        </button>
                    }.into_any()
                } else {
                    view! { <span>{"Node content"}</span> }.into_any()
                }}
            </div>
            {/* Input port - positioned at left edge, vertically centered */}
            <div
                class="node-port input"
                data-port="input"
                data-node-id={node_id}
                title="Input"
                on:mousedown=handle_input_mousedown
                on:mouseup=handle_input_mouseup
            ></div>
            {/* Output port - positioned at right edge, vertically centered */}
            <div
                class="node-port output"
                data-port="output"
                data-node-id={node_id}
                title="Output"
                on:mousedown=handle_output_mousedown
            ></div>
        </div>
    }
}
