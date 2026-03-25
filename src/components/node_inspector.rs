use leptos::prelude::*;
use crate::components::canvas::state::NodeState;

/// Node inspector drawer - displays details of selected node
#[component]
pub fn NodeInspector(
    selected_node_id: Signal<Option<u32>>,
    nodes: Signal<Vec<NodeState>>,
    #[prop(default = "".to_string())] config: String,
    on_delete: Callback<(u32,), ()>,
) -> impl IntoView {
    // Derive the selected node from signals
    let selected_node = move || {
        let id = selected_node_id.get()?;
        let nodes_snapshot = nodes.get();
        nodes_snapshot.into_iter().find(|n| n.id == id)
    };

    let is_visible = move || selected_node_id.get().is_some();

    view! {
        <div
            class="node-inspector"
            class:visible={is_visible}
        >
            {move || {
                if let Some(node) = selected_node() {
                    view! {
                        <div class="inspector-content">
                            <div class="inspector-header">
                                <div class="inspector-node-info">
                                    <span class="node-type-badge">{node.node_type.clone()}</span>
                                    <span class="node-label">{node.label.clone()}</span>
                                </div>
                                <button
                                    class="delete-btn"
                                    title="Delete node"
                                    on:click={move |_| {
                                        on_delete.run((node.id,));
                                    }}
                                >
                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
                                    </svg>
                                </button>
                            </div>
                            <div class="inspector-body">
                                <label class="config-label">Configuration</label>
                                <input
                                    type="text"
                                    class="config-input"
                                    placeholder="No configuration yet"
                                    value={config.clone()}
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
