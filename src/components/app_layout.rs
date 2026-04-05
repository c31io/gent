use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use std::collections::{HashMap, HashSet};

use crate::components::canvas::state::{ConnectionState, NodeState, NodeStatus, SavedSelection, default_ports_for_type, default_variant_for_type};
use crate::components::canvas::Canvas;
use crate::components::canvas::geometry::is_text_input_keyboard;
use crate::components::execution_engine::ExecutionState;
use crate::components::left_panel::{LeftPanel, NODE_TYPES};
use crate::components::graph_section::GraphSection;
use crate::components::right_panel::RightPanel;
use crate::components::node_inspector::NodeInspector;
use crate::components::save_load::{copy_to_clipboard, paste_from_clipboard, load_selection, save_saved_selections_to_storage, generate_id, export_to_file, import_from_file};
use crate::components::toast::{ToastContainer, Toast, ToastType};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmOutput {
    pub text: String,
    pub tokens_used: u32,
    pub model: String,
    pub finish_reason: String,
    pub error: String,
}

/// Call Tauri backend for LLM completion
async fn call_llm_complete(
    format: String,
    model_name: String,
    api_key: String,
    custom_url: String,
    prompt: String,
    temperature: f64,
) -> Result<LlmOutput, String> {
    use crate::tauri_invoke;
    let opts = js_sys::Object::new();
    let config = js_sys::Object::new();
    let config_js: JsValue = config.into();
    if !js_sys::Reflect::set(&opts, &"config".into(), &config_js).unwrap_or(false) {
        return Err("Failed to set config".to_string());
    }
    if !js_sys::Reflect::set(&config_js, &"format".into(), &format.into()).unwrap_or(false) {
        return Err("Failed to set format".to_string());
    }
    if !js_sys::Reflect::set(&config_js, &"model_name".into(), &model_name.into()).unwrap_or(false) {
        return Err("Failed to set model_name".to_string());
    }
    if !js_sys::Reflect::set(&config_js, &"api_key".into(), &api_key.into()).unwrap_or(false) {
        return Err("Failed to set api_key".to_string());
    }
    if !js_sys::Reflect::set(&config_js, &"custom_url".into(), &custom_url.into()).unwrap_or(false) {
        return Err("Failed to set custom_url".to_string());
    }
    let input = js_sys::Object::new();
    let input_js: JsValue = input.into();
    if !js_sys::Reflect::set(&opts, &"input".into(), &input_js).unwrap_or(false) {
        return Err("Failed to set input".to_string());
    }
    if !js_sys::Reflect::set(&input_js, &"prompt".into(), &prompt.into()).unwrap_or(false) {
        return Err("Failed to set prompt".to_string());
    }
    if !js_sys::Reflect::set(&input_js, &"temperature".into(), &JsValue::from_f64(temperature)).unwrap_or(false) {
        return Err("Failed to set temperature".to_string());
    }
    let js_value = tauri_invoke::invoke("llm_complete".into(), &opts).await?;
    serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deserialization failed: {:?}", e))
}

/// Main application layout with left panel, canvas, and right panel
#[component]
pub fn AppLayout() -> impl IntoView {
    // Shared state for panel sizes (in pixels)
    let (left_width, set_left_width) = signal(260i32);
    let (right_width, set_right_width) = signal(300i32);

    // Track if dragging divider
    let (dragging_left, set_dragging_left) = signal(false);
    let (dragging_right, set_dragging_right) = signal(false);

    // Lifted state from Canvas for NodeInspector
    let (nodes, set_nodes) = signal(vec![
        NodeState {
            id: 1,
            x: 80.0,
            y: 150.0,
            node_type: "trigger".to_string(),
            label: "Trigger".to_string(),
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type("trigger"),
            ports: default_ports_for_type("trigger"),
        },
        NodeState {
            id: 2,
            x: 300.0,
            y: 150.0,
            node_type: "user_input".to_string(),
            label: "Text Input".to_string(),
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type("user_input"),
            ports: default_ports_for_type("user_input"),
        },
        NodeState {
            id: 3,
            x: 520.0,
            y: 150.0,
            node_type: "chat_output".to_string(),
            label: "Text Output".to_string(),
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type("chat_output"),
            ports: default_ports_for_type("chat_output"),
        },
    ]);

    let (connections, set_connections) = signal(vec![
        ConnectionState {
            id: 1,
            source_node_id: 1,
            source_port_name: "output".to_string(),
            target_node_id: 2,
            target_port_name: "trigger".to_string(),
            selected: false,
        },
        ConnectionState {
            id: 2,
            source_node_id: 2,
            source_port_name: "output".to_string(),
            target_node_id: 3,
            target_port_name: "response".to_string(),
            selected: false,
        },
    ]);
    let (selected_node_ids, set_selected_node_ids) = signal(HashSet::<u32>::new());
    let (deleting_node_id, set_deleting_node_id) = signal(Option::<u32>::None);
    let (next_node_id, set_next_node_id) = signal(4u32);

    // Inspector state for selected node
    let (inspector_node, set_inspector_node) = signal(Option::<NodeState>::None);

    // Drag preview state
    let (dragging_node_type, set_dragging_node_type) = signal(Option::<String>::None);
    let (drag_x, set_drag_x) = signal(0.0);
    let (drag_y, set_drag_y) = signal(0.0);

    // Execution state for the execution engine
    let (execution_state, set_execution_state) = signal(ExecutionState::new());

    // Keyboard shortcut state
    let (saved_selections, set_saved_selections) = signal(Vec::<SavedSelection>::new());
    let (toasts, set_toasts) = signal(Vec::<Toast>::new());
    let (next_toast_id, set_next_toast_id) = signal(0u32);
    let (next_connection_id, set_next_connection_id) = signal(100u32);

    // Load saved selections on mount
    {
        let set_saved_selections = set_saved_selections.clone();
        spawn_local(async move {
            let loaded = crate::components::save_load::load_saved_selections();
            set_saved_selections.set(loaded);
        });
    }

    // Handler for text input changes
    let handle_text_change = move |node_id: u32, new_text: String| {
        set_nodes.update(|nodes: &mut Vec<NodeState>| {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == node_id) {
                if let crate::components::canvas::state::NodeVariant::UserInput { text } = &mut node.variant {
                    *text = new_text;
                }
            }
        });
    };

    // Toast helper function
    let add_toast = {
        let set_toasts = set_toasts.clone();
        let next_toast_id = next_toast_id;
        let set_next_toast_id = set_next_toast_id;
        move |message: String, toast_type: ToastType| {
            let id = next_toast_id.get();
            set_next_toast_id.update(|n| *n += 1);
            set_toasts.update(|t| t.push(Toast { id, message, toast_type }));
            let set_toasts_clone = set_toasts.clone();
            spawn_local(async move {
                TimeoutFuture::new(3000).await;
                set_toasts_clone.update(|t| t.retain(|toast| toast.id != id));
            });
        }
    };

    // Non-passive window keydown listener
    static KEYDOWN_LISTENER_ADDED: std::sync::Once = std::sync::Once::new();
    let keydown_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
        let mut nodes = nodes.clone();
        let mut connections = connections.clone();
        let mut selected_node_ids = selected_node_ids.clone();
        let mut set_selected_node_ids = set_selected_node_ids.clone();
        let mut set_nodes = set_nodes.clone();
        let mut set_connections = set_connections.clone();
        let mut set_deleting_node_id = set_deleting_node_id.clone();
        let mut set_next_node_id = set_next_node_id.clone();
        let mut set_next_connection_id = set_next_connection_id.clone();
        let mut next_node_id = next_node_id.clone();
        let mut next_connection_id = next_connection_id.clone();
        let mut saved_selections = saved_selections.clone();
        let mut set_saved_selections = set_saved_selections.clone();
        let mut add_toast = add_toast.clone();
        let on_selection_change = None::<Callback<Option<u32>>>;

        move |ev: web_sys::KeyboardEvent| {
            let ctrl = ev.ctrl_key() || ev.meta_key();
            let key = ev.key();

            // Ignore if focus is in text input
            if is_text_input_keyboard(&ev) {
                return;
            }

            match (ctrl, key.as_str()) {
                (true, "c") => {
                    // Copy selection to clipboard
                    ev.prevent_default();
                    let selected = selected_node_ids.get();
                    if selected.is_empty() {
                        return;
                    }
                    let nodes_snapshot = nodes.get();
                    let conns_snapshot = connections.get();
                    let selection = SavedSelection {
                        id: generate_id(),
                        name: "Selection".to_string(),
                        created_at: js_sys::Date::now(),
                        nodes: nodes_snapshot.into_iter().filter(|n| selected.contains(&n.id)).collect(),
                        connections: conns_snapshot.into_iter().filter(|c| selected.contains(&c.source_node_id) && selected.contains(&c.target_node_id)).collect(),
                    };
                    let add_toast_clone = add_toast.clone();
                    spawn_local(async move {
                        match copy_to_clipboard(selection, true).await {
                            Ok(_) => add_toast_clone("Copied to clipboard".to_string(), ToastType::Success),
                            Err(e) => add_toast_clone(format!("Copy failed: {}", e), ToastType::Error),
                        }
                    });
                }
                (true, "v") => {
                    // Paste from clipboard
                    ev.prevent_default();
                    let add_toast_clone = add_toast.clone();
                    let mut set_nodes_clone = set_nodes.clone();
                    let mut set_connections_clone = set_connections.clone();
                    let mut set_next_node_id_clone = set_next_node_id.clone();
                    let mut set_next_connection_id_clone = set_next_connection_id.clone();
                    spawn_local(async move {
                        match paste_from_clipboard().await {
                            Ok(selection) => {
                                let (new_nodes, new_conns, next_id, next_conn) = load_selection(
                                    selection,
                                    next_node_id.get(),
                                    next_connection_id.get(),
                                );
                                set_nodes_clone.update(|n| n.extend(new_nodes));
                                set_connections_clone.update(|c| c.extend(new_conns));
                                set_next_node_id_clone.set(next_id);
                                set_next_connection_id_clone.set(next_conn);
                                add_toast_clone("Pasted from clipboard".to_string(), ToastType::Success);
                            }
                            Err(e) => add_toast_clone(format!("Paste failed: {}", e), ToastType::Error),
                        }
                    });
                }
                (true, "s") => {
                    // Save selection
                    ev.prevent_default();
                    if selected_node_ids.get().is_empty() {
                        add_toast("No selection to save".to_string(), ToastType::Info);
                        return;
                    }
                    let selected = selected_node_ids.get();
                    let nodes_snapshot = nodes.get();
                    let conns_snapshot = connections.get();
                    let selection = SavedSelection {
                        id: generate_id(),
                        name: "Selection".to_string(),
                        created_at: js_sys::Date::now(),
                        nodes: nodes_snapshot.into_iter().filter(|n| selected.contains(&n.id)).collect(),
                        connections: conns_snapshot.into_iter().filter(|c| selected.contains(&c.source_node_id) && selected.contains(&c.target_node_id)).collect(),
                    };
                    let mut selections = saved_selections.get();
                    selections.push(selection.clone());
                    save_saved_selections_to_storage(&selections);
                    set_saved_selections.set(selections);
                    add_toast("Selection saved".to_string(), ToastType::Success);
                }
                (true, "e") => {
                    // Export selection to file
                    ev.prevent_default();
                    if selected_node_ids.get().is_empty() {
                        add_toast("No selection to export".to_string(), ToastType::Info);
                        return;
                    }
                    let selected = selected_node_ids.get();
                    let nodes_snapshot = nodes.get();
                    let conns_snapshot = connections.get();
                    let selection = SavedSelection {
                        id: generate_id(),
                        name: "Selection".to_string(),
                        created_at: js_sys::Date::now(),
                        nodes: nodes_snapshot.into_iter().filter(|n| selected.contains(&n.id)).collect(),
                        connections: conns_snapshot.into_iter().filter(|c| selected.contains(&c.source_node_id) && selected.contains(&c.target_node_id)).collect(),
                    };
                    let filename = format!("{}.json", selection.name.to_lowercase().replace(" ", "_"));
                    let add_toast_clone = add_toast.clone();
                    spawn_local(async move {
                        match export_to_file(&selection, &filename).await {
                            Ok(_) => add_toast_clone("Exported to file".to_string(), ToastType::Success),
                            Err(e) => add_toast_clone(format!("Export failed: {}", e), ToastType::Error),
                        }
                    });
                }
                (true, "i") => {
                    // Import from file
                    ev.prevent_default();
                    let add_toast_clone = add_toast.clone();
                    let set_nodes_clone = set_nodes.clone();
                    let set_connections_clone = set_connections.clone();
                    let set_next_node_id_clone = set_next_node_id.clone();
                    let set_next_connection_id_clone = set_next_connection_id.clone();
                    spawn_local(async move {
                        match import_from_file().await {
                            Ok((selection, _name)) => {
                                // Generate new IDs and remap
                                let (new_nodes, new_conns, next_id, next_conn) = load_selection(
                                    selection,
                                    next_node_id.get(),
                                    next_connection_id.get(),
                                );
                                set_nodes_clone.update(|n| n.extend(new_nodes));
                                set_connections_clone.update(|c| c.extend(new_conns));
                                set_next_node_id_clone.set(next_id);
                                set_next_connection_id_clone.set(next_conn);
                                add_toast_clone("Imported from file".to_string(), ToastType::Success);
                            }
                            Err(e) => add_toast_clone(format!("Import failed: {}", e), ToastType::Error),
                        }
                    });
                }
                (true, "a") => {
                    // Select all
                    ev.prevent_default();
                    let all_ids: HashSet<u32> = nodes.get().iter().map(|n| n.id).collect();
                    set_selected_node_ids.set(all_ids);
                }
                (_, "Delete") | (_, "Backspace") => {
                    // Delete selected nodes
                    ev.prevent_default();
                    let to_delete = selected_node_ids.get();
                    if to_delete.is_empty() { return; }
                    // Animate and delete
                    if let Some(first_id) = to_delete.iter().next().copied() {
                        set_deleting_node_id.set(Some(first_id));
                    }
                    let to_delete_clone = to_delete.clone();
                    let mut set_nodes_clone = set_nodes.clone();
                    let mut set_connections_clone = set_connections.clone();
                    let mut set_deleting_node_id_clone = set_deleting_node_id.clone();
                    let mut set_selected_node_ids_clone = set_selected_node_ids.clone();
                    spawn_local(async move {
                        TimeoutFuture::new(300).await;
                        set_nodes_clone.update(|n| n.retain(|node| !to_delete_clone.contains(&node.id)));
                        set_connections_clone.update(|c| c.retain(|conn|
                            !to_delete_clone.contains(&conn.source_node_id) && !to_delete_clone.contains(&conn.target_node_id)
                        ));
                        set_deleting_node_id_clone.set(None);
                        set_selected_node_ids_clone.update(|ids| ids.clear());
                    });
                }
                (_, "Escape") => {
                    // Clear selection
                    set_selected_node_ids.update(|ids| ids.clear());
                    if let Some(callback) = on_selection_change {
                        callback.run(None);
                    }
                }
                _ => {}
            }
        }
    }) as Box<dyn Fn(_)>);

    KEYDOWN_LISTENER_ADDED.call_once(|| {
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", keydown_closure.as_ref().unchecked_ref());
        }
    });
    keydown_closure.forget();

    /// Execute nodes in topological order (BFS from trigger), collecting upstream results
    fn execute_downstream_order(
        nodes: &[NodeState],
        connections: &[ConnectionState],
        trigger_id: u32,
    ) -> Vec<(u32, HashMap<u32, String>)> {
        use std::collections::{HashMap, VecDeque};

        let mut in_degree: HashMap<u32, usize> = HashMap::new();
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();

        for node in nodes {
            in_degree.insert(node.id, 0);
            adj.insert(node.id, vec![]);
        }

        for conn in connections {
            if let Some(list) = adj.get_mut(&conn.source_node_id) {
                list.push(conn.target_node_id);
            }
            *in_degree.entry(conn.target_node_id).or_insert(0) += 1;
        }

        // BFS from trigger
        let mut queue: VecDeque<u32> = VecDeque::new();
        queue.push_back(trigger_id);

        let mut execution_order: Vec<(u32, HashMap<u32, String>)> = vec![];
        let upstream_results: HashMap<u32, String> = HashMap::new();

        while let Some(node_id) = queue.pop_front() {
            execution_order.push((node_id, upstream_results.clone()));

            if let Some(downstream_ids) = adj.get(&node_id) {
                for &downstream_id in downstream_ids {
                    *in_degree.entry(downstream_id).or_insert(0) -= 1;
                    if in_degree[&downstream_id] == 0 {
                        queue.push_back(downstream_id);
                    }
                }
            }
        }

        execution_order
    }

    // Handle trigger node execution
    let handle_trigger = move |node_id: u32| {
        let nodes_snapshot = nodes.get();
        let connections_snapshot = connections.get();

        if nodes_snapshot.iter().find(|n| n.id == node_id && n.node_type == "trigger").is_none() {
            return;
        }

        // Get execution order with upstream results
        let execution_plan = execute_downstream_order(&nodes_snapshot, &connections_snapshot, node_id);

        let mut exec = ExecutionState::new();
        exec.running = true;

        let mut node_results: HashMap<u32, String> = HashMap::new();

        // Extract just the node IDs in order for upstream computation
        let exec_order_ids: Vec<u32> = execution_plan.iter().map(|(id, _)| *id).collect();

        for (exec_idx, (exec_node_id, _upstream)) in execution_plan.into_iter().enumerate() {
            // Compute actual upstream results from previously executed nodes
            let mut upstream: HashMap<u32, String> = HashMap::new();
            for prev_exec_node_id in exec_order_ids.iter().take(exec_idx) {
                if let Some(result) = node_results.get(prev_exec_node_id) {
                    upstream.insert(*prev_exec_node_id, result.clone());
                }
            }

            if exec_node_id == node_id {
                // Trigger node itself
                let mut task = crate::components::execution_engine::Task::new(exec_node_id, "trigger", None);
                task.status = crate::components::execution_engine::TaskStatus::Running;
                task.started_at = Some(crate::components::execution_engine::Timestamp::now());
                task.add_message(&format!("Trigger fired [task_id={}]", task.id), crate::components::execution_engine::TraceLevel::Info);
                task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                task.status = crate::components::execution_engine::TaskStatus::Complete;
                exec.tasks.push(task);
            } else {
                // Find the node state
                if let Some(node) = nodes_snapshot.iter().find(|n| n.id == exec_node_id) {
                    // Determine parent_id from the previous task in execution order
                    let parent_id = exec.tasks.last().map(|t| t.id.clone());
                    // Set waiting_on if we have upstream dependencies
                    let waiting_on = upstream.keys().next().copied();

                    let mut task = crate::components::execution_engine::Task::new(exec_node_id, &node.node_type, parent_id.clone());
                    task.waiting_on = waiting_on;
                    task.status = crate::components::execution_engine::TaskStatus::Running;
                    task.started_at = Some(crate::components::execution_engine::Timestamp::now());
                    task.add_message(&format!("{} [node_id={}, parent_id={:?}]", node.label, task.node_id, task.parent_id), crate::components::execution_engine::TraceLevel::Info);

                    // Execute node with upstream results
                    let mut skip_post_push = false;
                    let result = match node.node_type.as_str() {
                        "user_input" => {
                            if let crate::components::canvas::state::NodeVariant::UserInput { text } = &node.variant {
                                task.add_message(&format!("Text Input: {}", text), crate::components::execution_engine::TraceLevel::Info);
                                text.clone()
                            } else {
                                task.add_message("Text Input (no text)", crate::components::execution_engine::TraceLevel::Warn);
                                String::new()
                            }
                        }
                        "chat_output" => {
                            // Get input from upstream (user_input node)
                            let input = upstream.values().next().cloned().unwrap_or_default();
                            task.add_message(&format!("Text Output received: {}", input), crate::components::execution_engine::TraceLevel::Info);
                            // Update the chat_output node's variant response
                            set_nodes.update(|nodes: &mut Vec<NodeState>| {
                                if let Some(n) = nodes.iter_mut().find(|n| n.id == exec_node_id) {
                                    if let crate::components::canvas::state::NodeVariant::ChatOutput { response } = &mut n.variant {
                                        *response = input.clone();
                                    }
                                }
                            });
                            input
                        }
                        "web_search" => {
                            task.add_message("Web Search → { mock results }", crate::components::execution_engine::TraceLevel::Info);
                            r#"{"query":"mock","results":[]}"#.to_string()
                        }
                        "code_execute" => {
                            task.add_message("Code Execute → (TBD)", crate::components::execution_engine::TraceLevel::Info);
                            "code executed".to_string()
                        }
                        "image_input" => {
                            if let crate::components::canvas::state::NodeVariant::FileInput { path } = &node.variant {
                                task.add_message(&format!("Image: {}", path), crate::components::execution_engine::TraceLevel::Info);
                                path.clone()
                            } else {
                                task.add_message("Image Input (no path)", crate::components::execution_engine::TraceLevel::Warn);
                                String::new()
                            }
                        }
                        "audio_input" => {
                            if let crate::components::canvas::state::NodeVariant::FileInput { path } = &node.variant {
                                task.add_message(&format!("Audio: {}", path), crate::components::execution_engine::TraceLevel::Info);
                                path.clone()
                            } else {
                                task.add_message("Audio Input (no path)", crate::components::execution_engine::TraceLevel::Warn);
                                String::new()
                            }
                        }
                        "model" => {
                            // Extract config from the upstream "config" port connection
                            let config_json = upstream
                                .values()
                                .next()
                                .cloned()
                                .unwrap_or_else(|| {
                                    r#"{"format":"openai","model_name":"","api_key":"","custom_url":""}"#.to_string()
                                });

                            // Simple JSON parsing since serde_json is not available in wasm
                            fn get_json_str(json: &str, key: &str) -> String {
                                let pattern = format!(r#""{}":"#, key);
                                json.find(&pattern)
                                    .map(|start| {
                                        let value_start = start + pattern.len();
                                        let rest = &json[value_start..];
                                        if rest.starts_with('"') {
                                            // Quoted string value
                                            let end = rest[1..].find('"').map(|i| i + 1).unwrap_or(rest.len());
                                            rest[1..end].to_string()
                                        } else {
                                            // Fallback for other values
                                            rest.split(',').next().unwrap_or("").split('}').next().unwrap_or("").to_string()
                                        }
                                    })
                                    .unwrap_or_default()
                            }

                            let config = crate::components::canvas::state::ModelConfig {
                                format: get_json_str(&config_json, "format"),
                                model_name: get_json_str(&config_json, "model_name"),
                                api_key: get_json_str(&config_json, "api_key"),
                                custom_url: get_json_str(&config_json, "custom_url"),
                            };

                            // prompt from upstream
                            let prompt_text = upstream.values().next().cloned().unwrap_or_default();
                            let temperature = 1.0;

                            // Push a "waiting" task
                            let mut model_task = crate::components::execution_engine::Task::new(
                                exec_node_id, "model", parent_id.clone(),
                            );
                            model_task.status = crate::components::execution_engine::TaskStatus::Waiting;
                            model_task.waiting_on = Some(exec_node_id);
                            model_task.add_message(
                                &format!("Model call: {} / {} / prompt_len={}", config.format, config.model_name, prompt_text.len()),
                                crate::components::execution_engine::TraceLevel::Info,
                            );
                            exec.tasks.push(model_task);

                            // Spawn async call
                            let exec_state_setter = set_execution_state;
                            spawn_local(async move {
                                let result = call_llm_complete(
                                    config.format.clone(),
                                    config.model_name.clone(),
                                    config.api_key.clone(),
                                    config.custom_url.clone(),
                                    prompt_text.clone(),
                                    temperature,
                                ).await;
                                match result {
                                    Ok(output) => {
                                        let status = if output.error.is_empty() {
                                            crate::components::execution_engine::TaskStatus::Complete
                                        } else {
                                            crate::components::execution_engine::TaskStatus::Error
                                        };
                                        let trace_msg = if output.error.is_empty() {
                                            format!("LLM result: {} ({} tokens)", output.text, output.tokens_used)
                                        } else {
                                            format!("LLM error: {}", output.error)
                                        };
                                        exec_state_setter.update(|exec| {
                                            if let Some(task) = exec.tasks.iter_mut().find(|t| t.node_id == exec_node_id) {
                                                task.status = status;
                                                task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                                                task.result = Some(output.text.clone());
                                                task.add_message(
                                                    &trace_msg,
                                                    crate::components::execution_engine::TraceLevel::Info,
                                                );
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        exec_state_setter.update(|exec| {
                                            if let Some(task) = exec.tasks.iter_mut().find(|t| t.node_id == exec_node_id) {
                                                task.status = crate::components::execution_engine::TaskStatus::Error;
                                                task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                                                task.add_message(
                                                    &format!("Model call failed: {}", e),
                                                    crate::components::execution_engine::TraceLevel::Error,
                                                );
                                            }
                                        });
                                    }
                                }
                            });

                            skip_post_push = true;
                            String::new()
                        }
                        "model_config" => {
                            let config_json = if let crate::components::canvas::state::NodeVariant::ModelConfig { format, model_name, api_key, custom_url } = &node.variant {
                                format!(r#"{{"format":"{}","model_name":"{}","api_key":"{}","custom_url":"{}"}}"#, format, model_name, api_key, custom_url)
                            } else {
                                r#"{"format":"openai","model_name":"","api_key":"","custom_url":""}"#.to_string()
                            };
                            task.status = crate::components::execution_engine::TaskStatus::Complete;
                            task.add_message("Model Config node", crate::components::execution_engine::TraceLevel::Info);
                            config_json
                        }
                        _ => {
                            task.add_message(&format!("{} executed", node.label), crate::components::execution_engine::TraceLevel::Info);
                            upstream.values().next().cloned().unwrap_or_default()
                        }
                    };

                    if !skip_post_push {
                        task.result = Some(result.clone());
                        node_results.insert(exec_node_id, result.clone());

                        task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                        task.status = crate::components::execution_engine::TaskStatus::Complete;
                        exec.tasks.push(task);
                    }
                }
            }
        }

        exec.running = false;
        set_execution_state.set(exec);
    };

    // Callback to start palette drag
    let on_palette_drag_start: Callback<String> = Callback::new(move |node_type: String| {
        set_dragging_node_type.set(Some(node_type));
    });

    // Handle node drop from palette
    let handle_node_drop = move |node_type: String, x: f64, y: f64| {
        let node_id = next_node_id.get();
        let label = NODE_TYPES
            .iter()
            .find(|n| n.id == node_type)
            .map(|n| n.name)
            .unwrap_or(&node_type)
            .to_string();

        let new_node = NodeState {
            id: node_id,
            x: x - 75.0,
            y: y - 50.0,
            node_type: node_type.clone(),
            label,
            selected: false,
            status: NodeStatus::Pending,
            variant: default_variant_for_type(&node_type),
            ports: default_ports_for_type(&node_type),
        };

        set_nodes.update(|nodes: &mut Vec<NodeState>| nodes.push(new_node));
        set_next_node_id.update(|n| *n += 1);
        set_selected_node_ids.update(|ids| {
            ids.clear();
            ids.insert(node_id);
        });
    };

    let handle_left_divider_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_dragging_left.set(true);
    };

    let handle_right_divider_mouse_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_dragging_right.set(true);
    };

    let handle_mouse_up = move |_ev: web_sys::MouseEvent| {
        set_dragging_left.set(false);
        set_dragging_right.set(false);
        // Clear drag preview state
        set_dragging_node_type.set(None);
        // Clear window draggedNodeType to prevent stale state on subsequent clicks
        if let Some(window) = web_sys::window() {
            let _ = js_sys::Reflect::delete_property(&window, &"draggedNodeType".into());
        }
    };

    // Global mouse move for drag preview
    let handle_global_mousemove = move |ev: web_sys::MouseEvent| {
        if dragging_node_type.get().is_some() {
            set_drag_x.set(ev.client_x() as f64);
            set_drag_y.set(ev.client_y() as f64);
        }
        if dragging_left.get() {
            let new_width = ev.client_x();
            if new_width >= 180 && new_width <= 500 {
                set_left_width.set(new_width);
            }
        }
        if dragging_right.get() {
            let window = web_sys::window().unwrap();
            let inner_width = window.inner_width().unwrap().as_f64().unwrap() as i32;
            let new_width = inner_width - ev.client_x();
            if new_width >= 180 && new_width <= 500 {
                set_right_width.set(new_width);
            }
        }
    };

    // Callback to load a saved selection into canvas
    let on_load_selection = {
        let mut set_nodes = set_nodes.clone();
        let mut set_connections = set_connections.clone();
        let mut set_next_node_id = set_next_node_id.clone();
        let mut set_next_connection_id = set_next_connection_id.clone();
        let mut next_node_id = next_node_id.clone();
        let mut next_connection_id = next_connection_id.clone();
        let mut add_toast = add_toast.clone();
        Callback::new(move |selection: SavedSelection| {
            let (new_nodes, new_conns, next_id, next_conn) = crate::components::save_load::load_selection(
                selection,
                next_node_id.get(),
                next_connection_id.get(),
            );
            set_nodes.update(|n| n.extend(new_nodes));
            set_connections.update(|c| c.extend(new_conns));
            set_next_node_id.set(next_id);
            set_next_connection_id.set(next_conn);
            add_toast("Selection loaded".to_string(), ToastType::Success);
        })
    };

    // Callback to delete a saved selection
    let on_delete_selection = {
        let saved_selections_clone = saved_selections.clone();
        let set_saved_selections = set_saved_selections.clone();
        let add_toast = add_toast.clone();
        Callback::new(move |id: String| {
            set_saved_selections.update(|selections| {
                selections.retain(|s| s.id != id);
            });
            let selections = saved_selections_clone.get();
            crate::components::save_load::save_saved_selections_to_storage(&selections);
            add_toast("Selection deleted".to_string(), ToastType::Info);
        })
    };

    view! {
        <div
            class="app-layout"
            on:mousemove={handle_global_mousemove}
            on:mouseup={handle_mouse_up}
            on:mouseleave={handle_mouse_up}
        >
            <div class="app-layout-main">
                {/* Left Panel */}
                <div
                    class="panel"
                    style:width=move || format!("{}px", left_width.get())
                >
                    <LeftPanel
                        on_drag_start={Some(on_palette_drag_start)}
                        saved_selections={saved_selections.into()}
                        on_load_selection={on_load_selection}
                        on_delete_selection={on_delete_selection}
                    />
                </div>

                {/* Left Divider */}
                <div
                    class="divider"
                    on:mousedown={handle_left_divider_mouse_down}
                ></div>

                {/* Canvas */}
                <Canvas
                    nodes={nodes.into()}
                    connections={connections.into()}
                    selected_node_ids={selected_node_ids.into()}
                    set_selected_node_ids={set_selected_node_ids}
                    set_nodes={set_nodes}
                    set_connections={set_connections}
                    deleting_node_id={Some(deleting_node_id.into())}
                    on_node_drop={Some(Callback::from(handle_node_drop))}
                    left_width={Some(left_width.into())}
                    right_width={Some(right_width.into())}
                    on_trigger={Some(Callback::new(handle_trigger))}
                    on_text_change={Some(Callback::new(move |(node_id, new_text)| handle_text_change(node_id, new_text)))}
                    on_selection_change={Some(Callback::new(move |node_id| {
                        if let Some(id) = node_id {
                            let nodes_snapshot = nodes.get();
                            if let Some(node) = nodes_snapshot.iter().find(|n| n.id == id) {
                                set_inspector_node.set(Some(node.clone()));
                            }
                        } else {
                            set_inspector_node.set(None);
                        }
                    }))}
                />

                {/* Right Divider */}
                <div
                    class="divider"
                    on:mousedown={handle_right_divider_mouse_down}
                ></div>

                {/* Right Panel */}
                <div
                    class="panel panel-right"
                    style:width=move || format!("{}px", right_width.get())
                >
                    <RightPanel execution={execution_state.into()} />
                </div>
            </div>

            {/* Node Inspector Drawer */}
            <NodeInspector
                selected_node={inspector_node.into()}
                on_node_delete={Some(Callback::new(move |node_id: u32| {
                    // Get current selection
                    let selected = selected_node_ids.get();
                    let to_delete: HashSet<u32> = if selected.contains(&node_id) {
                        // If the deleted node is in selection, delete all selected
                        selected.clone()
                    } else {
                        // Otherwise just delete the single node
                        let mut s = HashSet::new();
                        s.insert(node_id);
                        s
                    };

                    // Set all as deleting
                    set_deleting_node_id.set(Some(*to_delete.iter().next().unwrap()));
                    set_selected_node_ids.update(|ids| ids.clear());

                    // After animation, remove nodes and their connections
                    spawn_local(async move {
                        let ids_to_delete = to_delete.clone();
                        set_nodes.update(|nodes| {
                            nodes.retain(|n| !ids_to_delete.contains(&n.id));
                        });
                        set_connections.update(|conns| {
                            conns.retain(|c| !ids_to_delete.contains(&c.source_node_id) && !ids_to_delete.contains(&c.target_node_id));
                        });
                        set_deleting_node_id.set(None);
                    });
                    set_inspector_node.set(None);
                }))}
                on_close={Some(Callback::new(move |_| {
                    set_inspector_node.set(None);
                }))}
            />

            {/* Drag Preview */}
            {move || {
                if let Some(node_type) = dragging_node_type.get() {
                    let label = NODE_TYPES
                        .iter()
                        .find(|n| n.id == node_type)
                        .map(|n| n.name.to_string())
                        .unwrap_or_else(|| node_type.clone());
                    Some(view! {
                        <div
                            class="drag-preview"
                            style:left={format!("{}px", drag_x.get())}
                            style:top={format!("{}px", drag_y.get())}
                        >
                            {label}
                        </div>
                    })
                } else {
                    None
                }
            }}

            {/* Toast notifications */}
            <ToastContainer toasts={toasts.into()} on_dismiss={Callback::new(move |id| {
                set_toasts.update(|t| t.retain(|toast| toast.id != id));
            })} />
        </div>
    }
}
