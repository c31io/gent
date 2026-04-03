use std::collections::HashMap;
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
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let random_part: u32 = {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        RandomState::new().build_hasher().finish() as u32
    };
    format!("{:x}-{:x}", duration.as_millis(), random_part)
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