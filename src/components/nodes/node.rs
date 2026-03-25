use leptos::prelude::*;

/// A DOM-based graph node component
#[component]
pub fn GraphNode(
    x: f64,
    y: f64,
    label: String,
    selected: bool,
    node_id: u32,
    on_output_drag_start: Option<Callback<(u32, f64, f64)>>,
    on_input_drag_end: Option<Callback<(u32, f64, f64)>>,
    on_input_click: Option<Callback<(u32,)>>,
) -> impl IntoView {
    let class = if selected { "node selected" } else { "node" };

    // Track if mouse moved between mousedown and mouseup on input port
    let input_drag_start = std::rc::Rc::new(std::cell::Cell::new(Option::<(f64, f64)>::None));

    // Output port mousedown - start connection drag
    // NO stop_propagation - let events bubble to canvas for reliable handling
    let handle_output_mousedown = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        if let Some(cb) = &on_output_drag_start {
            cb.run((node_id, ev.client_x() as f64, ev.client_y() as f64));
        }
    };

    // Input port mousedown - track start position for click vs drag detection
    let input_drag_start_clone = input_drag_start.clone();
    let handle_input_mousedown = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        // NO stop_propagation - let canvas see this for panning decisions
        input_drag_start_clone.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
    };

    // Input port mouseup - complete connection or remove on click
    let input_drag_start_clone2 = input_drag_start.clone();
    let on_input_drag_end_clone = on_input_drag_end.clone();
    let on_input_click_clone = on_input_click.clone();
    let handle_input_mouseup = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        // NO stop_propagation - let canvas handle too

        let start_pos = input_drag_start_clone2.get();
        input_drag_start_clone2.set(None);

        // If we have a start_pos and minimal movement, it's a click - remove connection
        let was_click = start_pos.map(|(sx, sy)| {
            let dx = (ev.client_x() as f64 - sx).abs();
            let dy = (ev.client_y() as f64 - sy).abs();
            dx < 5.0 && dy < 5.0
        }).unwrap_or(false);

        if was_click && start_pos.is_some() {
            // Click on input - remove connection
            if let Some(cb) = &on_input_click_clone {
                cb.run((node_id,));
            }
        } else {
            // Drag from output to this input (or drag from input) - complete connection
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
                {"Node content"}
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
