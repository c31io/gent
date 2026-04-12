use crate::components::canvas::state::{
    BundledGroup, ConnectionState, NodeState, NodeStatus, SavedSelection,
    default_ports_for_type, default_variant_for_type,
};
use leptos::prelude::*;
use std::sync::LazyLock;

fn bundled_node(id: u32, x: f64, y: f64, node_type: &'static str, label: &'static str) -> NodeState {
    NodeState {
        id,
        x,
        y,
        node_type: node_type.to_string(),
        label: label.to_string(),
        selected: false,
        status: NodeStatus::Pending,
        variant: default_variant_for_type(node_type),
        ports: default_ports_for_type(node_type),
    }
}

/// Bundled templates - lazily initialized so allocations are allowed
pub static BUNDLED_GROUPS: LazyLock<Vec<BundledGroup>> = LazyLock::new(|| {
    vec![BundledGroup {
        id: "simple_llm_chain",
        name: "Simple LLM Chain",
        description: "Trigger -> Text Input -> Model Config -> Model -> Text Output",
        nodes: vec![
            bundled_node(101, 80.0, 150.0, "trigger", "Trigger"),
            bundled_node(102, 300.0, 150.0, "user_input", "Text Input"),
            bundled_node(103, 300.0, 280.0, "model_config", "Model Config"),
            bundled_node(104, 520.0, 250.0, "model", "Model"),
            bundled_node(105, 740.0, 250.0, "chat_output", "Text Output"),
        ],
        connections: vec![
            ConnectionState {
                id: 201,
                source_node_id: 101,
                source_port_name: "output".to_string(),
                target_node_id: 102,
                target_port_name: "trigger".to_string(),
                selected: false,
            },
            ConnectionState {
                id: 202,
                source_node_id: 102,
                source_port_name: "output".to_string(),
                target_node_id: 104,
                target_port_name: "prompt".to_string(),
                selected: false,
            },
            ConnectionState {
                id: 203,
                source_node_id: 103,
                source_port_name: "config".to_string(),
                target_node_id: 104,
                target_port_name: "config".to_string(),
                selected: false,
            },
            ConnectionState {
                id: 204,
                source_node_id: 104,
                source_port_name: "text".to_string(),
                target_node_id: 105,
                target_port_name: "response".to_string(),
                selected: false,
            },
        ],
    }]
});

#[component]
pub fn GraphSection(
    saved_selections: Signal<Vec<SavedSelection>>,
    on_load_selection: Callback<SavedSelection>,
    on_delete_selection: Callback<String>,
    on_load_bundle: Callback<BundledGroup>,
) -> impl IntoView {
    let (bundled_expanded, set_bundled_expanded) = signal(true);
    let (saved_expanded, set_saved_expanded) = signal(true);

    view! {
        <div class="graph-section">
            <div class="graph-section-header">
                "Graph"
            </div>

            {/* Bundled Subsection */}
            <div class="graph-subsection">
                <div
                    class="graph-subsection-header"
                    on:click={move |_| set_bundled_expanded.update(|v| *v = !*v)}
                >
                    <span class="expand-icon">{if bundled_expanded.get() { "▼" } else { "▶" }}</span>
                    <span>"Bundled"</span>
                </div>
                {move || if bundled_expanded.get() {
                    view! {
                        <div class="graph-subsection-content">
                            {BUNDLED_GROUPS.iter().map(|bundle| {
                                let bundle_clone = bundle.clone();
                                view! {
                                    <div
                                        class="bundle-item"
                                        draggable=true
                                        on:dragstart={move |_ev| {
                                            // Store bundle id in window for canvas to pick up
                                            if let Some(window) = web_sys::window() {
                                                let _ = js_sys::Reflect::set(
                                                    &window,
                                                    &"draggedBundleId".into(),
                                                    &bundle.id.into()
                                                );
                                            }
                                        }}
                                        on:click={move |_| {
                                            on_load_bundle.run(bundle_clone.clone());
                                        }}
                                    >
                                        <span class="item-name">{bundle.name}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>

            {/* Saved Subsection */}
            <div class="graph-subsection">
                <div
                    class="graph-subsection-header"
                    on:click={move |_| set_saved_expanded.update(|v| *v = !*v)}
                >
                    <span class="expand-icon">{if saved_expanded.get() { "▼" } else { "▶" }}</span>
                    <span>"Saved"</span>
                </div>
                {move || if saved_expanded.get() {
                    let selections_vec = saved_selections.get();
                    view! {
                        <div class="graph-subsection-content">
                            {if selections_vec.is_empty() {
                                view! { <div class="empty-message">"No saved selections"</div> }.into_any()
                            } else {
                                let items: Vec<_> = (0..selections_vec.len()).map(|i| {
                                    let selection = &selections_vec[i];
                                    let selection_clone = selection.clone();
                                    let selection_id_for_drag = selection.id.clone();
                                    let selection_id_for_click = selection.id.clone();
                                    let selection_name = selection.name.clone();
                                    view! {
                                        <div
                                            class="saved-item"
                                            draggable=true
                                            on:dragstart={move |_ev| {
                                                if let Some(window) = web_sys::window() {
                                                    let _ = js_sys::Reflect::set(
                                                        &window,
                                                        &"draggedSelectionId".into(),
                                                        &selection_id_for_drag.clone().into()
                                                    );
                                                }
                                            }}
                                            on:click={move |_| {
                                                on_load_selection.run(selection_clone.clone());
                                            }}
                                        >
                                            <span class="item-name">{selection_name.clone()}</span>
                                            <button
                                                class="delete-save-btn"
                                                on:click={move |ev| {
                                                    ev.stop_propagation();
                                                    on_delete_selection.run(selection_id_for_click.clone());
                                                }}
                                            >
                                                "×"
                                            </button>
                                        </div>
                                    }
                                }).collect();
                                items.into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>
        </div>
    }
}
