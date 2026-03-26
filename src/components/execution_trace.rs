use leptos::prelude::*;
use crate::components::execution_engine::{ExecutionState, Task, TraceLevel};

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

/// Get color class for task status
fn status_color(status: &crate::components::execution_engine::TaskStatus) -> &'static str {
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
    view! {
        <div class="execution-trace">
            <div class="panel-header">"Execution Trace"</div>
            <div class="panel-content trace-content">
                {move || {
                    let exec = execution.get();
                    if exec.tasks.is_empty() {
                        view! {
                            <div class="trace-empty">
                                "Click a Trigger node to start execution"
                            </div>
                        }
                    } else {
                        exec.tasks.iter().map(|task| {
                            let status_class = status_color(&task.status);
                            let duration = task.started_at.map(|started| {
                                let end = task.finished_at.unwrap_or_else(std::time::Instant::now);
                                format_duration(end.duration_since(started).as_millis())
                            }).unwrap_or_default();

                            view! {
                                <div class="trace-thread">
                                    <div class="trace-task-header" class:status_class>
                                        <span class="trace-status-dot"></span>
                                        <span class="trace-node-type">{task.node_type.clone()}</span>
                                        <span class="trace-duration">{duration}</span>
                                    </div>
                                    <div class="trace-messages">
                                        {task.messages.iter().map(|msg| {
                                            let emoji = level_emoji(&msg.level);
                                            let ts = format_timestamp(msg.timestamp);
                                            view! {
                                                <div class="trace-message">
                                                    <span class="trace-ts">{ts}</span>
                                                    <span class="trace-emoji">{emoji}</span>
                                                    <span class="trace-msg-text">{msg.message.clone()}</span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }
                }}
            </div>
        </div>
    }
}