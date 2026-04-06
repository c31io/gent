use crate::tauri_invoke;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

/// Call Tauri backend to list loaded plugins
async fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    let opts = js_sys::Object::new();
    let js_value = tauri_invoke::invoke("list_plugins".into(), &opts).await?;
    let plugins: Vec<PluginInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deserialization failed: {:?}", e))?;
    Ok(plugins)
}

/// Call Tauri backend to load a plugin from path
async fn load_plugin_from_path(
    path: String,
    capabilities: Vec<String>,
) -> Result<PluginInfo, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"path".into(), &path.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    let caps_array: js_sys::Array = capabilities
        .iter()
        .map(|s| JsValue::from(s.clone()))
        .collect();
    js_sys::Reflect::set(&opts, &"capabilities".into(), &caps_array.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    let js_value = tauri_invoke::invoke("load_plugin_from_path".into(), &opts).await?;
    let info: PluginInfo = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deserialization failed: {:?}", e))?;
    Ok(info)
}

/// Call Tauri backend to call a plugin
async fn call_plugin(plugin_id: String, input: JsValue) -> Result<JsValue, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"plugin_id".into(), &plugin_id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    js_sys::Reflect::set(&opts, &"input".into(), &input)
        .map_err(|e| format!("set error: {:?}", e))?;
    let js_value = tauri_invoke::invoke("call_plugin".into(), &opts).await?;
    Ok(js_value)
}

#[component]
pub fn PluginManager() -> impl IntoView {
    let (plugins, set_plugins) = signal(Vec::<PluginInfo>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (result, set_result) = signal(Option::<String>::None);
    let (calling, set_calling) = signal(false);

    // Load plugin list on mount
    spawn_local(async move {
        set_loading.set(true);
        match list_plugins().await {
            Ok(list) => set_plugins.set(list),
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    let handle_load = {
        let set_plugins = set_plugins.clone();
        let set_error = set_error.clone();
        move |_| {
            let path = "../public/plugins/hello-world.wasm".to_string();
            let caps = vec!["context".to_string()];
            spawn_local(async move {
                set_loading.set(true);
                set_error.set(None);
                match load_plugin_from_path(path, caps).await {
                    Ok(info) => {
                        set_plugins.update(|p| p.push(info));
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    };

    let handle_call = {
        let set_result = set_result.clone();
        let set_error = set_error.clone();
        let plugins = plugins.clone();
        move |_| {
            let plugins_snapshot = plugins.get();
            let Some(first) = plugins_snapshot.first() else {
                return;
            };
            let plugin_id = first.id.clone();
            let input = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&input, &"test".into(), &"value".into());
            set_calling.set(true);
            set_result.set(None);
            set_error.set(None);
            spawn_local(async move {
                match call_plugin(plugin_id, input.into()).await {
                    Ok(output) => {
                        if let Some(s) = output.as_string() {
                            set_result.set(Some(s));
                        } else {
                            let output_clone = output.clone();
                            if let Ok(s) = serde_wasm_bindgen::from_value::<String>(output_clone) {
                                set_result.set(Some(s));
                            } else {
                                set_result.set(Some(format!("{:?}", output)));
                            }
                        }
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_calling.set(false);
            });
        }
    };

    view! {
        <div class="plugin-manager">
            <h2>"Plugins"</h2>

            <div class="plugin-actions">
                <button on:click={handle_load} disabled={loading.get()}>
                    "Load Hello World"
                </button>
                <button on:click={handle_call} disabled={calling.get() || plugins.get().is_empty()}>
                    {if calling.get() { "Calling..." } else { "Call Plugin" }}
                </button>
            </div>

            {move || {
                if let Some(err) = error.get() {
                    view! { <p class="error">{err}</p> }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || {
                if let Some(res) = result.get() {
                    view! {
                        <div class="plugin-result">
                            <h3>"Result:"</h3>
                            <pre>{res}</pre>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || {
                if loading.get() {
                    view! { <p>"Loading..."</p> }.into_any()
                } else if plugins.get().is_empty() {
                    view! { <p>"No plugins loaded"</p> }.into_any()
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
