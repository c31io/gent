use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use std::collections::{HashMap, HashSet};

use crate::components::canvas::state::{ConnectionState, NodeState, NodeStatus, SavedSelection, default_ports_for_type, default_variant_for_type};
use crate::components::canvas::geometry::is_text_input_keyboard;
use crate::components::canvas::Canvas;
use crate::components::execution_engine::ExecutionState;
use crate::components::left_panel::{LeftPanel, NODE_TYPES};
use crate::components::right_panel::RightPanel;
use crate::components::node_inspector::NodeInspector;
use crate::components::toast::{Toast, ToastType, ToastContainer};
use crate::components::modal::{ConfirmModal, CredentialPromptModal};
use crate::components::save_load::{load_saved_selections, save_saved_selections_to_storage, copy_to_clipboard, paste_from_clipboard, generate_id, load_selection};

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

    // Toast notifications
    let (toasts, set_toasts) = signal(vec![]);
    let (next_toast_id, set_next_toast_id) = signal(0u32);

    // Helper to add a toast
    let add_toast = move |message: String, toast_type: ToastType| {
        let id = next_toast_id.get();
        set_next_toast_id.update(|n| *n += 1);
        set_toasts.update(|t| t.push(Toast { id, message, toast_type }));
    };

    // Dismiss toast handler
    let dismiss_toast = move |id: u32| {
        set_toasts.update(|t| t.retain(|toast| toast.id != id));
    };

    // Modal state
    let (confirm_modal_visible, set_confirm_modal_visible) = signal(false);
    let (confirm_modal_title, set_confirm_modal_title) = signal(String::new());
    let (confirm_modal_message, set_confirm_modal_message) = signal(String::new());
    let (confirm_modal_action, set_confirm_modal_action) = signal(Option::<Callback<()>>::None);

    let (credential_modal_visible, set_credential_modal_visible) = signal(false);
    let (credential_modal_title, set_credential_modal_title) = signal(String::new());
    let (credential_modal_message, set_credential_modal_message) = signal(String::new());
    let (credential_modal_action, set_credential_modal_action) = signal(Option::<Callback<bool>>::None);

    // Saved selections (loaded from localStorage)
    let (saved_selections, set_saved_selections) = signal(load_saved_selections());

    // Next connection ID counter
    let (next_conn_id, set_next_conn_id) = signal(3u32);

    // Keyboard shortcut handler
    let handle_key_down = move |ev: web_sys::KeyboardEvent| {
        let ctrl = ev.ctrl_key();
        let key = ev.key();

        // Escape - clear selection
        if key == "Escape" {
            set_selected_node_ids.update(|ids| ids.clear());
            set_inspector_node.set(None);
            return;
        }

        // Ctrl+A - select all nodes
        if ctrl && key == "A" {
            ev.prevent_default();
            let all_ids: HashSet<u32> = nodes.get().iter().map(|n| n.id).collect();
            set_selected_node_ids.set(all_ids);
            return;
        }

        // Ctrl+C - copy selected nodes
        if ctrl && key == "C" {
            ev.prevent_default();
            let selected = selected_node_ids.get();
            if selected.is_empty() {
                add_toast("No nodes selected to copy".to_string(), ToastType::Info);
                return;
            }
            // Show credential prompt before copying
            set_credential_modal_title.set("Copy Selection".to_string());
            set_credential_modal_message.set(format!("Copy {} selected nodes to clipboard?", selected.len()));
            set_credential_modal_action.set(Some(Callback::new(move |strip| {
                let selected_ids = selected.clone();
                let nodes_snapshot = nodes.get();
                let conns_snapshot = connections.get();
                let selected_nodes: Vec<NodeState> = nodes_snapshot.into_iter().filter(|n| selected_ids.contains(&n.id)).collect();
                let selected_conns: Vec<ConnectionState> = conns_snapshot.into_iter().filter(|c| selected_ids.contains(&c.source_node_id) && selected_ids.contains(&c.target_node_id)).collect();
                let selection = SavedSelection {
                    id: generate_id(),
                    name: String::new(),
                    created_at: js_sys::Date::now(),
                    nodes: selected_nodes,
                    connections: selected_conns,
                };
                spawn_local(async move {
                    match copy_to_clipboard(selection, strip).await {
                        Ok(_) => {
                            // Toast would be shown via callback but we can't capture set_toasts in async
                        }
                        Err(e) => {
                            // Error handling
                        }
                    }
                });
            })));
            set_credential_modal_visible.set(true);
            return;
        }

        // Ctrl+V - paste from clipboard
        if ctrl && key == "V" {
            ev.prevent_default();
            spawn_local(async move {
                match paste_from_clipboard().await {
                    Ok(selection) => {
                        let count = selection.nodes.len();
                        let (new_nodes, new_conns, new_node_id, new_conn_id) = load_selection(selection, next_node_id.get(), next_conn_id.get());
                        set_next_node_id.set(new_node_id);
                        set_next_conn_id.set(new_conn_id);
                        set_nodes.update(|n| n.extend(new_nodes));
                        set_connections.update(|c| c.extend(new_conns));
                        add_toast(format!("Pasted {} nodes", count), ToastType::Success);
                    }
                    Err(e) => {
                        add_toast(format!("Paste failed: {}", e), ToastType::Error);
                    }
                }
            });
            return;
        }

        // Ctrl+S - save selection
        if ctrl && key == "S" {
            ev.prevent_default();
            let selected = selected_node_ids.get();
            if selected.is_empty() {
                add_toast("No nodes selected to save".to_string(), ToastType::Info);
                return;
            }
            // For now just save with empty name - modal would ask for name
            let nodes_snapshot = nodes.get();
            let conns_snapshot = connections.get();
            let selected_nodes: Vec<NodeState> = nodes_snapshot.into_iter().filter(|n| selected.contains(&n.id)).collect();
            let selected_conns: Vec<ConnectionState> = conns_snapshot.into_iter().filter(|c| selected.contains(&c.source_node_id) && selected.contains(&c.target_node_id)).collect();
            let selection = SavedSelection {
                id: generate_id(),
                name: format!("Selection {}", saved_selections.get().len() + 1),
                created_at: js_sys::Date::now(),
                nodes: selected_nodes,
                connections: selected_conns,
            };
            set_saved_selections.update(|s| {
                s.push(selection.clone());
            });
            save_saved_selections_to_storage(&saved_selections.get());
            add_toast("Selection saved".to_string(), ToastType::Success);
            return;
        }

        // Delete/Backspace - delete selected nodes
        if key == "Delete" || key == "Backspace" {
            // Don't delete if focus is on an input element
            if is_text_input_keyboard(&ev) {
                return;
            }
            let selected = selected_node_ids.get();
            if selected.is_empty() {
                return;
            }
            // Set all as deleting
            if let Some(first_id) = selected.iter().next() {
                set_deleting_node_id.set(Some(*first_id));
            }
            set_selected_node_ids.update(|ids| ids.clear());
            // After animation, remove nodes and connections
            let selected_clone = selected.clone();
            spawn_local(async move {
                set_nodes.update(|nodes| {
                    nodes.retain(|n| !selected_clone.contains(&n.id));
                });
                set_connections.update(|conns| {
                    conns.retain(|c| !selected_clone.contains(&c.source_node_id) && !selected_clone.contains(&c.target_node_id));
                });
                set_deleting_node_id.set(None);
            });
            set_inspector_node.set(None);
            add_toast(format!("Deleted {} nodes", selected.len()), ToastType::Info);
        }
    };

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

    view! {
        <div
            class="app-layout"
            tabindex="0"
            on:mousemove={handle_global_mousemove}
            on:mouseup={handle_mouse_up}
            on:mouseleave={handle_mouse_up}
            on:keydown={handle_key_down}
        >
            <div class="app-layout-main">
                {/* Left Panel */}
                <div
                    class="panel"
                    style:width=move || format!("{}px", left_width.get())
                >
                    <LeftPanel on_drag_start={Some(on_palette_drag_start)} />
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

            {/* Toast Notifications */}
            <ToastContainer
                toasts={toasts.into()}
                on_dismiss={Callback::new(dismiss_toast)}
            />

            {/* Confirm Modal */}
            <ConfirmModal
                visible={confirm_modal_visible.get()}
                title={confirm_modal_title.get()}
                message={confirm_modal_message.get()}
                on_confirm={Callback::new(move |_| {
                    if let Some(action) = confirm_modal_action.get() {
                        action.run(());
                    }
                    set_confirm_modal_visible.set(false);
                })}
                on_cancel={Callback::new(move |_| {
                    set_confirm_modal_visible.set(false);
                })}
            />

            {/* Credential Prompt Modal */}
            <CredentialPromptModal
                visible={credential_modal_visible.get()}
                title={credential_modal_title.get()}
                message={credential_modal_message.get()}
                on_confirm={Callback::new(move |strip| {
                    if let Some(action) = credential_modal_action.get() {
                        action.run(strip);
                    }
                    set_credential_modal_visible.set(false);
                })}
                on_cancel={Callback::new(move |_| {
                    set_credential_modal_visible.set(false);
                })}
            />
        </div>
    }
}
