use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::app_layout::AppLayout;

async fn show_main_window() {
    let opts = js_sys::Object::new();
    let _ = crate::tauri_invoke::invoke("show_main_window".into(), &opts).await;
}

#[component]
pub fn App() -> impl IntoView {
    spawn_local(async {
        show_main_window().await;
    });

    view! {
        <AppLayout />
    }
}
