use leptos::prelude::*;
use crate::components::canvas::state::{NodeState, NodeVariant};

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
                                <InspectorProperties node={node} />
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
                    rows="3"
                >{text.clone()}</textarea>
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
                <textarea class="property-textarea" rows="4">{template.clone()}</textarea>
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
                <input type="number" class="property-input" value={max_length as f64} min="50" max="2000" />
            </div>
        }.into_any(),
        NodeVariant::PlannerAgent { goal } => view! {
            <div class="property-group">
                <label class="property-label">"Goal"</label>
                <textarea class="property-textarea" rows="2">{goal.clone()}</textarea>
            </div>
        }.into_any(),
        NodeVariant::ExecutorAgent { task } => view! {
            <div class="property-group">
                <label class="property-label">"Task"</label>
                <textarea class="property-textarea" rows="2">{task.clone()}</textarea>
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
                    <input type="number" class="property-input" value={num_results as f64} min="1" max="20" />
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
                    <textarea class="property-textarea code" rows="4">{code.clone()}</textarea>
                </div>
            </div>
        }.into_any(),
        NodeVariant::IfCondition { branches } => view! {
            <div class="property-group">
                <label class="property-label">"Branches"</label>
                <input type="number" class="property-input" value={branches as f64} min="2" max="10" />
            </div>
        }.into_any(),
        NodeVariant::Loop { iterations } => view! {
            <div class="property-group">
                <label class="property-label">"Iterations"</label>
                <input type="number" class="property-input" value={iterations as f64} min="1" max="100" />
            </div>
        }.into_any(),
        NodeVariant::ChatOutput { response } => view! {
            <div class="property-group">
                <label class="property-label">"Response"</label>
                <textarea class="property-textarea" rows="3">{response.clone()}</textarea>
            </div>
        }.into_any(),
        NodeVariant::JsonOutput { schema } => view! {
            <div class="property-group">
                <label class="property-label">"JSON Schema"</label>
                <textarea class="property-textarea code" rows="4">{schema.clone()}</textarea>
            </div>
        }.into_any(),
        NodeVariant::ModelConfig { format, model_name, api_key, custom_url } => view! {
            <div class="property-groups">
                <div class="property-group">
                    <label class="property-label">"Format"</label>
                    <input type="text" class="property-input" value={format.clone()} />
                </div>
                <div class="property-group">
                    <label class="property-label">"Model Name"</label>
                    <input type="text" class="property-input" value={model_name.clone()} />
                </div>
                <div class="property-group">
                    <label class="property-label">"API Key"</label>
                    <input type="password" class="property-input" value={api_key.clone()} />
                </div>
                <div class="property-group">
                    <label class="property-label">"Custom URL"</label>
                    <input type="text" class="property-input" value={custom_url.clone()} />
                </div>
            </div>
        }.into_any(),
        NodeVariant::Model => view! {
            <div class="property-group">
                <span class="property-readonly">"Model node — config comes via port connection"</span>
            </div>
        }.into_any(),
    }
}
