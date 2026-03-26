use leptos::prelude::*;
use crate::components::canvas::state::{Port, PortDirection, PortType, NodeVariant};

/// Renders variant-specific body content for a node
fn render_variant_body(variant: &NodeVariant) -> impl IntoView {
    match variant {
        NodeVariant::UserInput { text } => view! {
            <input
                type="text"
                class="node-variant-input"
                value={text.clone()}
                placeholder="Enter text..."
            />
        }.into_any(),
        NodeVariant::FileInput { path } => view! {
            <input
                type="text"
                class="node-variant-input"
                value={path.clone()}
                placeholder="File path..."
            />
        }.into_any(),
        NodeVariant::Template { template } => view! {
            <textarea
                class="node-variant-textarea"
                placeholder="Template..."
                rows="3"
            >{template.clone()}</textarea>
        }.into_any(),
        NodeVariant::Retrieval { query } => view! {
            <input
                type="text"
                class="node-variant-input"
                value={query.clone()}
                placeholder="Search query..."
            />
        }.into_any(),
        NodeVariant::Summarizer { max_length } => view! {
            <div class="node-variant-field">
                <label>"Max Length"</label>
                <input
                    type="number"
                    class="node-variant-input"
                    value={*max_length as f64}
                    min="50"
                    max="2000"
                />
            </div>
        }.into_any(),
        NodeVariant::PlannerAgent { goal } => view! {
            <textarea
                class="node-variant-textarea"
                placeholder="Agent goal..."
                rows="2"
            >{goal.clone()}</textarea>
        }.into_any(),
        NodeVariant::ExecutorAgent { task } => view! {
            <textarea
                class="node-variant-textarea"
                placeholder="Task description..."
                rows="2"
            >{task.clone()}</textarea>
        }.into_any(),
        NodeVariant::WebSearch { query, num_results } => view! {
            <div class="node-variant-fields">
                <input
                    type="text"
                    class="node-variant-input"
                    value={query.clone()}
                    placeholder="Search query..."
                />
                <div class="node-variant-field">
                    <label>"Results"</label>
                    <input
                        type="number"
                        class="node-variant-input small"
                        value={*num_results as f64}
                        min="1"
                        max="20"
                    />
                </div>
            </div>
        }.into_any(),
        NodeVariant::CodeExecute { code, language } => view! {
            <div class="node-variant-fields">
                <textarea
                    class="node-variant-textarea code"
                    placeholder="Code..."
                    rows="2"
                >{code.clone()}</textarea>
                <input
                    type="text"
                    class="node-variant-input small"
                    value={language.clone()}
                    placeholder="Language..."
                />
            </div>
        }.into_any(),
        NodeVariant::IfCondition { branches } => view! {
            <div class="node-variant-field">
                <label>"Branches"</label>
                <input
                    type="number"
                    class="node-variant-input"
                    value={*branches as f64}
                    min="2"
                    max="10"
                />
            </div>
        }.into_any(),
        NodeVariant::Loop { iterations } => view! {
            <div class="node-variant-field">
                <label>"Iterations"</label>
                <input
                    type="number"
                    class="node-variant-input"
                    value={*iterations as f64}
                    min="1"
                    max="100"
                />
            </div>
        }.into_any(),
        NodeVariant::ChatOutput { response } => view! {
            <textarea
                class="node-variant-textarea"
                placeholder="Response..."
                rows="2"
            >{response.clone()}</textarea>
        }.into_any(),
        NodeVariant::JsonOutput { schema } => view! {
            <textarea
                class="node-variant-textarea code"
                placeholder="JSON Schema..."
                rows="2"
            >{schema.clone()}</textarea>
        }.into_any(),
        // Trigger variant is handled separately in the GraphNode view
        _ => view! { <div /> }.into_any(),
    }
}

/// A DOM-based graph node component
#[component]
pub fn GraphNode(
    x: f64,
    y: f64,
    label: String,
    selected: bool,
    node_id: u32,
    variant: NodeVariant,
    ports: Vec<Port>,
    has_input_connection: bool,
    #[prop(default = false)] is_deleting: bool,
    on_output_drag_start: Option<Callback<(u32, f64, f64)>>,
    on_input_drag_end: Option<Callback<(u32, f64, f64)>>,
    on_input_click: Option<Callback<(u32,)>>,
    on_input_reroute_start: Option<Callback<(u32,)>>,
    cancel_connection_drag: Option<Callback<(), ()>>,
    on_trigger: Option<Callback<u32>>,
    #[prop(default = None)] on_variant_change: Option<Callback<NodeVariant>>,
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
                {match variant {
                    NodeVariant::Trigger => view! {
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
                    }.into_any(),
                    _ => render_variant_body(&variant).into_any(),
                }}
            </div>
            {/* Dynamic input ports */}
            {ports.iter().filter(|p| p.direction == PortDirection::In).map(|port| {
                let port_class_str = match &port.port_type {
                    PortType::Text => "node-port text".to_string(),
                    PortType::Image => "node-port image".to_string(),
                    PortType::Audio => "node-port audio".to_string(),
                    PortType::File => "node-port file".to_string(),
                    PortType::Embeddings => "node-port embeddings".to_string(),
                    PortType::Trigger => "node-port trigger".to_string(),
                };
                let port_name = port.name.clone();
                let input_drag_start_clone = input_drag_start.clone();
                let on_input_reroute_start_clone = on_input_reroute_start.clone();
                let handle_input_mousedown = move |ev: web_sys::MouseEvent| {
                    ev.prevent_default();
                    input_drag_start_clone.set(Some((ev.client_x() as f64, ev.client_y() as f64, has_input_connection)));
                    if has_input_connection {
                        if let Some(cb) = &on_input_reroute_start_clone {
                            cb.run((node_id,));
                        }
                    }
                };
                let input_drag_start_clone2 = input_drag_start.clone();
                let on_input_drag_end_clone = on_input_drag_end.clone();
                let on_input_click_clone = on_input_click.clone();
                let cancel_connection_drag_clone = cancel_connection_drag.clone();
                let handle_input_mouseup = move |ev: web_sys::MouseEvent| {
                    ev.prevent_default();
                    let start_pos = input_drag_start_clone2.get();
                    input_drag_start_clone2.set(None);
                    let was_click = start_pos.map(|(sx, sy, _)| {
                        let dx = (ev.client_x() as f64 - sx).abs();
                        let dy = (ev.client_y() as f64 - sy).abs();
                        dx < 5.0 && dy < 5.0
                    }).unwrap_or(false);
                    if was_click && start_pos.is_some() {
                        if let Some(cb) = &on_input_click_clone {
                            cb.run((node_id,));
                        }
                        if let Some(cb) = &cancel_connection_drag_clone {
                            cb.run(());
                        }
                    } else {
                        if let Some(cb) = &on_input_drag_end_clone {
                            cb.run((node_id, ev.client_x() as f64, ev.client_y() as f64));
                        }
                    }
                };
                let port_name_clone = port_name.clone();
                let port_name_clone2 = port_name.clone();
                view! {
                    <div
                        class={port_class_str}
                        data-port="input"
                        data-node-id={node_id}
                        data-port-name={port_name.clone()}
                        title={port_name_clone}
                        on:mousedown=handle_input_mousedown
                        on:mouseup=handle_input_mouseup
                    >
                        <span class="port-label">{port_name_clone2}</span>
                    </div>
                }
            }).collect::<Vec<_>>()}
            {/* Dynamic output ports */}
            {ports.iter().filter(|p| p.direction == PortDirection::Out).map(|port| {
                let port_class_str = match &port.port_type {
                    PortType::Text => "node-port text".to_string(),
                    PortType::Image => "node-port image".to_string(),
                    PortType::Audio => "node-port audio".to_string(),
                    PortType::File => "node-port file".to_string(),
                    PortType::Embeddings => "node-port embeddings".to_string(),
                    PortType::Trigger => "node-port trigger".to_string(),
                };
                let port_name = port.name.clone();
                let port_name_clone = port_name.clone();
                let port_name_clone2 = port_name.clone();
                view! {
                    <div
                        class={port_class_str}
                        data-port="output"
                        data-node-id={node_id}
                        data-port-name={port_name}
                        title={port_name_clone}
                        on:mousedown=handle_output_mousedown
                    >
                        <span class="port-label">{port_name_clone2}</span>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
