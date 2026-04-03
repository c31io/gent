use leptos::prelude::*;
use crate::components::canvas::state::{SavedSelection, BundledGroup};

/// Bundled templates - these would be loaded from static data
pub static BUNDLED_GROUPS: &[BundledGroup] = &[
    // Example bundled group - a simple chain
    // BundledGroup {
    //     id: "simple_chain",
    //     name: "Simple Chain",
    //     description: "A basic 3-node chain",
    //     nodes: vec![],
    //     connections: vec![],
    // },
];

#[component]
pub fn GraphSection(
    saved_selections: Signal<Vec<SavedSelection>>,
    on_load_selection: Callback<SavedSelection>,
    on_delete_selection: Callback<String>,
    on_load_bundle: Callback<BundledGroup>,
) -> impl IntoView {
    let (bundled_expanded, set_bundled_expanded) = signal(true);
    let (saved_expanded, set_saved_expanded) = signal(true);

    view! {
        <div class="graph-section">
            <div class="graph-section-header">
                "Graph"
            </div>

            {/* Bundled Subsection */}
            <div class="graph-subsection">
                <div
                    class="graph-subsection-header"
                    on:click={move |_| set_bundled_expanded.update(|v| !v)}
                >
                    <span class="expand-icon">{if bundled_expanded.get() { "▼" } else { "▶" }}</span>
                    <span>"Bundled"</span>
                </div>
                {move || if bundled_expanded.get() {
                    view! {
                        <div class="graph-subsection-content">
                            {BUNDLED_GROUPS.iter().map(|bundle| {
                                let bundle_clone = bundle.clone();
                                view! {
                                    <div
                                        class="bundle-item"
                                        draggable=true
                                        on:dragstart={move |ev| {
                                            // Store bundle id in window for canvas to pick up
                                            if let Some(window) = web_sys::window() {
                                                let _ = js_sys::Reflect::set(
                                                    &window,
                                                    &"draggedBundleId".into(),
                                                    &bundle.id.into()
                                                );
                                            }
                                        }}
                                        on:click={move |_| {
                                            on_load_bundle.run(bundle_clone.clone());
                                        }}
                                    >
                                        <span class="item-name">{bundle.name}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>

            {/* Saved Subsection */}
            <div class="graph-subsection">
                <div
                    class="graph-subsection-header"
                    on:click={move |_| set_saved_expanded.update(|v| !v)}
                >
                    <span class="expand-icon">{if saved_expanded.get() { "▼" } else { "▶" }}</span>
                    <span>"Saved"</span>
                </div>
                {move || if saved_expanded.get() {
                    view! {
                        <div class="graph-subsection-content">
                            {if saved_selections.get().is_empty() {
                                view! { <div class="empty-message">"No saved selections"</div> }.into_any()
                            } else {
                                saved_selections.get().iter().map(|selection| {
                                    let selection_clone = selection.clone();
                                    view! {
                                        <div
                                            class="saved-item"
                                            draggable=true
                                            on:dragstart={move |ev| {
                                                if let Some(window) = web_sys::window() {
                                                    let _ = js_sys::Reflect::set(
                                                        &window,
                                                        &"draggedSelectionId".into(),
                                                        &selection.id.clone().into()
                                                    );
                                                }
                                            }}
                                            on:click={move |_| {
                                                on_load_selection.run(selection_clone.clone());
                                            }}
                                        >
                                            <span class="item-name">{selection.name.clone()}</span>
                                            <button
                                                class="delete-save-btn"
                                                on:click={move |ev| {
                                                    ev.stop_propagation();
                                                    on_delete_selection.run(selection.id.clone());
                                                }}
                                            >
                                                "×"
                                            </button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>
        </div>
    }
}