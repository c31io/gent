# Gent Rune Plugin System Design

**Date:** 2026-03-27
**Status:** Approved
**Author:** c31io

---

## Overview

Gent's key differentiator is a first-class plugin system powered by Rune (a WASM-native Rust companion language) running inside a Tauri desktop application. Plugins are compiled to `.wasm` modules and loaded at runtime with capability-based security.

This document describes the plugin architecture, capability model, lifecycle, host API, and security model.

---

## Why Rune

| Aspect | Lua | Rune |
|--------|-----|------|
| Rust integration | via mlua/rlua | Native |
| WASM support | Limited | First-class |
| Ecosystem maturity | Battle-tested | Early-stage |
| Learning curve | Low | Moderate |

Rune was chosen because:
- It aligns with Gent's Rust/Tauri/WASM stack philosophy
- WASM-native design enables true sandboxing
- Type-safe FFI with Rust reduces runtime errors
- Early adopter positioning in a nascent ecosystem

---

## Plugin Structure

Each plugin is a Rune module compiled to WASM with two required exports:

```rune
// manifest.rn

pub fn manifest() -> Manifest {
    Manifest {
        name: "My Plugin",
        version: "1.0.0",
        description: "What it does",
        capabilities: ["context", "tools"],
    }
}

pub fn process(input: Input) -> Output {
    // plugin logic
}
```

### Required Exports

| Export | Type | Description |
|--------|------|-------------|
| `manifest()` | `fn() -> Manifest` | Plugin metadata and capability requirements |
| `process(input)` | `fn(Input) -> Output` | Main entry point for plugin execution |

### Optional Exports

| Export | Type | Description |
|--------|------|-------------|
| `init(ctx)` | `fn(Context) -> ()` | One-time initialization with capability-gated handle |

---

## Capability Model

Plugins declare needed capabilities upfront. Gent provides a capability-gated API at load time.

| Capability | What It Grants |
|------------|----------------|
| `context` | Read/write context data (prompt fragments, variables) |
| `tools` | Invoke Gent's built-in tools (search, code exec, etc.) |
| `memory` | Read/write to session or long-term memory |
| `nodes` | Create/connect/modify graph nodes at runtime |
| `execution` | Trigger execution, read trace data |

**A plugin with `capabilities: ["context"]` cannot touch nodes or execution** — sandbox enforced at the Rust host level.

---

## Plugin Lifecycle

```
Load → Validate Capabilities → Initialize → Process Messages → Unload
```

### Load
- User selects a `.wasm` file via Gent's plugin manager UI
- Gent validates the WASM binary structure (not the content)
- Manifest export is read and parsed

### Validate
- Requested capabilities are checked against Gent's allowed list
- If any requested capability is not supported, load fails with `PluginError::UnsupportedCapability`

### Initialize
- If the plugin exports `init(ctx)`, it is called with a capability-gated Context handle
- This handle provides access only to the declared capabilities
- Initialization failures cause immediate unload

### Process
- Gent calls `process(input)` for each plugin invocation
- Input/output are JSON-serializable values
- Errors are caught and returned as `PluginError::Runtime`

### Unload
- Clean shutdown — no callbacks pending
- All allocated resources released by the WASM runtime
- Plugin state cleared from memory

---

## Host API (Gent → Plugin)

```rust
// gent/src/plugins/host.rs

pub trait PluginHost {
    /// Call a loaded plugin with JSON input, returns JSON output
    fn call_plugin(
        &mut self,
        plugin_id: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError>;

    /// Get the capabilities a plugin was granted
    fn get_capabilities(&self, plugin_id: &str) -> &[Capability];

    /// Revoke and unload a plugin
    fn revoke_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError>;
}
```

Plugins receive a typed handle to the host — they cannot call arbitrary Rust code.

---

## Security Model

### Capability Enforcement
- Capability declarations are enforced by the Rust host, not by the plugin
- Plugin authors cannot request capabilities they weren't granted
- The Rust host validates all return values before using them

### WASM Isolation
- Plugins run in isolated WASM linear memory — no pointer leaks to host
- No `wasm32-unknown-unknown` host functions allowed unless explicitly provided
- Plugins cannot allocate unbounded memory (WASM page limits apply)

### Error Containment
- Plugin panics → caught as `Result::Err`, execution continues without corruption
- Malformed output → `PluginError::InvalidOutput`, no state mutation
- Timeout → configurable per-plugin, enforced by the host

### User Control
- Plugins can only be loaded by explicit user action (file picker)
- Users can view plugin capabilities before loading
- Plugins can be revoked at any time via the plugin manager

---

## Data Flow

```
User loads plugin (.wasm)
        ↓
Gent validates WASM structure
        ↓
Manifest read → capabilities extracted
        ↓
Capability validation
        ↓
[Success] → Plugin initialized with capability-gated context
[Failure] → Error shown, plugin not loaded
        ↓
Graph execution reaches plugin node
        ↓
Host calls plugin.process(input)
        ↓
Plugin executes in WASM sandbox with granted capabilities
        ↓
Output returned to host
        ↓
Execution continues
```

---

## File Structure

```
gent/
├── src/
│   ├── components/
│   │   ├── plugins/
│   │   │   ├── mod.rs
│   │   │   ├── host.rs        # PluginHost trait
│   │   │   ├── loader.rs      # WASM loading and validation
│   │   │   ├── capabilities.rs
│   │   │   ├── errors.rs
│   │   │   └── registry.rs    # Loaded plugin registry
│   │   └── plugin_manager.rs  # UI for managing plugins
│   └── ...
├── plugins/                   # User-installed plugins
│   └── .gitkeep
└── Cargo.toml
```

---

## Future Considerations

- **Plugin marketplace**: Signed plugins with verified capabilities
- **Plugin dependencies**: Allow plugins to depend on other plugins
- **Hot reload**: Update plugins without restarting Gent
- **Debugging tools**: Step-through plugin execution in the trace panel

---

## Open Questions

- How should plugins be versioned and updated?
- Should plugins be allowed to spawn child plugins (agent-like behavior)?
- What is the migration path if Rune's ecosystem stalls?
