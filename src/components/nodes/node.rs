use leptos::prelude::*;

/// A DOM-based graph node component
#[component]
pub fn GraphNode(
    x: f64,
    y: f64,
    label: String,
    selected: bool,
) -> impl IntoView {
    let class = if selected { "node selected" } else { "node" };

    view! {
        <div
            class={class}
            style:left={format!("{}px", x)}
            style:top={format!("{}px", y)}
        >
            <div class="node-header">
                <span>{label}</span>
            </div>
            <div class="node-body">
                <div class="node-ports">
                    {/* Input port */}
                    <div class="node-port" title="Input"></div>
                    {/* Output port */}
                    <div class="node-port" title="Output"></div>
                </div>
            </div>
        </div>
    }
}
