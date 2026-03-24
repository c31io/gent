use leptos::prelude::*;

/// Connection/wire between two nodes
#[component]
pub fn Connection(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    selected: bool,
) -> impl IntoView {
    // Create bezier curve path
    let path = move || {
        let mid_x = (start_x + end_x) / 2.0;
        format!(
            "M {} {} C {} {}, {} {}, {} {}",
            start_x, start_y,
            mid_x, start_y,
            mid_x, end_y,
            end_x, end_y
        )
    };

    let class = move || {
        if selected {
            "connection selected"
        } else {
            "connection"
        }
    };

    view! {
        <svg class={class} style:width="100%" style:height="100%">
            <path d={path()} />
        </svg>
    }
}
