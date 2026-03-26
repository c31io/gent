use leptos::prelude::*;
use crate::components::canvas::state::NodeState;

/// Node inspector drawer - displays details of selected node
#[component]
pub fn NodeInspector(
    /// The selected node signal
    selected_node: Signal<Option<NodeState>>,
    /// Callback when node delete is requested
    #[prop(default = None)] on_node_delete: Option<Callback<u32>>,
    /// Callback when inspector is closed
    #[prop(default = None)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let is_visible = move || selected_node.get().is_some();

    view! {
        <div
            class="node-inspector"
            class:visible={is_visible}
        >
            {move || {
                if let Some(node) = selected_node.get() {
                    view! {
                        <div class="inspector-content">
                            <div class="inspector-header">
                                <div class="inspector-node-info">
                                    <span class="node-type-badge">{node.node_type.clone()}</span>
                                    <span class="node-label">{node.label.clone()}</span>
                                </div>
                                <div class="inspector-actions">
                                    <button
                                        class="close-btn"
                                        title="Close inspector"
                                        on:click={move |_| {
                                            if let Some(callback) = on_close {
                                                callback.run(());
                                            }
                                        }}
                                    >
                                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <path d="M18 6L6 18M6 6l12 12"/>
                                        </svg>
                                    </button>
                                    <button
                                        class="delete-btn"
                                        title="Delete node"
                                        on:click={move |_| {
                                            if let Some(callback) = on_node_delete {
                                                callback.run(node.id);
                                            }
                                        }}
                                    >
                                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
                                        </svg>
                                    </button>
                                </div>
                            </div>
                            <div class="inspector-body">
                                <label class="config-label">Configuration</label>
                                <input
                                    type="text"
                                    class="config-input"
                                    placeholder="No configuration yet"
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
