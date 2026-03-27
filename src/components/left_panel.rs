use leptos::prelude::*;

/// Node palette for the left panel
#[derive(Clone, Debug)]
pub struct NodeType {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

pub const NODE_TYPES: &[NodeType] = &[
    // Input
    NodeType {
        id: "user_input",
        name: "Text Input",
        category: "Input",
        description: "Text input from user",
    },
    NodeType {
        id: "file_input",
        name: "File Input",
        category: "Input",
        description: "Load file contents",
    },
    NodeType {
        id: "trigger",
        name: "Trigger",
        category: "Input",
        description: "Click to start execution",
    },
    // Context
    NodeType {
        id: "template",
        name: "Template",
        category: "Context",
        description: "Jinja-style template",
    },
    NodeType {
        id: "retrieval",
        name: "Retrieval",
        category: "Context",
        description: "Vector DB retrieval",
    },
    NodeType {
        id: "summarizer",
        name: "Summarizer",
        category: "Context",
        description: "Summarize text",
    },
    // Agent
    NodeType {
        id: "planner_agent",
        name: "Planner Agent",
        category: "Agent",
        description: "Plans steps",
    },
    NodeType {
        id: "executor_agent",
        name: "Executor Agent",
        category: "Agent",
        description: "Executes tasks",
    },
    // Tool
    NodeType {
        id: "web_search",
        name: "Web Search",
        category: "Tool",
        description: "Search the web",
    },
    NodeType {
        id: "code_execute",
        name: "Code Execute",
        category: "Tool",
        description: "Run code",
    },
    // Control
    NodeType {
        id: "if_condition",
        name: "If / Condition",
        category: "Control",
        description: "Branch on condition",
    },
    NodeType {
        id: "loop",
        name: "Loop",
        category: "Control",
        description: "Iterate multiple times",
    },
    // Output
    NodeType {
        id: "chat_output",
        name: "Text Output",
        category: "Output",
        description: "Display chat response",
    },
    NodeType {
        id: "json_output",
        name: "JSON Output",
        category: "Output",
        description: "Structured JSON output",
    },
];

fn get_nodes_by_category(category: &str) -> Vec<&'static NodeType> {
    NODE_TYPES.iter().filter(|n| n.category == category).collect()
}

#[component]
pub fn LeftPanel(
    /// Callback when drag starts from palette
    #[prop(default = None)] on_drag_start: Option<Callback<String>>,
) -> impl IntoView {
    let categories = ["Input", "Context", "Agent", "Tool", "Control", "Output"];

    view! {
        <>
            <div class="panel-header">"Node Palette"</div>
            <div class="panel-content">
                {categories.iter().filter_map(|category| {
                    let nodes = get_nodes_by_category(category);
                    if nodes.is_empty() {
                        None
                    } else {
                        Some(view! {
                            <PaletteSection category={*category} nodes={nodes} on_drag_start={on_drag_start} />
                        })
                    }
                }).collect::<Vec<_>>()}
            </div>
        </>
    }
}

#[component]
pub fn PaletteSection(
    category: &'static str,
    nodes: Vec<&'static NodeType>,
    /// Callback when drag starts from palette
    #[prop(default = None)] on_drag_start: Option<Callback<String>>,
) -> impl IntoView {
    let items: Vec<_> = nodes.iter().map(|node| {
        let node_id = node.id;
        view! {
            <div
                class="palette-item"
                data-node-type={node.id}
                title={node.description}
                on:mousedown={move |_ev| {
                    // Store in window for canvas to pick up
                    if let Some(window) = web_sys::window() {
                        let _ = js_sys::Reflect::set(
                            &window,
                            &"draggedNodeType".into(),
                            &node_id.into()
                        );
                    }
                    // Also call the callback if provided
                    if let Some(callback) = &on_drag_start {
                        callback.run(node_id.to_string());
                    }
                }}
            >
                {node.name}
            </div>
        }
    }).collect();

    view! {
        <div class="palette-section">
            <div class="palette-section-title">{category}</div>
            <div class="palette-items">
                {items}
            </div>
        </div>
    }
}
