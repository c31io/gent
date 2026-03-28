use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

/// Call Tauri backend to list loaded plugins
async fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    // Access window.__TAURI__.core.invoke
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;

    // Check if __TAURI__ exists
    let tauri_val = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("failed to access __TAURI__: {:?}", e))?;

    // __TAURI__ is undefined when running in browser (trunk serve) without Tauri
    if tauri_val.is_undefined() {
        return Err("Plugins are only available in the Tauri desktop app".to_string());
    }

    if !tauri_val.is_object() {
        return Err(format!("__TAURI__ is not an object: {:?}", tauri_val));
    }

    let tauri = tauri_val;
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("failed to get core: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("failed to get invoke: {:?}", e))?;

    // Create invoke("list_plugins", {}) call
    let args = js_sys::Array::new();
    args.push(&"list_plugins".into());
    args.push(&js_sys::Object::new());

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("invoke failed: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    // Await the promise
    let js_value = JsFuture::from(promise)
        .await
        .map_err(|e| format!("promise failed: {:?}", e))?;

    // Deserialize JSON to PluginInfo
    let plugins: Vec<PluginInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deserialization failed: {:?}", e))?;

    Ok(plugins)
}

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