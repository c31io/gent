use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn PluginManager() -> impl IntoView {
    let (plugins, set_plugins) = signal(Vec::<PluginInfo>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    // Load plugin list on mount
    spawn_local(async move {
        set_loading.set(true);
        match list_plugins().await {
            Ok(list) => set_plugins.set(list),
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    view! {
        <div class="plugin-manager">
            <h2>"Plugins"</h2>

            {move || {
                if loading.get() {
                    view! { <p>"Loading..."</p> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <p class="error">{err}</p> }.into_any()
                } else {
                    view! {
                        <ul>
                            {plugins.get().iter().map(|p| {
                                view! {
                                    <li>
                                        <span>{p.manifest.name.clone()}</span>
                                        <span>" v"{p.manifest.version.clone()}</span>
                                    </li>
                                }.into_any()
                            }).collect::<Vec<_>>()}
                        </ul>
                    }.into_any()
                }
            }}
        </div>
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub manifest: Manifest,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
}

async fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    // For now, return empty list until Tauri invoke is properly configured
    Ok(Vec::new())
}