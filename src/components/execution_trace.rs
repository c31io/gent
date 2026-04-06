use crate::components::execution_engine::{ExecutionState, Timestamp, TraceLevel};
use leptos::prelude::*;

/// Format duration in milliseconds
fn format_duration(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

/// Format timestamp as HH:MM:SS.mmm
fn format_timestamp(ts: Timestamp) -> String {
    let elapsed = ts.elapsed().as_millis();
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
pub fn ExecutionTrace(execution: Signal<ExecutionState>) -> impl IntoView {
    view! {
        <div class="execution-trace">
            <div class="panel-content trace-content">
                {move || {
                    let exec = execution.get();
                    if exec.tasks.is_empty() {
                        view! {
                            <div class="trace-empty">
                                "Click a Trigger node to start execution"
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="trace-threads">
                                {exec.tasks.iter().map(|task| {
                                    let status_cls = status_class(&task.status);
                                    let duration = task.finished_at
                                        .and_then(|f| task.started_at.map(|s| f.duration_since(s).as_millis()))
                                        .map(|ms| format_duration(ms))
                                        .unwrap_or_default();
                                    view! {
                                        <div class="trace-thread">
                                            <div class="trace-task-header">
                                                <span class={status_cls}></span>
                                                <span class="trace-task-type">{task.node_type.clone()}</span>
                                                <span class="trace-task-duration">{duration}</span>
                                            </div>
                                            <div class="trace-messages">
                                                {task.messages.iter().map(|msg| {
                                                    view! {
                                                        <div class="trace-message-item">
                                                            <span class="trace-emoji">{level_emoji(&msg.level)}</span>
                                                            <span class="trace-time">{format_timestamp(msg.timestamp)}</span>
                                                            <span class="trace-msg">{msg.message.clone()}</span>
                                                        </div>
                                                    }.into_any()
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
