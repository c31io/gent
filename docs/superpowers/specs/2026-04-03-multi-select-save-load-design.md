# Multi-Select & Save/Load Feature Design

## Context

Gent is a visual node editor for context engineering and agent orchestration. Users need to select multiple nodes at once for batch operations, and save/load node groups for reuse.

## Feature Summary

1. **Multi-select nodes** — Shift+click and rubber-band drag selection
2. **Batch delete** — Delete all selected nodes and their connections
3. **Save/Load selections** — Named saves in localStorage, drag-to-canvas to load
4. **Clipboard/Import/Export** — JSON-based with optional credential stripping

---

## 1. Multi-Select

### Interaction Model

| Action | Behavior |
|--------|----------|
| Click node | Single select (replaces current selection) |
| Shift+Click node | Add/remove node from current selection |
| Drag on empty canvas | Rubber-band selection box — all nodes within box are selected on mouse up |
| Click empty canvas | Clear selection |
| Drag node (any selection) | Move all selected nodes together |

### State Changes

**Before (single-select):**
```rust
selected_node_id: Signal<Option<u32>>
```

**After (multi-select):**
```rust
selected_node_ids: Signal<HashSet<u32>>
```

### Visual Feedback

- Selected nodes show the `selected` class (existing `border-color: var(--node-selected)` + glow ring)
- Rubber-band selection box: dashed border, semi-transparent fill (`rgba(99, 102, 241, 0.1)`)

### Implementation Notes

- Rubber-band drag: track `is_selecting` + `selection_box: Rect` in canvas state
- `handle_mousedown` on empty canvas starts rubber-band if not on node/port
- `handle_mousemove` expands selection box
- `handle_mouseup` computes intersected nodes, sets `selected_node_ids`
- Moving a node moves ALL selected nodes by same delta
- `canvas.rs` currently calls `set_selected_node_id` on node click — update to handle shift-key modifier

---

## 2. Batch Delete

### Behavior

- Triggered via `Delete` or `Backspace` key when nodes are selected
- OR via a "Delete Selected" button in the inspector/menu
- Each node gets the existing shrink animation (`is_deleting` → `node.deleting` CSS class)
- After 200ms animation: remove nodes + all connections touching those nodes
- Clear selection after delete

### Connection Cleanup

```rust
// Delete all connections where source or target is in selected set
set_connections.update(|conns| {
    conns.retain(|c|
        !selected_ids.contains(&c.source_node_id) &&
        !selected_ids.contains(&c.target_node_id)
    );
});
```

---

## 3. Save / Load Selections

### Data Structure

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedSelection {
    pub id: String,           // UUID
    pub name: String,          // User-provided name
    pub created_at: f64,       // js_sys::Date::now() timestamp
    pub nodes: Vec<NodeState>, // Full node data
    pub connections: Vec<ConnectionState>,
}
```

### Save Flow

1. User selects nodes
2. User clicks "Save Selection" (appears in inspector when selection is active, or via keyboard shortcut)
3. Modal/prompt asks for save name
4. Serialize selected nodes + connections → localStorage key: `gent_saved_selections`
5. Saved selection appears in left panel under **Saved**

### Load Flow

1. User drags a saved selection from the left panel
2. On drop: assign fresh sequential IDs to all nodes (avoids conflicts)
3. Remap connections to use new node IDs
4. Place nodes at original positions (or center if overlapping with existing nodes)
5. Connections between loaded nodes preserved

```rust
fn load_selection(selection: SavedSelection, set_nodes, set_connections, next_node_id) {
    let id_map: HashMap<u32, u32> = HashMap::new();
    let mut new_nodes = Vec::new();
    let mut new_conns = Vec::new();

    // Assign fresh IDs
    for node in selection.nodes {
        let old_id = node.id;
        let new_id = next_node_id.get();
        id_map.insert(old_id, new_id);
        let mut new_node = node;
        new_node.id = new_id;
        new_nodes.push(new_node);
        set_next_node_id.update(|n| *n += 1);
    }

    // Remap connections
    for conn in selection.connections {
        if let Some(&new_src) = id_map.get(&conn.source_node_id) {
            if let Some(&new_tgt) = id_map.get(&conn.target_node_id) {
                let mut new_conn = conn;
                new_conn.id = /* next conn id */;
                new_conn.source_node_id = new_src;
                new_conn.target_node_id = new_tgt;
                new_conns.push(new_conn);
            }
        }
    }

    set_nodes.update(|n| n.extend(new_nodes));
    set_connections.update(|c| c.extend(new_conns));
}
```

---

## 4. Left Panel: Graph Section

### Structure

```
┌─────────────────────┐
│ NODE PALETTE         │
├─────────────────────┤
│ ▶ Graph             │  ← Collapsible section
│   ├─ Bundled        │    ← Pre-made node groups (static)
│   │   ├─ Chain     │
│   │   ├─ LLM Flow  │
│   │   └─ ...       │
│   └─ Saved          │    ← User saves from localStorage
│       ├─ My Select │
│       └─ ...       │
└─────────────────────┘
```

### Bundled (Static Pre-Made Groups)

Defined as static data in code:

```rust
pub struct BundledGroup {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub nodes: Vec<NodeState>,
    pub connections: Vec<ConnectionState>,
}

pub static BUNDLED_GROUPS: &[BundledGroup] = &[
    BundledGroup { id: "simple_chain", name: "Simple Chain", ... },
    BundledGroup { id: "llm_flow", name: "LLM Flow", ... },
];
```

Each bundle renders as a draggable item in the left panel. On drag-to-canvas: same `load_selection` flow but with static data.

### Saved (User Saves)

Stored in localStorage as JSON array of `SavedSelection`. Loaded on app start. User can click ✕ to delete a save.

---

## 5. Copy / Paste / Import / Export

### Copy to Clipboard

1. User selects nodes + clicks "Copy" (or Ctrl+C)
2. Modal prompt: "Remove credentials before copying?" (checkbox, default unchecked)
3. If checked: strip fields named `api_key`, `custom_url`, `api_key_source` from all node variants
4. Serialize to JSON string
5. Write to clipboard via `navigator.clipboard.writeText()`
6. Show toast: "Copied N nodes to clipboard"

### Paste from Clipboard

1. User clicks "Paste" or Ctrl+V
2. Read clipboard text, parse as `SavedSelection` JSON
3. Validate structure (nodes array, connections array)
4. If valid: show "Paste X nodes?" confirmation
5. On confirm: same load flow (fresh IDs, remap connections, original positions)

### Import from File

1. User clicks "Import" button (in Graph section header)
2. Tauri backend opens file dialog → reads selected `.json` file
3. Parse as `SavedSelection`
4. Validate and load (same as Paste)

### Export to File

1. User selects nodes + clicks "Export"
2. Modal prompt: "Remove credentials before exporting?" (checkbox, default unchecked)
3. If checked: strip sensitive fields
4. Serialize to JSON
5. Tauri backend triggers file download dialog → save as `selection_<name>.json`

---

## 6. Credential Stripping

Fields to remove when user opts in:
- `api_key` — ModelConfig, any node variant
- `custom_url` — ModelConfig
- `api_key_source` — if exists

Strip at serialization time, before clipboard write or file export:

```rust
fn strip_credentials(selection: &mut SavedSelection) {
    for node in &mut selection.nodes {
        match &mut node.variant {
            NodeVariant::ModelConfig { api_key, .. } => *api_key = String::new(),
            // Add other variants that may have credentials
            _ => {}
        }
    }
}
```

---

## 7. UI Components

### New Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `GraphSection` | `left_panel.rs` | Collapsible section with Bundled + Saved subsections |
| `BundledGroupItem` | `left_panel.rs` | Draggable bundle in left panel |
| `SavedGroupItem` | `left_panel.rs` | Draggable save in left panel (has ✕ delete) |
| `SaveSelectionModal` | `app_layout.rs` | Name input + save button |
| `ConfirmModal` | shared | Generic confirm/cancel dialog |
| `CredentialPrompt` | shared | "Remove credentials?" checkbox modal |
| `Toast` | shared | Transient success/error notifications |

### Modified Components

- `Canvas` — add rubber-band selection state and handlers
- `AppLayout` — add `selected_node_ids` signal, save/load handlers, keyboard shortcuts
- `left_panel.rs` — add Graph section, integrate bundled + saved

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Delete` / `Backspace` | Delete selected nodes |
| `Ctrl+A` | Select all nodes |
| `Ctrl+C` | Copy selected nodes |
| `Ctrl+V` | Paste from clipboard |
| `Ctrl+S` | Save selection (opens name prompt) |
| `Escape` | Clear selection |

---

## 8. Files to Modify

### New Files
- `src/components/graph_section.rs` — Graph section with Bundled + Saved
- `src/components/save_load.rs` — Save/Load/Copy/Paste/Import/Export logic
- `src/components/toast.rs` — Toast notification component
- `src/components/modal.rs` — Shared modal components

### Modified Files
- `src/components/left_panel.rs` — Add Graph section
- `src/components/canvas/canvas.rs` — Multi-select state, rubber-band selection
- `src/components/canvas/state.rs` — `SelectedNodeIds` type, `SavedSelection` struct
- `src/components/app_layout.rs` — State, handlers, keyboard shortcuts

### CSS Additions (styles.css)
- `.selection-box` — rubber-band selection rectangle
- `.toast` — toast notification
- `.modal-overlay`, `.modal` — modal dialogs
- `.graph-section` — collapsible Graph section in left panel
- `.bundle-item`, `.saved-item` — left panel items

---

## 9. Technical Approach

### Serialization

- `serde` + `serde_wasm_bindgen` already in dependencies
- `NodeVariant` already has `#[derive(Clone, Debug)]` — add `serde::Serialize, serde::Deserialize`
- Need to add `#[serde(rename_all = "camelCase")]` if localStorage JSON keys should be camelCase

### Storage

- localStorage key: `gent_saved_selections`
- Value: JSON array of `SavedSelection`
- Load on app start (in `AppLayout::new()`)
- Save on user action (via `set_local_storage` JS interop)

### Drag-and-Drop from Left Panel

- Same pattern as existing palette drag: `window.draggedNodeType` / `window.draggedSelectionId`
- `handle_node_drop` extended to handle both node types and selection IDs

### Tauri Commands (Import/Export)

```rust
// src-tauri/src/commands.rs
#[tauri::command]
fn import_graph(path: String) -> Result<SavedSelection, String>;

#[tauri::command]
fn export_graph(path: String, selection: SavedSelection) -> Result<(), String>;
```

### ID Generation

- Use `next_node_id` counter (already exists in `app_layout.rs`)
- Atomic increment for fresh IDs on load
- Connection IDs also need fresh IDs via `next_connection_id`

---

## 10. Testing Considerations

- Rubber-band selection doesn't include nodes being dragged
- Shift+click toggles without clearing existing selection
- Delete key doesn't fire when typing in a text input
- Credential stripping only removes specified fields, preserves structure
- Load with overlapping node IDs assigns correct fresh IDs to all connections
- localStorage handles missing/corrupt data gracefully
