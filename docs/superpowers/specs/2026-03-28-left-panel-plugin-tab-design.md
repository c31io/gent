# Design: Left Panel Plugin Tab

## Context

Cargo warnings revealed dead code in `plugin_manager.rs` (`list_plugins`, `PluginInfo`, `Manifest`) because `PluginManager` component — while fully implemented — is never rendered. The backend already has `list_plugins` command wired up and working.

The goal is to eliminate these warnings by integrating `PluginManager` into the UI.

## Decision

Add a tab bar to the left panel switching between "Palette" (existing node palette) and "Plugins" (`PluginManager`).

## Design

### Layout

```
┌─────────────────────────────────┐
│ [Palette] [Plugins]             │  ← tab bar (top of panel)
├─────────────────────────────────┤
│ ▼ Input          ▼ Context       │
│   Trigger          Context+      │  ← active tab content
│   Text Input      Embed Text     │
│   ...            ...            │
└─────────────────────────────────┘
```

### Components

**TabBar** (new, inline in `left_panel.rs`)
- Two tabs: "Palette" and "Plugins"
- Active tab has accent underline/border
- Click switches active tab

**LeftPanel** (modified)
- Holds `active_tab` signal: `signal(Tab::Palette)`
- Renders `TabBar` + content via `match active_tab.get()`
- Tab "Palette" → `<NodePalette />`
- Tab "Plugins" → `<PluginManager />`

**NodePalette** (extracted from existing left_panel.rs)
- All existing palette code extracted into this component
- `LeftPanel` imports and renders it for the Palette tab

### No State Lifted

`active_tab` is local to `LeftPanel` — no other component needs to know which tab is active.

### Data Flow

`PluginManager` already calls `list_plugins` via Tauri invoke on mount. No backend changes needed.

## Files Changed

| File | Change |
|------|--------|
| `src/components/left_panel.rs` | Extract palette to `NodePalette`, add `TabBar`, add `PluginManager` tab |

## Implementation Notes

- TabBar CSS: use existing CSS variables (`--accent-color`, `--bg-primary`)
- Keep `PluginManager` as-is — it's already correct
- Node categories remain in left panel under Palette tab
