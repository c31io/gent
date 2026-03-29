use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

/// Script info from list_scripts
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[wasm_bindgen]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub description: String,
}

/// Console line from run_script
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[wasm_bindgen]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
    pub run_id: String,
}

/// Run result from run_script
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RunResult {
    pub run_id: String,
    pub console_lines: Vec<ConsoleLine>,
}

/// Call Tauri backend to list scripts
async fn list_scripts() -> Result<Vec<ScriptInfo>, String> {
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".to_string());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"list_scripts".into());
    args.push(&js_sys::Object::new());

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    let js_value = JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    let scripts: Vec<ScriptInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(scripts)
}

/// Call Tauri backend to read a script
async fn read_script(id: String) -> Result<String, String> {
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".to_string());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"read_script".into());
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    args.push(&opts);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    let js_value = JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;

    #[derive(serde::Deserialize)]
    struct Content { source: String }

    let content: Content = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(content.source)
}

/// Call Tauri backend to save a script
async fn save_script(id: String, content: String) -> Result<(), String> {
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".to_string());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"save_script".into());
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    js_sys::Reflect::set(&opts, &"content".into(), &content.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    args.push(&opts);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    Ok(())
}

/// Call Tauri backend to run a script
async fn run_script(id: String) -> Result<RunResult, String> {
    let window = web_sys::window()
        .ok_or_else(|| "failed to get window".to_string())?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|e| format!("__TAURI__ error: {:?}", e))?;
    if tauri.is_undefined() {
        return Err("Scripts only available in Tauri desktop app".to_string());
    }
    let core = js_sys::Reflect::get(&tauri, &"core".into())
        .map_err(|e| format!("core error: {:?}", e))?;
    let invoke = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|e| format!("invoke error: {:?}", e))?;

    let args = js_sys::Array::new();
    args.push(&"run_script".into());
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    // Create empty object as input
    let empty_input = JsValue::from(js_sys::Object::new());
    js_sys::Reflect::set(&opts, &"input".into(), &empty_input)
        .map_err(|e| format!("set error: {:?}", e))?;
    args.push(&opts);

    let promise: js_sys::Promise = js_sys::Reflect::apply(&invoke.into(), &wasm_bindgen::JsValue::UNDEFINED, &args)
        .map_err(|e| format!("apply error: {:?}", e))?
        .dyn_into()
        .map_err(|e| format!("not a promise: {:?}", e))?;

    let js_value = JsFuture::from(promise).await
        .map_err(|e| format!("promise error: {:?}", e))?;
    let result: RunResult = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(result)
}

#[component]
pub fn ScriptEditor() -> impl IntoView {
    let (scripts, set_scripts) = signal(Vec::<ScriptInfo>::new());
    let (selected_script, set_selected_script) = signal(Option::<ScriptInfo>::None);
    let (editor_content, set_editor_content) = signal(String::new());
    let (console_lines, set_console_lines) = signal(Vec::<ConsoleLine>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (running, set_running) = signal(false);

    // Load script list on mount
    spawn_local(async move {
        set_loading.set(true);
        match list_scripts().await {
            Ok(list) => {
                // Auto-select first script if available (before moving list)
                let first_script = list.first().cloned();
                set_scripts.set(list);
                if let Some(first) = first_script {
                    set_selected_script.set(Some(first.clone()));
                    match read_script(first.id.clone()).await {
                        Ok(source) => set_editor_content.set(source),
                        Err(e) => set_error.set(Some(e)),
                    }
                }
            }
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    // Listen for console events
    spawn_local({
        let set_console_lines = set_console_lines.clone();
        async move {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let tauri = match js_sys::Reflect::get(&window, &"__TAURI__".into()) {
                Ok(t) => t,
                Err(_) => return,
            };
            let event = match js_sys::Reflect::get(&tauri, &"event".into()) {
                Ok(e) => e,
                Err(_) => return,
            };
            let listen_fn = match js_sys::Reflect::get(&event, &"listen".into()) {
                Ok(f) => f,
                Err(_) => return,
            };

            let args = js_sys::Array::new();
            args.push(&"script-console-line".into());
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |line: JsValue| {
                if let Ok(cl) = serde_wasm_bindgen::from_value::<ConsoleLine>(line) {
                    set_console_lines.update(|lines| {
                        lines.push(cl);
                    });
                }
            }) as Box<dyn FnMut(JsValue)>);
            let _ = js_sys::Reflect::apply(&listen_fn.into(), &event, &args);

            // Keep callback alive
            let _cb = cb;
        }
    });

    let handle_run = {
        let set_running = set_running.clone();
        let set_error = set_error.clone();
        let set_console_lines = set_console_lines.clone();
        move |_| {
            let Some(script) = selected_script.get() else { return };
            set_running.set(true);
            set_console_lines.set(Vec::new());

            spawn_local({
                let set_running = set_running.clone();
                let set_error = set_error.clone();
                async move {
                    match run_script(script.id.clone()).await {
                        Ok(_result) => {
                            set_running.set(false);
                        }
                        Err(e) => {
                            set_error.set(Some(e));
                            set_running.set(false);
                        }
                    }
                }
            });
        }
    };

    let handle_save = {
        let set_error = set_error.clone();
        move |_| {
            let Some(script) = selected_script.get() else { return };
            let content = editor_content.get();

            spawn_local({
                let set_error = set_error.clone();
                async move {
                    match save_script(script.id.clone(), content).await {
                        Ok(()) => { /* saved */ }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
            });
        }
    };

    let handle_select = {
        let scripts = scripts.clone();
        let set_selected_script = set_selected_script.clone();
        let set_editor_content = set_editor_content.clone();
        let set_console_lines = set_console_lines.clone();
        let set_error = set_error.clone();
        move |id: String| {
            let scripts_snapshot = scripts.get();
            if let Some(script) = scripts_snapshot.iter().find(|s| s.id == id).cloned() {
                set_selected_script.set(Some(script.clone()));
                set_console_lines.set(Vec::new());
                let script_id = script.id.clone();
                spawn_local(async move {
                    match read_script(script_id).await {
                        Ok(source) => set_editor_content.set(source),
                        Err(e) => set_error.set(Some(e)),
                    }
                });
            }
        }
    };

    view! {
        <div class="script-editor">
            <div class="panel-header">"Scripts"</div>

            {move || {
                if loading.get() {
                    view! { <p>"Loading scripts..."</p> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <p class="error">{err}</p> }.into_any()
                } else {
                    view! {
                        <>
                            {/* Script selector dropdown */}
                            <div class="script-selector">
                                <select
                                    class="script-select"
                                    on:change={move |ev| {
                                        let id = event_target_value(&ev);
                                        handle_select(id);
                                    }}
                                >
                                    <option value="" disabled=true selected={selected_script.get().is_none()}>
                                        "Select a script..."
                                    </option>
                                    {scripts.get().iter().map(|s| {
                                        let is_selected = selected_script.get().as_ref().map(|sel| sel.id == s.id).unwrap_or(false);
                                        view! {
                                            <option
                                                value={s.id.clone()}
                                                selected={is_selected}
                                            >
                                                {format!("[{}] {}", s.origin, s.name)}
                                            </option>
                                        }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </div>

                            {/* Code editor textarea (CodeMirror integration point) */}
                            <div class="script-editor-area">
                                <textarea
                                    class="code-textarea"
                                    rows="12"
                                    on:input={move |ev| {
                                        set_editor_content.set(event_target_value(&ev));
                                    }}
                                    placeholder="// Select or create a script..."
                                >{editor_content.get()}</textarea>
                            </div>

                            {/* Action buttons */}
                            <div class="script-actions">
                                <button
                                    class="btn-run"
                                    on:click={handle_run}
                                    disabled={running.get() || selected_script.get().is_none()}
                                >
                                    {if running.get() { "Running..." } else { "Run" }}
                                </button>
                                <button
                                    class="btn-save"
                                    on:click={handle_save}
                                    disabled={selected_script.get().is_none()}
                                >
                                    "Save"
                                </button>
                            </div>

                            {/* Console output */}
                            <div class="script-console">
                                <div class="console-header">"Console"</div>
                                <div class="console-lines">
                                    {console_lines.get().iter().map(|line| {
                                        let cls = if line.level == "error" { "console-error" } else { "console-info" };
                                        view! {
                                            <div class={cls}>
                                                <span class="console-level">{format!("[{}]", line.level)}</span>
                                                <span class="console-message">{line.message.clone()}</span>
                                            </div>
                                        }.into_any()
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        </>
                    }.into_any()
                }
            }}
        </div>
    }
}