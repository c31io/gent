# Gent Plugin System Design

**Date:** 2026-03-27
**Status:** Approved
**Author:** c31io

---

## Overview

Gent's key differentiator is a first-class plugin system powered by WASM running inside a Tauri desktop application. Plugins are compiled to `.wasm` modules and loaded at runtime with capability-based security.

Gent supports **two plugin backends** via a unified interface:
- **Rune** — scripting for rapid iteration, no Rust toolchain needed
- **Rust** — full WASM compilation for power users who need the Rust ecosystem

This document describes the plugin architecture, capability model, lifecycle, host API, and security model.

---

## Why WASM + Two Backends

| Aspect | Lua | Rune scripting | Rust WASM |
|--------|-----|----------------|-----------|
| Rust integration | via mlua/rlua | Native | Native |
| WASM support | Limited | First-class | First-class |
| Ecosystem maturity | Battle-tested | Early-stage | Battle-tested |
| Learning curve | Low | Moderate | High |
| Access to crates.io | No | Limited | Full |
| Iteration speed | Fast | Fast | Slow (compile) |

**Design decision:** Rune for scripting (fast iteration, low barrier), Rust WASM for power (full ecosystem access). Both share the same `Plugin` interface and `PluginHost` API.

**Key insight:** Rune and Rust WASM both compile to `.wasm` — Gent's loader can treat them identically at the interface level, with different compilation/packaging flows.

---

## Plugin Structure

All plugins expose the same required exports, regardless of backend:

### Required Exports

| Export | Type | Description |
|--------|------|-------------|
| `manifest()` | `fn() -> Manifest` | Plugin metadata and capability requirements |
| `process(input)` | `fn(Input) -> Output` | Main entry point for plugin execution |

### Optional Exports

| Export | Type | Description |
|--------|------|-------------|
| `init(ctx)` | `fn(Context) -> ()` | One-time initialization with capability-gated handle |

### Rune Plugin Example

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
    // plugin logic in Rune
}
```

### Rust Plugin Example

```rust
// lib.rs
use gent_plugin::prelude::*;

pub fn manifest() -> Manifest {
    Manifest {
        name: "My Plugin",
        version: "1.0.0",
        description: "What it does",
        capabilities: vec!["context".into(), "tools".into()],
    }
}

pub fn process(input: Input) -> Output {
    // plugin logic in Rust, compiled to wasm32-wasip2
}

#[gent_plugin::gent_main]
fn main() {}
```

### gent-plugin SDK

Rust plugin authors use the `gent-plugin` crate for type-safe bindings:

```toml
# Cargo.toml (plugin project)
[dependencies]
gent-plugin = { path = "path/to/gent/src/gent-plugin" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

The SDK provides:

- `#[gent_plugin::gent_main]` — marks the WASM entry point
- `manifest!()` macro — builds `Manifest` with compile-time validation
- `Input` / `Output` types — JSON-serializable plugin I/O
- `Context` — capability-gated host access
- `Capability` enum — typed capability declarations

---

## Loader Architecture

```
┌─────────────────────────────────────────────┐
│              PluginRegistry                  │
│  (tracks all loaded plugins, their state)   │
└─────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│            dyn Plugin trait                  │
│   fn manifest() -> Manifest                  │
│   fn process(input) -> Result<Output, Err>   │
│   fn init(ctx: Context) -> Result<(), Err>  │
└─────────────────────────────────────────────┘
                      │
         ┌────────────┴────────────┐
         ▼                         ▼
┌─────────────────┐      ┌─────────────────────┐
│   RuneLoader    │      │    RustWasmLoader    │
│                 │      │                     │
│ - Loads .wasm   │      │ - Loads .wasm       │
│   compiled from │      │   compiled from      │
│   Rune source  │      │   Rust/wasm-bindgen  │
│                │      │                     │
│ - Uses rune    │      │ - Uses wasmtime      │
│   runtime     │      │   or similar         │
└─────────────────┘      └─────────────────────┘
```

### Loader Trait

```rust
trait WasmLoader: Send + Sync {
    /// Probe wasm bytes to check if this loader can handle it
    fn can_load(&self, wasm: &[u8]) -> bool;

    /// Load and instantiate the plugin
    fn load(
        &self,
        wasm: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError>;
}

struct RuneLoader { /* rune runtime instance */ }
struct RustWasmLoader { /* wasmtime instance */ }
```

### Loader Selection

- Gent probes the `.wasm` binary to determine which loader to use
- Rune-compiled WASM includes a known module name prefix or custom section
- Rust-compiled WASM uses standard `wasm32-wasip2` target
- A `PluginLoader` registry tries loaders in sequence until one succeeds

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
│   │   │   ├── host.rs           # PluginHost trait
│   │   │   ├── loader.rs         # PluginLoader registry + selection
│   │   │   ├── rune_loader.rs    # Rune WASM backend
│   │   │   ├── rust_loader.rs    # Rust WASM backend (wasmtime)
│   │   │   ├── capabilities.rs
│   │   │   ├── errors.rs
│   │   │   ├── registry.rs       # Loaded plugin registry
│   │   │   └── plugin_manager.rs # UI for managing plugins
│   │   └── ...
│   └── gent-plugin/              # Plugin SDK (for Rust plugin authors)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs            # manifest!, gent_main! macros
├── plugins/                      # User-installed plugins
│   └── .gitkeep
└── Cargo.toml
```

---

## Future Considerations

- **Plugin marketplace**: Not planned — manual loading only
- **Plugin dependencies**: Plugins can call other plugins via namespacing
- **Hot reload**: Update plugins without restarting Gent
- **Debugging tools**: Step-through plugin execution in the trace panel
- **WASM component model**: Align with emerging WASM component model standard

---

## Open Questions

- How should plugins be versioned and updated?
- Should plugins be allowed to spawn child plugins (agent-like behavior)?
- What is the migration path if Rune's ecosystem stalls?
- How to detect Rune vs Rust WASM at load time (module name prefix, custom section)?
