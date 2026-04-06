use leptos::prelude::*;

/// Toast notification type
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Info,
}

/// A toast message
#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub toast_type: ToastType,
}

/// Toast container component
#[component]
pub fn ToastContainer(toasts: Signal<Vec<Toast>>, on_dismiss: Callback<u32>) -> impl IntoView {
    view! {
        <div class="toast-container">
            {move || {
                let toasts_vec = toasts.get();
                toasts_vec.iter().map(|toast| {
                    let class = match toast.toast_type {
                        ToastType::Success => "toast toast-success",
                        ToastType::Error => "toast toast-error",
                        ToastType::Info => "toast toast-info",
                    };
                    let toast_id = toast.id;
                    let message = toast.message.clone();
                    view! {
                        <div class={class}>
                            <span>{message}</span>
                            <button
                                class="toast-dismiss"
                                on:click={move |_| on_dismiss.run(toast_id)}
                            >
                                "×"
                            </button>
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}
