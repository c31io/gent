use leptos::prelude::*;

/// Generic confirm modal
#[component]
pub fn ConfirmModal(
    visible: bool,
    title: String,
    message: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    view! {
        {move || if visible {
            view! {
                <div class="modal-overlay" on:click={move |_| on_cancel.run(())}>
                    <div class="modal" on:click={|ev| ev.stop_propagation()}>
                        <div class="modal-header">
                            <h3>{title}</h3>
                        </div>
                        <div class="modal-body">
                            <p>{message}</p>
                        </div>
                        <div class="modal-footer">
                            <button class="btn-cancel" on:click={move |_| on_cancel.run(())}>
                                "Cancel"
                            </button>
                            <button class="btn-confirm" on:click={move |_| on_confirm.run(())}>
                                "Confirm"
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any()
        } else {
            view! { <div></div> }.into_any()
        }}
    }
}

/// Credential strip prompt modal
#[component]
pub fn CredentialPromptModal(
    visible: bool,
    title: String,
    message: String,
    on_confirm: Callback<bool>, // true = strip credentials
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (strip_credentials, set_strip_credentials) = signal(false);

    view! {
        {move || if visible {
            view! {
                <div class="modal-overlay" on:click={move |_| on_cancel.run(())}>
                    <div class="modal" on:click={|ev| ev.stop_propagation()}>
                        <div class="modal-header">
                            <h3>{title}</h3>
                        </div>
                        <div class="modal-body">
                            <p>{message}</p>
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    checked={strip_credentials.get()}
                                    on:change={move |ev| {
                                        set_strip_credentials.set(event_target_checked(&ev));
                                    }}
                                />
                                "Remove credentials before copying"
                            </label>
                        </div>
                        <div class="modal-footer">
                            <button class="btn-cancel" on:click={move |_| on_cancel.run(())}>
                                "Cancel"
                            </button>
                            <button
                                class="btn-confirm"
                                on:click={move |_| on_confirm.run(strip_credentials.get())}
                            >
                                "Continue"
                            </button>
                        </div>
                    </div>
                </div>
            }.into_any()
        } else {
            view! { <div></div> }.into_any()
        }}
    }
}