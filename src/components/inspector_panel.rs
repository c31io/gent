use leptos::prelude::*;
use crate::components::canvas::state::{NodeState, NodeVariant};

/// A tab in the inspector panel
#[derive(Clone, Debug)]
pub struct InspectorTab {
    pub node_id: u32,
    pub is_preview: bool,
}

/// Bottom inspector panel with tabs
#[component]
pub fn InspectorPanel(
    /// All open tabs
    tabs: Signal<Vec<InspectorTab>>,
    /// Index of active tab (None if no tabs)
    active_tab: Signal<Option<usize>>,
    /// All nodes (to look up node data by ID)
    nodes: Signal<Vec<NodeState>>,
    /// Panel height in pixels
    height: Signal<i32>,
    /// Callback when active tab changes
    set_active_tab: Callback<Option<usize>>,
    /// Callback when tabs list changes (for closing/reordering)
    set_tabs: Callback<Vec<InspectorTab>>,
    /// Callback when a node property is updated
    on_update_node: Callback<(u32, NodeVariant)>,
) -> impl IntoView {
    view! {
        <div class="inspector-panel" style:height=move || format!("{}px", height.get())>
            <div class="inspector-divider"></div>
            <div class="inspector-content">
                {move || {
                    if tabs.get().is_empty() {
                        view! {
                            <div class="inspector-empty">
                                "Right-click a node to inspect"
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <TabBar
                                tabs=tabs
                                active_tab=active_tab
                                set_active_tab=set_active_tab
                                set_tabs=set_tabs
                            />
                            <InspectorContent
                                tabs=tabs
                                active_tab=active_tab
                                nodes=nodes
                                on_update_node=on_update_node
                            />
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn TabBar(
    tabs: Signal<Vec<InspectorTab>>,
    active_tab: Signal<Option<usize>>,
    set_active_tab: Callback<Option<usize>>,
    set_tabs: Callback<Vec<InspectorTab>>,
) -> impl IntoView {
    view! {
        <div class="inspector-tab-bar">
            {move || {
                tabs.get().iter().enumerate().map(|(idx, tab)| {
                    let is_active = active_tab.get() == Some(idx);
                    let node_id = tab.node_id;
                    view! {
                        <div
                            class="inspector-tab"
                            class:active=is_active
                            class:preview=tab.is_preview
                            on:click=move |_| set_active_tab.run(Some(idx))
                        >
                            <span class="tab-label">{format!("Node {}", node_id)}</span>
                            <button
                                class="tab-close"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    let mut new_tabs = tabs.get();
                                    if idx < new_tabs.len() {
                                        new_tabs.remove(idx);
                                        let new_active = if new_tabs.is_empty() {
                                            None
                                        } else if is_active {
                                            Some(idx.saturating_sub(1))
                                        } else {
                                            active_tab.get()
                                        };
                                        set_tabs.run(new_tabs);
                                        set_active_tab.run(new_active);
                                    }
                                }
                            >
                                "×"
                            </button>
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}

#[component]
fn InspectorContent(
    tabs: Signal<Vec<InspectorTab>>,
    active_tab: Signal<Option<usize>>,
    nodes: Signal<Vec<NodeState>>,
    on_update_node: Callback<(u32, NodeVariant)>,
) -> impl IntoView {
    view! {
        <div class="inspector-body">
            {move || {
                let tabs = tabs.get();
                let active_idx = active_tab.get()?;
                let tab = tabs.get(active_idx)?;
                let node_id = tab.node_id;
                let nodes = nodes.get();
                let node = nodes.iter().find(|n| n.id == node_id)?.clone();

                Some(view! {
                    <div class="inspector-header">
                        <span class="node-type-badge">{node.node_type.clone()}</span>
                        <span class="node-label">{node.label.clone()}</span>
                    </div>
                    <InspectorProperties node=node on_update_node=on_update_node />
                })
            }}
        </div>
    }
}

#[component]
fn InspectorProperties(
    node: NodeState,
    on_update_node: Callback<(u32, NodeVariant)>,
) -> impl IntoView {
    // TODO: Move from old node_inspector.rs
    view! {
        <div class="inspector-properties">
            "Properties for node " {node.id}
        </div>
    }
}
