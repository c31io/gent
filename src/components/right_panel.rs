use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tab {
    Trace,
    Scripts,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Trace
    }
}

#[component]
fn TabBar(active_tab: ReadSignal<Tab>, set_active_tab: WriteSignal<Tab>) -> impl IntoView {
    view! {
        <div class="tab-bar">
            <button
                class=move || format!("tab{}", if active_tab.get() == Tab::Trace { " tab-active" } else { "" })
                on:click={move |_| set_active_tab.set(Tab::Trace)}
            >
                "Trace"
            </button>
            <button
                class=move || format!("tab{}", if active_tab.get() == Tab::Scripts { " tab-active" } else { "" })
                on:click={move |_| set_active_tab.set(Tab::Scripts)}
            >
                "Scripts"
            </button>
        </div>
    }
}

#[component]
pub fn RightPanel(
    execution: Signal<crate::components::execution_engine::ExecutionState>,
) -> impl IntoView {
    let (active_tab, set_active_tab) = signal(Tab::default());

    view! {
        <div class="right-panel">
            <TabBar active_tab={active_tab.into()} set_active_tab={set_active_tab} />
            {move || match active_tab.get() {
                Tab::Trace => view! {
                    <crate::components::execution_trace::ExecutionTrace execution={execution} />
                }.into_any(),
                Tab::Scripts => view! {
                    <crate::components::script_editor::ScriptEditor />
                }.into_any(),
            }}
        </div>
    }
}
