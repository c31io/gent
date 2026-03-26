use leptos::prelude::*;
use crate::components::execution_engine::{ExecutionState, TraceEntry, TraceLevel};

/// Format duration in milliseconds
fn format_duration(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

/// Format timestamp as HH:MM:SS.mmm
fn format_timestamp(instant: std::time::Instant) -> String {
    let elapsed = instant.elapsed().as_millis();
    let secs = (elapsed / 1000) % 60;
    let mins = (elapsed / 60000) % 60;
    let hours = elapsed / 3600000;
    let ms = elapsed % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms)
}

/// Get emoji for trace level
fn level_emoji(level: &TraceLevel) -> &'static str {
    match level {
        TraceLevel::Debug => "⚪",
        TraceLevel::Info => "🟢",
        TraceLevel::Warn => "🟡",
        TraceLevel::Error => "🔴",
    }
}

/// Get status display class
fn status_class(status: &crate::components::execution_engine::TaskStatus) -> &'static str {
    match status {
        crate::components::execution_engine::TaskStatus::Pending => "status-pending",
        crate::components::execution_engine::TaskStatus::Running => "status-running",
        crate::components::execution_engine::TaskStatus::Waiting => "status-waiting",
        crate::components::execution_engine::TaskStatus::Complete => "status-complete",
        crate::components::execution_engine::TaskStatus::Error => "status-error",
    }
}

#[component]
pub fn ExecutionTrace(
    execution: Signal<ExecutionState>,
) -> impl IntoView {
    let exec = execution.get();

    view! {
        <div class="execution-trace">
            <div class="panel-header">"Execution Trace"</div>
            <div class="panel-content trace-content">
                <div class="trace-empty">
                    "Click a Trigger node to start execution"
                </div>
            </div>
        </div>
    }
}