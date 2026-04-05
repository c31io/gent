use std::collections::HashMap;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use crate::components::canvas::state::{NodeState, ConnectionState, SavedSelection};

const STORAGE_KEY: &str = "gent_saved_selections";

/// Load saved selections from localStorage
pub fn load_saved_selections() -> Vec<SavedSelection> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };
    let stored = match storage.get_item(STORAGE_KEY) {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };
    serde_json::from_str(&stored).unwrap_or_default()
}

/// Save selections to localStorage
pub fn save_saved_selections_to_storage(selections: &[SavedSelection]) {
    let window = web_sys::window();
    let storage = window.and_then(|w| w.local_storage().ok().flatten());
    if let Some(storage) = storage {
        if let Ok(json) = serde_json::to_string(selections) {
            let _ = storage.set_item(STORAGE_KEY, &json);
        }
    }
}

/// Strip credentials from a saved selection
pub fn strip_credentials(selection: &mut SavedSelection) {
    for node in &mut selection.nodes {
        match &mut node.variant {
            crate::components::canvas::state::NodeVariant::ModelConfig { api_key, custom_url, .. } => {
                *api_key = String::new();
                *custom_url = String::new();
            }
            _ => {}
        }
    }
}

/// Generate a UUID-like ID for new saves using timestamp + random
pub fn generate_id() -> String {
    let timestamp = js_sys::Date::now() as u64;
    let random_part: u32 = {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        RandomState::new().build_hasher().finish() as u32
    };
    format!("{:x}-{:x}", timestamp, random_part)
}

/// Load a saved selection into the canvas state
pub fn load_selection(
    selection: SavedSelection,
    next_node_id: u32,
    next_conn_id: u32,
) -> (Vec<NodeState>, Vec<ConnectionState>, u32, u32) {
    let mut id_map: HashMap<u32, u32> = HashMap::new();
    let mut new_nodes = Vec::new();
    let mut new_conns = Vec::new();
    let mut current_node_id = next_node_id;
    let mut current_conn_id = next_conn_id;

    // Assign fresh IDs to nodes
    for node in selection.nodes {
        let old_id = node.id;
        id_map.insert(old_id, current_node_id);
        let mut new_node = node;
        new_node.id = current_node_id;
        new_nodes.push(new_node);
        current_node_id += 1;
    }

    // Remap connections
    for conn in selection.connections {
        if let (Some(&new_src), Some(&new_tgt)) = (
            id_map.get(&conn.source_node_id),
            id_map.get(&conn.target_node_id),
        ) {
            let mut new_conn = conn;
            new_conn.id = current_conn_id;
            new_conn.source_node_id = new_src;
            new_conn.target_node_id = new_tgt;
            new_conns.push(new_conn);
            current_conn_id += 1;
        }
    }

    (new_nodes, new_conns, current_node_id, current_conn_id)
}

/// Export a saved selection to a downloadable JSON file
pub async fn export_to_file(selection: &SavedSelection, default_name: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(selection)
        .map_err(|e| format!("serialization failed: {}", e))?;

    let window = web_sys::window().ok_or("window not available")?;

    let blob = web_sys::Blob::new_with_str_sequence(
        &js_sys::Array::of1(&json.into()),
    ).map_err(|e| format!("blob creation failed: {:?}", e))?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("URL creation failed: {:?}", e))?;

    let document = window.document().ok_or("document not available")?;
    let body = document.body().ok_or("body not available")?;

    let anchor: web_sys::HtmlAnchorElement = document.create_element("a")
        .map_err(|e| format!("anchor creation failed: {:?}", e))?
        .dyn_into()
        .map_err(|_| "could not cast to HtmlAnchorElement")?;
    anchor.set_href(&url);
    anchor.set_download(default_name);

    body.append_child(&anchor)
        .map_err(|e| format!("append child failed: {:?}", e))?;
    anchor.click();
    let _ = body.remove_child(&anchor);

    web_sys::Url::revoke_object_url(&url)
        .map_err(|e| format!("revoke URL failed: {:?}", e))?;

    Ok(())
}

/// Copy selection to clipboard as JSON
pub async fn copy_to_clipboard(selection: SavedSelection, strip: bool) -> Result<(), String> {
    let mut selection = selection;
    if strip {
        strip_credentials(&mut selection);
    }
    let json = serde_json::to_string(&selection)
        .map_err(|e| format!("serialization failed: {}", e))?;
    let window = web_sys::window().ok_or("window not available")?;
    let clipboard = window.navigator().clipboard();
    let promise = clipboard.write_text(&json);
    wasm_bindgen_futures::JsFuture::from(promise).await
        .map_err(|e| format!("clipboard write failed: {:?}", e))?;
    Ok(())
}

/// Read clipboard and parse as SavedSelection
pub async fn paste_from_clipboard() -> Result<SavedSelection, String> {
    let window = web_sys::window().ok_or("window not available")?;
    let clipboard = window.navigator().clipboard();
    let promise = clipboard.read_text();
    let text = wasm_bindgen_futures::JsFuture::from(promise).await
        .map_err(|e| format!("clipboard read failed: {:?}", e))?;
    let text_str = text.as_string().ok_or("clipboard text was not a string")?;
    serde_json::from_str(&text_str)
        .map_err(|e| format!("parse failed: {}", e))
}

/// Import a SavedSelection from a JSON file via browser file picker
/// Returns the parsed selection and its name (derived from filename)
pub async fn import_from_file() -> Result<(SavedSelection, String), String> {
    let window = web_sys::window().ok_or("window not available")?;
    let document = window.document().ok_or("document not available")?;
    let body = document.body().ok_or("body not available")?;

    let input: web_sys::HtmlInputElement = document.create_element("input")
        .map_err(|e| format!("input creation failed: {:?}", e))?
        .dyn_into()
        .map_err(|_| "could not cast to HtmlInputElement")?;
    input.set_attribute("type", "file").map_err(|e| format!("{:?}", e))?;
    input.set_attribute("accept", ".json").map_err(|e| format!("{:?}", e))?;

    let style = input.style();
    style.set_property("display", "none").map_err(|e| format!("{:?}", e))?;

    body.append_child(&input).map_err(|e| format!("{:?}", e))?;

    input.click();

    let file_promise = js_sys::Promise::new(&mut |resolve, reject| {
        let input_for_listener = input.clone();
        let input_for_closure = input.clone();
        let closure = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
            if let Some(files) = input_for_closure.files() {
                if files.length() > 0 {
                    if let Some(file) = files.get(0) {
                        let _ = resolve.call1(&resolve, &file);
                        return;
                    }
                }
            }
            let _ = reject.call1(&reject, &"No file selected".into());
        }) as Box<dyn FnMut(_)>);
        input_for_listener.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    });

    let file: web_sys::File = JsFuture::from(file_promise)
        .await
        .map_err(|e| format!("file selection failed: {:?}", e))?
        .dyn_into()
        .map_err(|_| "could not cast to File")?;

    let reader = web_sys::FileReader::new()
        .map_err(|e| format!("FileReader creation failed: {:?}", e))?;

    let load_promise = js_sys::Promise::new(&mut |resolve, reject| {
        let reader_for_closure = reader.clone();
        let closure = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
            let result = reader_for_closure.result();
            if let Ok(result) = result {
                let _ = resolve.call1(&resolve, &result);
            } else {
                let _ = reject.call1(&reject, &"Read failed".into());
            }
        }) as Box<dyn FnMut(_)>);
        reader.add_event_listener_with_callback("load", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    });

    reader.read_as_text(&file)
        .map_err(|e| format!("read_as_text failed: {:?}", e))?;

    let result = JsFuture::from(load_promise)
        .await
        .map_err(|e| format!("load failed: {:?}", e))?;
    let text = result.as_string()
        .ok_or("result was not a string")?;

    let _ = body.remove_child(&input);

    let selection: SavedSelection = serde_json::from_str(&text)
        .map_err(|e| format!("parse failed: {}", e))?;
    let name = file.name()
        .replace(".json", "")
        .replace("_", " ");

    Ok((selection, name))
}
