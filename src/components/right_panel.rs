use leptos::prelude::*;

/// Execution trace item
#[derive(Clone, Debug)]
pub struct TraceEntry {
    pub id: u32,
    pub procedure: String,
    pub status: TraceStatus,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum TraceStatus {
    Running,
    Complete,
    Error,
    Intervention,
}

#[component]
pub fn RightPanel() -> impl IntoView {
    // Placeholder trace data - will be connected to actual execution engine later
    let trace_items = vec![
        TraceEntry {
            id: 1,
            procedure: "Parse user input".to_string(),
            status: TraceStatus::Complete,
            message: "Successfully parsed 3 fields".to_string(),
        },
        TraceEntry {
            id: 2,
            procedure: "Retrieve context".to_string(),
            status: TraceStatus::Complete,
            message: "Found 5 relevant documents".to_string(),
        },
        TraceEntry {
            id: 3,
            procedure: "Generate response".to_string(),
            status: TraceStatus::Intervention,
            message: "Ambiguous intent detected - review suggested".to_string(),
        },
    ];

    view! {
        <>
            <div class="panel-header">"Execution Trace"</div>
            <div class="panel-content">
                {trace_items.iter().map(|entry| {
                    let highlight_class = if matches!(entry.status, TraceStatus::Intervention) {
                        "highlight"
                    } else {
                        ""
                    };
                    view! {
                        <div class="trace-item" class:highlight_class>
                            <div class="trace-header">
                                {entry.procedure.clone()}
                            </div>
                            <div class="trace-body">
                                {entry.message.clone()}
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
                <div style="padding: 16px; text-align: center; color: var(--text-secondary); font-size: 12px;">
                    "Execution trace will populate when graph runs"
                </div>
            </div>
        </>
    }
}
