use crate::tauri_invoke;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

/// Script info from list_scripts
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptInfo {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub description: String,
}

/// Console line from run_script
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    let opts = js_sys::Object::new();
    let js_value = tauri_invoke::invoke("list_scripts".into(), &opts).await?;
    let scripts: Vec<ScriptInfo> = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(scripts)
}

/// Call Tauri backend to read a script
async fn read_script(id: String) -> Result<String, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    let js_value = tauri_invoke::invoke("read_script".into(), &opts).await?;

    #[derive(serde::Deserialize)]
    struct Content { source: String }
    let content: Content = serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deser error: {:?}", e))?;
    Ok(content.source)
}

/// Call Tauri backend to save a script
async fn save_script(id: String, content: String) -> Result<(), String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    js_sys::Reflect::set(&opts, &"content".into(), &content.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    tauri_invoke::invoke("save_script".into(), &opts).await?;
    Ok(())
}

/// Call Tauri backend to run a script
async fn run_script(id: String) -> Result<RunResult, String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"id".into(), &id.into())
        .map_err(|e| format!("set error: {:?}", e))?;
    let empty_input = JsValue::from(js_sys::Object::new());
    js_sys::Reflect::set(&opts, &"input".into(), &empty_input)
        .map_err(|e| format!("set error: {:?}", e))?;
    let js_value = tauri_invoke::invoke("run_script".into(), &opts).await?;
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

    let (codemirror_editor, set_codemirror_editor) = signal(Option::<JsValue>::None);
    let (editor_ready, set_editor_ready) = signal(false);
    let (pending_content, set_pending_content) = signal(Option::<String>::None);

    let (editor_height, set_editor_height) = signal(200i32);
    let (resizing, set_resizing) = signal(false);
    let (resize_start_y, set_resize_start_y) = signal(0i32);
    let (resize_start_height, set_resize_start_height) = signal(0i32);

    // Load script list on mount
    spawn_local(async move {
        set_loading.set(true);
        match list_scripts().await {
            Ok(list) => {
                let first_script = list.first().cloned();
                set_scripts.set(list);
                if let Some(first) = first_script {
                    set_selected_script.set(Some(first.clone()));
                    match read_script(first.id.clone()).await {
                        Ok(source) => {
                            // Store in pending_content so init_editor can pick it up
                            set_pending_content.set(Some(source.clone()));
                            set_editor_content.set(source);
                        }
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

            let promise: js_sys::Promise = match js_sys::Reflect::apply(&listen_fn.into(), &event, &args) {
                Ok(p) => match p.dyn_into::<js_sys::Promise>() {
                    Ok(p) => p,
                    Err(_) => return,
                },
                Err(_) => return,
            };

            let _unlisten: JsValue = match JsFuture::from(promise).await {
                Ok(v) => v,
                Err(_) => return,
            };

            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |line: JsValue| {
                if let Ok(cl) = serde_wasm_bindgen::from_value::<ConsoleLine>(line) {
                    set_console_lines.update(|lines| {
                        if lines.len() >= 10000 {
                            lines.drain(0..lines.len().saturating_sub(9999));
                        }
                        lines.push(cl);
                    });
                }
            }) as Box<dyn FnMut(JsValue)>);
            cb.forget();

            set_console_lines.update(|lines| {
                if lines.len() >= 10000 {
                    lines.drain(0..lines.len().saturating_sub(9999));
                }
                lines.push(ConsoleLine {
                    level: "info".into(),
                    message: "Console ready".into(),
                    run_id: "init".into(),
                });
            });
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

            let content = editor_content.get();
            spawn_local({
                let set_running = set_running.clone();
                let set_error = set_error.clone();
                let set_console_lines = set_console_lines.clone();
                let is_bundled = script.origin == "bundled";
                async move {
                    // Only save if not a bundled script (bundled scripts are read-only)
                    if !is_bundled {
                        if let Err(e) = save_script(script.id.clone(), content).await {
                            set_error.set(Some(e));
                            set_running.set(false);
                            return;
                        }
                    }
                    match run_script(script.id.clone()).await {
                        Ok(result) => {
                            set_console_lines.set(result.console_lines);
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
        let editor_content = editor_content.clone();
        move |_| {
            let Some(script) = selected_script.get() else { return };
            let content = editor_content.get();
            spawn_local({
                let set_error = set_error.clone();
                async move {
                    match save_script(script.id.clone(), content).await {
                        Ok(()) => {}
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
        let codemirror_editor = codemirror_editor.clone();
        let editor_ready = editor_ready.clone();
        let set_pending_content = set_pending_content.clone();
        move |id: String| {
            let scripts_snapshot = scripts.get();
            if let Some(script) = scripts_snapshot.iter().find(|s| s.id == id).cloned() {
                set_selected_script.set(Some(script.clone()));
                set_console_lines.set(Vec::new());
                let script_id = script.id.clone();
                spawn_local(async move {
                    match read_script(script_id).await {
                        Ok(source) => {
                            set_editor_content.set(source.clone());
                            // Update CodeMirror with new script content
                            if editor_ready.get() {
                                if let Some(editor) = codemirror_editor.get() {
                                    if let Ok(set_value) = js_sys::Reflect::get(&editor, &"setValue".into()) {
                                        let args = js_sys::Array::new();
                                        args.push(&source.into());
                                        let _ = js_sys::Reflect::apply(&set_value.into(), &editor, &args);
                                    }
                                }
                            } else {
                                // Editor not ready yet, store as pending
                                set_pending_content.set(Some(source));
                            }
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                });
            }
        }
    };

    // Resizer handlers
    let handle_resizer_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_resizing.set(true);
        set_resize_start_y.set(ev.client_y() as i32);
        set_resize_start_height.set(editor_height.get());
    };

    let handle_mouse_up = move |_ev: web_sys::MouseEvent| {
        set_resizing.set(false);
    };

    let handle_editor_mouseleave = move |_ev: web_sys::MouseEvent| {
        set_resizing.set(false);
    };

    let handle_editor_mousemove = move |ev: web_sys::MouseEvent| {
        if resizing.get() {
            let delta = ev.client_y() as i32 - resize_start_y.get();
            let new_height = (resize_start_height.get() + delta).max(100).min(600);
            set_editor_height.set(new_height);
        }
    };

    // Initialize CodeMirror when DOM is ready
    let init_editor = move || {
        let set_editor_content = set_editor_content.clone();
        let set_codemirror_editor = set_codemirror_editor.clone();
        let pending_content = pending_content.clone();

        spawn_local(async move {
            // Wait for pending content to be set (first script loaded)
            // Poll until we have content (max 50 attempts with 100ms delay)
            let mut initial_content = String::new();
            for _ in 0..50 {
                if let Some(content) = pending_content.get() {
                    initial_content = content;
                    break;
                }
                // Yield to event loop to let other tasks run
                gloo_timers::future::TimeoutFuture::new(100).await;
            }

            let document = match web_sys::window().and_then(|w| w.document()) {
                Some(d) => d,
                None => return,
            };
            let container = match document.get_element_by_id("script-codemirror") {
                Some(e) => e,
                None => return,
            };

            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let codemirror = match js_sys::Reflect::get(&window, &"CodeMirror".into()) {
                Ok(cm) => cm,
                Err(_) => return,
            };

            let options = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&options, &"value".into(), &initial_content.clone().into());
            let _ = js_sys::Reflect::set(&options, &"mode".into(), &"rust".into());
            let _ = js_sys::Reflect::set(&options, &"theme".into(), &"dracula".into());
            let _ = js_sys::Reflect::set(&options, &"lineNumbers".into(), &true.into());
            let _ = js_sys::Reflect::set(&options, &"tabSize".into(), &2.into());
            let _ = js_sys::Reflect::set(&options, &"indentWithTabs".into(), &false.into());
            let _ = js_sys::Reflect::set(&options, &"lineWrapping".into(), &true.into());
            let _ = js_sys::Reflect::set(&options, &"styleActiveLine".into(), &true.into());

            let args = js_sys::Array::new();
            args.push(&container.into());
            args.push(&options);

            let editor = match js_sys::Reflect::apply(&codemirror.into(), &wasm_bindgen::JsValue::UNDEFINED, &args) {
                Ok(e) => match e.dyn_into::<js_sys::Object>() {
                    Ok(obj) => obj,
                    Err(_) => return,
                },
                Err(_) => return,
            };

            // Store the editor instance for later access
            set_codemirror_editor.set(Some(editor.clone().into()));
            set_editor_ready.set(true);

            // Set up change listener
            if let Ok(on_fn) = js_sys::Reflect::get(&editor, &"on".into()) {
                let set_editor_content = set_editor_content.clone();
                let editor_clone = editor.clone();
                let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |_cm: JsValue, _change_obj: JsValue| {
                    if let Ok(get_value) = js_sys::Reflect::get(&editor_clone, &"getValue".into()) {
                        if let Ok(value) = js_sys::Reflect::apply(&get_value.into(), &editor_clone, &js_sys::Array::new()) {
                            if let Some(s) = value.as_string() {
                                set_editor_content.set(s);
                            }
                        }
                    }
                }) as Box<dyn FnMut(JsValue, JsValue)>);
                let on_args = js_sys::Array::new();
                on_args.push(&"change".into());
                on_args.push(&callback.as_js_value());
                let _ = js_sys::Reflect::apply(&on_fn.into(), &editor, &on_args);
                callback.forget();
            }
        });
    };

    init_editor();

    view! {
        <div
            class="script-editor"
            on:mousemove={handle_editor_mousemove}
            on:mouseup={handle_mouse_up}
            on:mouseleave={handle_editor_mouseleave}
        >
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

                            {/* Code editor container for CodeMirror */}
                            <div
                                class="script-editor-area"
                                style:height={move || format!("{}px", editor_height.get())}
                            >
                                <div
                                    id="script-codemirror"
                                    class="codemirror-container"
                                ></div>
                            </div>

                            {/* Horizontal resizer */}
                            <div
                                class="script-resizer"
                                on:mousedown={handle_resizer_mouse_down}
                            ></div>

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
