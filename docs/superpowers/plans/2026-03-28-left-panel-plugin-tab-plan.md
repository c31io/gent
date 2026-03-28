# Left Panel Plugin Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a tab bar to the left panel switching between "Palette" (existing node palette) and "Plugins" (the `PluginManager` component).

**Architecture:** `LeftPanel` becomes a shell with a `Tab` enum signal and `TabBar` component. Palette content extracted to `NodePalette` component. Plugin tab renders `<PluginManager />` directly.

**Tech Stack:** Leptos 0.7, wasm-bindgen

---

## File Map

- **Modify:** `src/components/left_panel.rs` — tab enum, `active_tab` signal, `TabBar` component, `NodePalette` extraction, tab switcher
- **No backend changes needed** — `list_plugins` command already wired

---

## Tasks

### Task 1: Add Tab enum and active_tab signal

**Files:**
- Modify: `src/components/left_panel.rs:1`

- [ ] **Step 1: Add Tab enum and active_tab signal**

Add after the `use leptos::prelude::*;` import (line 1):

```rust
#[derive(Clone, Copy, Debug)]
enum Tab {
    Palette,
    Plugins,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Palette
    }
}
```

Add `active_tab` signal inside `LeftPanel` (line 126, before `let categories`):

```rust
let (active_tab, set_active_tab) = signal(Tab::default());
```

- [ ] **Step 2: Commit**

```bash
git add src/components/left_panel.rs
git commit -m "feat(left_panel): add Tab enum and active_tab signal"
```

---

### Task 2: Create TabBar component

**Files:**
- Modify: `src/components/left_panel.rs` (append before `LeftPanel`)

- [ ] **Step 1: Add TabBar component before LeftPanel**

Add after line 119 (`get_nodes_by_category` function) and before line 121 (`LeftPanel`):

```rust
#[component]
fn TabBar(active_tab: ReadSignal<Tab>, set_active_tab: WriteSignal<Tab>) -> impl IntoView {
    view! {
        <div class="tab-bar">
            <button
                class=move || format!("tab{}", if active_tab.get() == Tab::Palette { " tab-active" } else { "" })
                on:click={move |_| set_active_tab.set(Tab::Palette)}
            >
                "Palette"
            </button>
            <button
                class=move || format!("tab{}", if active_tab.get() == Tab::Plugins { " tab-active" } else { "" })
                on:click={move |_| set_active_tab.set(Tab::Plugins)}
            >
                "Plugins"
            </button>
        </div>
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/left_panel.rs
git commit -m "feat(left_panel): add TabBar component"
```

---

### Task 3: Extract NodePalette component

**Files:**
- Modify: `src/components/left_panel.rs`

- [ ] **Step 1: Extract palette content to NodePalette**

Rename `PaletteSection` stays as-is. Add a new `NodePalette` component that holds the palette content (lines 128–144), and call it from `LeftPanel`.

Add after `PaletteSection` (after line 189):

```rust
#[component]
pub fn NodePalette(
    #[prop(default = None)] on_drag_start: Option<Callback<String>>,
) -> impl IntoView {
    let categories = ["Input", "Context", "Agent", "Tool", "Control", "Output"];

    view! {
        <div class="panel-content">
            {categories.iter().filter_map(|category| {
                let nodes = get_nodes_by_category(category);
                if nodes.is_empty() {
                    None
                } else {
                    Some(view! {
                        <PaletteSection category={*category} nodes={nodes} on_drag_start={on_drag_start} />
                    })
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
```

- [ ] **Step 2: Simplify LeftPanel to use NodePalette and TabBar**

Replace the `LeftPanel` function body (lines 121–145) with:

```rust
#[component]
pub fn LeftPanel(
    /// Callback when drag starts from palette
    #[prop(default = None)] on_drag_start: Option<Callback<String>>,
) -> impl IntoView {
    let (active_tab, set_active_tab) = signal(Tab::default());

    view! {
        <>
            <div class="panel-header">"Node Palette"</div>
            <TabBar active_tab={active_tab.into()} set_active_tab={set_active_tab} />
            {move || match active_tab.get() {
                Tab::Palette => view! { <NodePalette on_drag_start={on_drag_start} /> }.into_any(),
                Tab::Plugins => view! { <crate::components::plugin_manager::PluginManager /> }.into_any(),
            }}
        </>
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/left_panel.rs
git commit -m "feat(left_panel): extract NodePalette, add tab switching"
```

---

### Task 4: Add tab CSS

**Files:**
- Modify: `src/components/left_panel.rs` or the global CSS file

- [ ] **Step 1: Add tab bar CSS**

First, find where the CSS lives:

```bash
grep -r "\.panel-header" --include="*.css" -l
```

Then add these styles to that file (or append to `left_panel.rs` `<style>` if it uses scoped styles — check if there's a `<style>` tag in the file):

```css
.tab-bar {
    display: flex;
    border-bottom: 1px solid var(--border-color, #333);
    padding: 0 8px;
}

.tab-bar button {
    background: none;
    border: none;
    padding: 8px 16px;
    cursor: pointer;
    color: var(--text-secondary, #888);
    font-size: 13px;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
}

.tab-bar button.tab-active {
    color: var(--accent-color, #4a9eff);
    border-bottom-color: var(--accent-color, #4a9eff);
}
```

- [ ] **Step 2: Test in browser**

Run `trunk serve` and verify:
1. Left panel shows "Palette" and "Plugins" tabs
2. Clicking "Plugins" shows the plugin list (loading empty state if no plugins)
3. Clicking "Palette" shows the node palette

- [ ] **Step 3: Commit**

```bash
git add src/
git commit -m "feat(left_panel): add tab bar styling"
```

---

### Task 5: Verify warnings eliminated

**Files:**
- Check: `src/components/plugin_manager.rs`

- [ ] **Step 1: Run cargo check**

```bash
cargo check 2>&1 | grep -E "plugin_manager|PluginInfo|Manifest|list_plugins"
```

Expected: No warnings related to `plugin_manager.rs`. Warnings in other files (e.g., `state.rs`, `execution_engine.rs`) are out of scope for this plan.

- [ ] **Step 2: Commit**

```bash
git add src/components/left_panel.rs src/components/plugin_manager.rs
git commit -m "fix(plugins): eliminate dead code warnings by wiring PluginManager UI"
```
