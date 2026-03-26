use leptos::prelude::*;
use crate::components::canvas::state::{NodeState, NodeVariant};

/// Node inspector drawer - displays details of selected node
#[component]
pub fn NodeInspector(
    /// The selected node (None = inspector hidden)
    selected_node: Option<NodeState>,
    /// Callback when a node property changes
    on_node_update: Option<Callback<NodeState>>,
    /// Callback when delete is clicked
    on_node_delete: Option<Callback<u32>>,
    /// Callback when close is clicked
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    let is_visible = move || selected_node.is_some();

    view! {
        <div class={format!("node-inspector {}", if is_visible() { "visible" } else { "" })}>
            {move || {
                if let Some(node) = selected_node() {
                    view! {
                        <div class="inspector-content">
                            <div class="inspector-header">
                                <div class="inspector-node-info">
                                    <span class="node-type-badge">{node.node_type.clone()}</span>
                                    <span class="node-label">{node.label.clone()}</span>
                                </div>
                                <div class="inspector-actions">
                                    <button
                                        class="delete-btn"
                                        on:click={move |_| {
                                            if let Some(cb) = &on_node_delete {
                                                cb.run(node.id);
                                            }
                                        }}
                                        title="Delete node"
                                    >
                                        "🗑"
                                    </button>
                                    <button
                                        class="close-btn"
                                        on:click={move |_| {
                                            if let Some(cb) = &on_close {
                                                cb.run(());
                                            }
                                        }}
                                        title="Close"
                                    >
                                        "✕"
                                    </button>
                                </div>
                            </div>
                            <div class="inspector-body">
                                <InspectorProperties node={node} />
                            </div>
                        </div>
                    }
                } else {
                    view! { <></> }
                }
            }}
        </div>
    }
}

#[component]
pub fn InspectorProperties(
    node: NodeState,
) -> impl IntoView {
    // Render variant-specific property editors
    match node.variant.clone() {
        NodeVariant::UserInput { text } => view! {
            <div class="property-group">
                <label class="property-label">"Text"</label>
                <textarea
                    class="property-textarea"
                    value={text}
                    rows="3"
                />
            </div>
        }.into_any(),
        NodeVariant::FileInput { path } => view! {
            <div class="property-group">
                <label class="property-label">"File Path"</label>
                <input type="text" class="property-input" value={path} />
            </div>
        }.into_any(),
        NodeVariant::Trigger => view! {
            <div class="property-group">
                <span class="property-readonly">"Trigger nodes start execution"</span>
            </div>
        }.into_any(),
        NodeVariant::Template { template } => view! {
            <div class="property-group">
                <label class="property-label">"Template"</label>
                <textarea class="property-textarea" value={template} rows="4" />
            </div>
        }.into_any(),
        NodeVariant::Retrieval { query } => view! {
            <div class="property-group">
                <label class="property-label">"Query"</label>
                <input type="text" class="property-input" value={query} />
            </div>
        }.into_any(),
        NodeVariant::Summarizer { max_length } => view! {
            <div class="property-group">
                <label class="property-label">"Max Length"</label>
                <input type="number" class="property-input" value={*max_length as f64} min="50" max="2000" />
            </div>
        }.into_any(),
        NodeVariant::PlannerAgent { goal } => view! {
            <div class="property-group">
                <label class="property-label">"Goal"</label>
                <textarea class="property-textarea" value={goal} rows="2" />
            </div>
        }.into_any(),
        NodeVariant::ExecutorAgent { task } => view! {
            <div class="property-group">
                <label class="property-label">"Task"</label>
                <textarea class="property-textarea" value={task} rows="2" />
            </div>
        }.into_any(),
        NodeVariant::WebSearch { query, num_results } => view! {
            <div class="property-groups">
                <div class="property-group">
                    <label class="property-label">"Query"</label>
                    <input type="text" class="property-input" value={query} />
                </div>
                <div class="property-group">
                    <label class="property-label">"Number of Results"</label>
                    <input type="number" class="property-input" value={*num_results as f64} min="1" max="20" />
                </div>
            </div>
        }.into_any(),
        NodeVariant::CodeExecute { code, language } => view! {
            <div class="property-groups">
                <div class="property-group">
                    <label class="property-label">"Language"</label>
                    <input type="text" class="property-input" value={language} />
                </div>
                <div class="property-group">
                    <label class="property-label">"Code"</label>
                    <textarea class="property-textarea code" value={code} rows="4" />
                </div>
            </div>
        }.into_any(),
        NodeVariant::IfCondition { branches } => view! {
            <div class="property-group">
                <label class="property-label">"Branches"</label>
                <input type="number" class="property-input" value={*branches as f64} min="2" max="10" />
            </div>
        }.into_any(),
        NodeVariant::Loop { iterations } => view! {
            <div class="property-group">
                <label class="property-label">"Iterations"</label>
                <input type="number" class="property-input" value={*iterations as f64} min="1" max="100" />
            </div>
        }.into_any(),
        NodeVariant::ChatOutput { response } => view! {
            <div class="property-group">
                <label class="property-label">"Response"</label>
                <textarea class="property-textarea" value={response} rows="3" />
            </div>
        }.into_any(),
        NodeVariant::JsonOutput { schema } => view! {
            <div class="property-group">
                <label class="property-label">"JSON Schema"</label>
                <textarea class="property-textarea code" value={schema} rows="4" />
            </div>
        }.into_any(),
    }
}