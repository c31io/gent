# Gent Plugin System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a WASM plugin system in the Tauri backend with two loaders (Rune scripting, Rust WASM) and a frontend UI for managing plugins.

**Architecture:** The plugin system lives in the Tauri backend (`src-tauri/src/plugins/`) where wasmtime and Rune runtime run natively. The frontend communicates via Tauri commands. The `gent-plugin` SDK crate provides type-safe bindings for Rust plugin authors.

**Tech Stack:** Rust, wasmtime (Rust WASM loader), rune (Rune scripting), Tauri 2, serde, Leptos

---

## File Structure

```
src-tauri/src/plugins/
├── mod.rs              # Module root + PluginLoader registry
├── errors.rs           # PluginError enum
├── capabilities.rs     # Capability enum
├── plugin.rs           # dyn Plugin trait
├── host.rs             # PluginHost trait
├── registry.rs         # PluginRegistry
├── rune_loader.rs      # Rune WASM backend
└── rust_loader.rs      # Rust WASM backend (wasmtime)

src/gent-plugin/              # Plugin SDK crate
├── Cargo.toml
└── src/
    └── lib.rs                # manifest!, gent_main! macros, types

src/components/
├── mod.rs                    # Add plugins module
└── plugin_manager.rs         # Plugin manager UI

src-tauri/Cargo.toml          # Add dependencies
src-tauri/src/lib.rs          # Add plugin commands
```

---

## Task 1: Add Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add wasmtime, rune, and logging dependencies**

```toml
[dependencies]
# WASM plugin runtime
wasmtime = "22"
wasmtime-wasi = "22"

# Rune scripting
rune = "0.13"
rune-modules = "0.13"

# Utilities
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
thiserror = "1"
once_cell = "1"
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore(plugin): add wasmtime, rune, tracing dependencies"
```

---

## Task 2: Core Types (errors + capabilities)

**Files:**
- Create: `src-tauri/src/plugins/errors.rs`
- Create: `src-tauri/src/plugins/capabilities.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write tests for PluginError**

```rust
// src-tauri/src/plugins/tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_error_display() {
        let err = PluginError::UnsupportedCapability("context".into());
        assert!(err.to_string().contains("context"));
    }

    #[test]
    fn test_plugin_error_from_rune() {
        let err = PluginError::Runtime("test".into());
        assert!(matches!(err, PluginError::Runtime(_)));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gent plugins::tests --no-run`
Expected: Test binary builds, no errors yet

- [ ] **Step 3: Implement errors.rs**

```rust
// src-tauri/src/plugins/errors.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("unsupported capability: {0}")]
    UnsupportedCapability(String),

    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("invalid output: {0}")]
    InvalidOutput(String),

    #[error("initialization failed: {0}")]
    InitFailed(String),

    #[error("loader error: {0}")]
    Loader(String),

    #[error("capability denied: {0}")]
    CapabilityDenied(String),
}
```

- [ ] **Step 4: Implement capabilities.rs**

```rust
// src-tauri/src/plugins/capabilities.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Context,
    Tools,
    Memory,
    Nodes,
    Execution,
}

impl Capability {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "context" => Some(Self::Context),
            "tools" => Some(Self::Tools),
            "memory" => Some(Self::Memory),
            "nodes" => Some(Self::Nodes),
            "execution" => Some(Self::Execution),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Tools => "tools",
            Self::Memory => "memory",
            Self::Nodes => "nodes",
            Self::Execution => "execution",
        }
    }
}
```

- [ ] **Step 5: Implement mod.rs with test module**

```rust
// src-tauri/src/plugins/mod.rs

pub mod errors;
pub mod capabilities;

pub use errors::PluginError;
pub use capabilities::Capability;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add core types (PluginError, Capability)"
```

---

## Task 3: Plugin Trait and Manifest

**Files:**
- Create: `src-tauri/src/plugins/plugin.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write tests for Manifest and Plugin trait**

```rust
// src-tauri/src/plugins/tests.rs (add to existing)

#[test]
fn test_manifest_default() {
    let manifest = Manifest::default();
    assert_eq!(manifest.name, "");
    assert!(manifest.capabilities.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gent plugins::tests --no-run`
Expected: Compilation error (Manifest not defined)

- [ ] **Step 3: Implement plugin.rs**

```rust
// src-tauri/src/plugins/plugin.rs

use serde::{Deserialize, Serialize};
use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".into(),
            description: String::new(),
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input(pub serde_json::Value);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output(pub serde_json::Value);

/// Core plugin trait - implemented by both Rune and Rust loaders
pub trait Plugin: Send + Sync {
    /// Returns the plugin manifest
    fn manifest(&self) -> &Manifest;

    /// Process an input and return output
    fn process(&self, input: Input) -> Result<Output, PluginError>;

    /// Optional initialization with context
    fn init(&mut self, _context: Context) -> Result<(), PluginError> {
        Ok(())
    }

    /// Returns the plugin ID
    fn id(&self) -> &str;
}

/// Capability-gated context passed to plugins during init
#[derive(Debug, Clone)]
pub struct Context {
    capabilities: Vec<Capability>,
    // Future: host handle for calling back into Gent
}

impl Context {
    pub fn new(capabilities: Vec<Capability>) -> Self {
        Self { capabilities }
    }

    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }
}
```

- [ ] **Step 4: Update mod.rs**

```rust
pub mod errors;
pub mod capabilities;
pub mod plugin;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add Plugin trait and Manifest types"
```

---

## Task 4: PluginHost Trait

**Files:**
- Create: `src-tauri/src/plugins/host.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write test for PluginHost mock**

Remove the placeholder test. The PluginHost trait is a interface that will be implemented by Gent's execution engine later. No meaningful unit test exists at this stage — integration tests in Task 12 will cover the full flow.

- [ ] **Step 2: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS

- [ ] **Step 3: Implement host.rs**

```rust
// src-tauri/src/plugins/host.rs

use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::{Input, Output};

/// Host API provided to Gent's execution engine
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

- [ ] **Step 4: Update mod.rs**

```rust
pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add PluginHost trait"
```

---

## Task 5: PluginRegistry

**Files:**
- Create: `src-tauri/src/plugins/registry.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write tests for PluginRegistry**

```rust
// src-tauri/src/plugins/tests.rs (add to existing)

use super::*;
use std::sync::Arc;

struct TestPlugin {
    id: String,
    manifest: Manifest,
}

impl TestPlugin {
    fn new(id: &str, name: &str) -> Self {
        let mut manifest = Manifest::default();
        manifest.name = name.to_string();
        Self {
            id: id.to_string(),
            manifest,
        }
    }
}

impl Plugin for TestPlugin {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn process(&self, _input: Input) -> Result<Output, PluginError> {
        Ok(Output(serde_json::json!({"result": "ok"})))
    }

    fn id(&self) -> &str {
        &self.id
    }
}

#[test]
fn test_registry_register_and_get() {
    let registry = PluginRegistry::new();
    let plugin = Arc::new(TestPlugin::new("test-1", "Test Plugin"));
    registry.register(plugin.clone()).unwrap();

    let found = registry.get("test-1").unwrap();
    assert_eq!(found.id(), "test-1");
}

#[test]
fn test_registry_unregister() {
    let registry = PluginRegistry::new();
    let plugin = Arc::new(TestPlugin::new("test-1", "Test Plugin"));
    registry.register(plugin.clone()).unwrap();
    registry.unregister("test-1").unwrap();

    assert!(registry.get("test-1").is_none());
}

#[test]
fn test_registry_duplicate_id() {
    let registry = PluginRegistry::new();
    let p1 = Arc::new(TestPlugin::new("test-1", "Plugin 1"));
    let p2 = Arc::new(TestPlugin::new("test-1", "Plugin 2"));
    registry.register(p1).unwrap();
    let result = registry.register(p2);
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gent plugins::tests --no-run`
Expected: Compilation error (PluginRegistry not defined)

- [ ] **Step 3: Implement registry.rs**

```rust
// src-tauri/src/plugins/registry.rs

use crate::plugins::errors::PluginError;
use crate::plugins::plugin::Plugin;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new plugin, returns unique plugin_id
    pub fn register(&self, plugin: Arc<dyn Plugin>) -> Result<String, PluginError> {
        let mut plugins = self.plugins.write().map_err(|_| {
            PluginError::Runtime("registry poisoned".into())
        })?;

        // Generate unique ID
        let id = Uuid::new_v4().to_string();
        plugins.insert(id.clone(), plugin);

        Ok(id)
    }

    /// Unregister a plugin
    pub fn unregister(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().map_err(|_| {
            PluginError::Runtime("registry poisoned".into())
        })?;

        plugins.remove(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.into()))?;

        Ok(())
    }

    /// Get a plugin by ID
    pub fn get(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().ok()?;
        plugins.get(plugin_id).cloned()
    }

    /// List all plugin IDs
    pub fn list_ids(&self) -> Vec<String> {
        let plugins = self.plugins.read().unwrap();
        plugins.keys().cloned().collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Update mod.rs**

```rust
pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add PluginRegistry"
```

---

## Task 6: RustWasmLoader (wasmtime)

**Files:**
- Create: `src-tauri/src/plugins/rust_loader.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write tests for RustWasmLoader**

```rust
// src-tauri/src/plugins/tests.rs (add to existing)

use super::*;

#[test]
fn test_rust_loader_probe_rust_wasm() {
    // A minimal valid Rust-compiled WASM module
    // (wasmtime would need actual bytes - test the can_load logic)
    let loader = RustWasmLoader::new();
    // can't_load empty bytes
    assert!(!loader.can_load(&[]));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS (loader exists but basic tests pass)

- [ ] **Step 3: Implement rust_loader.rs**

```rust
// src-tauri/src/plugins/rust_loader.rs

use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::{Input, Manifest, Output, Plugin};
use std::sync::Arc;
use wasmtime::{Engine, Instance, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

/// Loader for Rust-compiled WASM plugins (wasm32-wasip2 target)
pub struct RustWasmLoader {
    engine: Engine,
}

impl RustWasmLoader {
    pub fn new() -> Result<Self, PluginError> {
        let engine = Engine::new_default()
            .map_err(|e| PluginError::Loader(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Check if bytes appear to be a Rust-compiled WASM module
    /// Rust WASM uses wasm32-wasip2 target with "wasi" module name
    fn is_rust_wasm(wasm: &[u8]) -> bool {
        // Check for WASM magic number
        if wasm.len() < 4 {
            return false;
        }
        wasm[0..4] == [0x00, 0x61, 0x73, 0x6d] // \0asm
    }
}

impl Default for RustWasmLoader {
    fn default() -> Self {
        Self::new().expect("failed to create RustWasmLoader")
    }
}

impl super::WasmLoader for RustWasmLoader {
    fn can_load(&self, wasm: &[u8]) -> bool {
        Self::is_rust_wasm(wasm)
    }

    fn load(
        &self,
        wasm: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError> {
        if !self.can_load(wasm) {
            return Err(PluginError::Loader(
                "not a valid Rust WASM module".into(),
            ));
        }

        let module = Module::from_binary(&self.engine, wasm)
            .map_err(|e| PluginError::Loader(e.to_string()))?;

        // Create a minimal plugin wrapper
        let plugin = RustWasmPlugin {
            module,
            manifest: Manifest::default(), // Will be extracted from WASM
            capabilities: capabilities.to_vec(),
        };

        Ok(Box::new(plugin))
    }
}

/// Inner plugin instance for Rust WASM
struct RustWasmPlugin {
    module: Module,
    manifest: Manifest,
    capabilities: Vec<Capability>,
}

impl Plugin for RustWasmPlugin {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn process(&self, _input: Input) -> Result<Output, PluginError> {
        // TODO: Implement actual WASM invocation via wasmtime
        // For now, return error indicating not yet implemented
        Err(PluginError::Runtime(
            "RustWasmPlugin::process not yet implemented".into(),
        ))
    }

    fn id(&self) -> &str {
        "rust-wasm-placeholder"
    }
}
```

- [ ] **Step 4: Update mod.rs to include WasmLoader trait and RustWasmLoader**

```rust
pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;
pub mod rust_loader;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;
pub use rust_loader::RustWasmLoader;

/// WASM loader trait - implemented by RuneLoader and RustWasmLoader
pub trait WasmLoader: Send + Sync {
    fn can_load(&self, wasm: &[u8]) -> bool;
    fn load(&self, wasm: &[u8], capabilities: &[Capability]) -> Result<Box<dyn Plugin>, PluginError>;
}
```

- [ ] **Step 5: Run tests and check compilation**

Run: `cargo check -p gent`
Expected: Compilation succeeds (may have warnings about unused code)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add RustWasmLoader skeleton with wasmtime"
```

---

## Task 7: RuneLoader (Rune runtime)

**Files:**
- Create: `src-tauri/src/plugins/rune_loader.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Implement rune_loader.rs**

```rust
// src-tauri/src/plugins/rune_loader.rs

use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::{Input, Manifest, Output, Plugin};

/// Loader for Rune-compiled WASM plugins
pub struct RuneLoader {
    // Rune runtime context - will be initialized per-plugin
    runtime: rune::Runtime,
}

impl RuneLoader {
    pub fn new() -> Result<Self, PluginError> {
        let runtime = rune::Runtime::new()
            .map_err(|e| PluginError::Loader(format!("rune runtime error: {}", e)))?;
        Ok(Self { runtime })
    }

    /// Check if bytes appear to be a Rune-compiled WASM module
    /// Rune WASM includes a custom "rune" module section marker
    fn is_rune_wasm(wasm: &[u8]) -> bool {
        if wasm.len() < 8 {
            return false;
        }
        // Rune WASM modules have "rune" as first custom section name
        // This is a simplified check - actual implementation may differ
        wasm.starts_with(&[0x00, 0x61, 0x73, 0x6d]) // WASM magic
    }
}

impl Default for RuneLoader {
    fn default() -> Self {
        Self::new().expect("failed to create RuneLoader")
    }
}

impl super::WasmLoader for RuneLoader {
    fn can_load(&self, wasm: &[u8]) -> bool {
        Self::is_rune_wasm(wasm)
    }

    fn load(
        &self,
        wasm: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError> {
        if !self.can_load(wasm) {
            return Err(PluginError::Loader(
                "not a valid Rune WASM module".into(),
            ));
        }

        // TODO: Actually instantiate the Rune module
        // For now, create a placeholder
        let plugin = RuneWasmPlugin {
            manifest: Manifest::default(),
            capabilities: capabilities.to_vec(),
        };

        Ok(Box::new(plugin))
    }
}

struct RuneWasmPlugin {
    manifest: Manifest,
    capabilities: Vec<Capability>,
}

impl Plugin for RuneWasmPlugin {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn process(&self, _input: Input) -> Result<Output, PluginError> {
        Err(PluginError::Runtime(
            "RuneWasmPlugin::process not yet implemented".into(),
        ))
    }

    fn id(&self) -> &str {
        "rune-wasm-placeholder"
    }
}
```

- [ ] **Step 2: Update mod.rs to include RuneLoader**

```rust
pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;
pub mod rust_loader;
pub mod rune_loader;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;
pub use rust_loader::RustWasmLoader;
pub use rune_loader::RuneLoader;

pub trait WasmLoader: Send + Sync {
    fn can_load(&self, wasm: &[u8]) -> bool;
    fn load(&self, wasm: &[u8], capabilities: &[Capability]) -> Result<Box<dyn Plugin>, PluginError>;
}
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check -p gent`
Expected: Compilation succeeds

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add RuneLoader skeleton"
```

---

## Task 8: PluginLoader Registry (loader selection)

**Files:**
- Create: `src-tauri/src/plugins/loader.rs`
- Modify: `src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write tests for PluginLoader**

```rust
// src-tauri/src/plugins/tests.rs (add to existing)

#[test]
fn test_plugin_loader_selects_correct_backend() {
    let loader = PluginLoader::new();
    // RuneLoader should not load empty bytes
    assert!(!loader.can_load(b""));
}
```

- [ ] **Step 2: Implement loader.rs**

```rust
// src-tauri/src/plugins/loader.rs

use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::Plugin;
use crate::plugins::{RuneLoader, RustWasmLoader, WasmLoader};
use std::sync::Arc;

/// Registry of WASM loaders that tries each in sequence
pub struct PluginLoader {
    loaders: Vec<Arc<dyn WasmLoader>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        let loaders = vec![
            Arc::new(RustWasmLoader::new().unwrap()) as Arc<dyn WasmLoader>,
            Arc::new(RuneLoader::new().unwrap()) as Arc<dyn WasmLoader>,
        ];
        Self { loaders }
    }

    /// Check if any loader can handle this WASM binary
    pub fn can_load(&self, wasm: &[u8]) -> bool {
        self.loaders.iter().any(|l| l.can_load(wasm))
    }

    /// Load a plugin using the appropriate loader
    pub fn load_plugin(
        &self,
        wasm: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError> {
        for loader in &self.loaders {
            if loader.can_load(wasm) {
                return loader.load(wasm, capabilities);
            }
        }
        Err(PluginError::Loader(
            "no loader found for this WASM binary".into(),
        ))
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Update mod.rs**

```rust
pub mod errors;
pub mod capabilities;
pub mod plugin;
pub mod host;
pub mod registry;
pub mod rust_loader;
pub mod rune_loader;
pub mod loader;

pub use errors::PluginError;
pub use capabilities::Capability;
pub use plugin::{Manifest, Input, Output, Plugin, Context};
pub use host::PluginHost;
pub use registry::PluginRegistry;
pub use rust_loader::RustWasmLoader;
pub use rune_loader::RuneLoader;
pub use loader::{PluginLoader, WasmLoader};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p gent plugins::tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "feat(plugin): add PluginLoader registry with backend selection"
```

---

## Task 9: Tauri Commands (plugin system bridge)

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/plugins/commands.rs`

- [ ] **Step 1: Implement commands.rs**

```rust
// src-tauri/src/plugins/commands.rs

use crate::plugins::{Manifest, PluginError, PluginLoader, PluginRegistry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

pub struct PluginState {
    pub registry: PluginRegistry,
    pub loader: PluginLoader,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadPluginRequest {
    pub wasm_bytes: Vec<u8>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub manifest: Manifest,
}

/// Load a plugin from WASM bytes
#[tauri::command]
pub fn load_plugin(
    state: State<'_, Arc<PluginState>>,
    request: LoadPluginRequest,
) -> Result<PluginInfo, String> {
    // Parse requested capabilities
    let requested_caps: Vec<_> = request
        .capabilities
        .iter()
        .filter_map(|s| crate::plugins::Capability::from_str(s))
        .collect();

    // Validate: all requested capabilities must be Gent-supported
    let supported_caps = &[crate::plugins::Capability::Context,
                           crate::plugins::Capability::Tools,
                           crate::plugins::Capability::Memory,
                           crate::plugins::Capability::Nodes,
                           crate::plugins::Capability::Execution];
    for cap in &requested_caps {
        if !supported_caps.contains(cap) {
            return Err(format!("unsupported capability: {:?}", cap));
        }
    }

    let plugin = state
        .loader
        .load_plugin(&request.wasm_bytes, &requested_caps)
        .map_err(|e| e.to_string())?;

    // Validate: plugin manifest capabilities must be subset of granted capabilities
    let manifest = plugin.manifest();
    for cap in &manifest.capabilities {
        if !requested_caps.contains(cap) {
            return Err(format!(
                "plugin {} requires {:?} capability but it was not granted",
                manifest.name, cap
            ));
        }
    }

    let manifest = plugin.manifest().clone();
    let id = state.registry.register(plugin).map_err(|e| e.to_string())?;

    Ok(PluginInfo { id, manifest })
}

/// List all loaded plugins
#[tauri::command]
pub fn list_plugins(state: State<'_, Arc<PluginState>>) -> Vec<PluginInfo> {
    state
        .registry
        .list_ids()
        .iter()
        .filter_map(|id| {
            state.registry.get(id).map(|p| PluginInfo {
                id: id.clone(),
                manifest: p.manifest().clone(),
            })
        })
        .collect()
}

/// Unload a plugin
#[tauri::command]
pub fn unload_plugin(state: State<'_, Arc<PluginState>>, plugin_id: String) -> Result<(), String> {
    state
        .registry
        .unregister(&plugin_id)
        .map_err(|e| e.to_string())
}

/// Call a plugin's process function
#[tauri::command]
pub fn call_plugin(
    state: State<'_, Arc<PluginState>>,
    plugin_id: String,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let plugin = state
        .registry
        .get(&plugin_id)
        .ok_or_else(|| format!("plugin not found: {}", plugin_id))?;

    let input = crate::plugins::Input(input);
    let output = plugin.process(input).map_err(|e| e.to_string())?;
    Ok(output.0)
}
```

- [ ] **Step 2: Update lib.rs to wire up plugin state and commands**

```rust
use std::sync::Arc;
use gent_lib::plugins::{PluginLoader, PluginRegistry};
use gent_lib::plugins::commands::{
    self, call_plugin, list_plugins, load_plugin, unload_plugin, PluginState
};

mod plugins;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let plugin_state = Arc::new(PluginState {
        registry: PluginRegistry::new(),
        loader: PluginLoader::new(),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(plugin_state)
        .invoke_handler(tauri::generate_handler![
            execute_code,
            load_plugin,
            list_plugins,
            unload_plugin,
            call_plugin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Fix Cargo.toml lib name**

The lib.rs uses `gent_lib` as the crate name. Update Cargo.toml if needed:

```toml
[lib]
name = "gent_lib"
```

- [ ] **Step 4: Run cargo check**

Run: `cargo check -p gent`
Expected: Compilation succeeds

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/
git commit -m "feat(plugin): add Tauri commands for plugin management"
```

---

## Task 10: gent-plugin SDK

**Files:**
- Create: `src/gent-plugin/Cargo.toml`
- Create: `src/gent-plugin/src/lib.rs`

- [ ] **Step 1: Create gent-plugin/Cargo.toml**

```toml
[package]
name = "gent-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Create gent-plugin/src/lib.rs**

```rust
//! gent-plugin SDK - Type-safe bindings for Gent plugin authors
//!
//! Usage:
//!
//! ```rust
//! use gent_plugin::prelude::*;
//!
//! pub fn manifest() -> Manifest {
//!     Manifest {
//!         name: "My Plugin",
//!         version: "1.0.0",
//!         description: "What it does",
//!         capabilities: vec![Capability::Context],
//!     }
//! }
//!
//! pub fn process(input: Input) -> Output {
//!     Output(serde_json::json!({ "result": "ok" }))
//! }
//!
//! #[gent_plugin::gent_main]
//! fn main() {}
//! ```

pub mod prelude {
    pub use crate::{Capability, Context, Input, Manifest, Output, gent_main, manifest};
}

use serde::{Deserialize, Serialize};

/// Plugin manifest - returned by the required `manifest()` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
}

/// Plugin input - passed to the required `process()` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input(pub serde_json::Value);

/// Plugin output - returned by the required `process()` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output(pub serde_json::Value);

/// Capability enum - plugins declare what they need
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Context,
    Tools,
    Memory,
    Nodes,
    Execution,
}

/// Capability-gated context (passed to optional `init()`)
#[derive(Debug, Clone)]
pub struct Context {
    // Placeholder for capability-gated host handle
}

/// Macro to export the WASM entry point
#[proc_macro]
pub fn gent_main(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // TODO: Implement actual WASM export logic
    item
}

/// Macro to build a Manifest with compile-time checks
#[proc_macro]
pub fn manifest(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // TODO: Implement manifest! macro
    input
}
```

- [ ] **Step 3: Run cargo check on new crate**

Run: `cargo check -p gent-plugin`
Expected: Compilation succeeds

- [ ] **Step 4: Commit**

```bash
git add src/gent-plugin/
git commit -m "feat(plugin-sdk): add gent-plugin SDK crate"
```

---

## Task 11: Frontend Plugin Manager UI

**Files:**
- Modify: `src/components/mod.rs`
- Create: `src/components/plugin_manager.rs`

- [ ] **Step 1: Implement plugin_manager.rs**

```rust
// src/components/plugin_manager.rs

use leptos::*;

#[component]
pub fn PluginManager() -> impl IntoView {
    let (plugins, set_plugins) = create_signal(Vec::<PluginInfo>::new());
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);

    // Load plugin list on mount
    on_mount(|| {
        spawn(async move {
            set_loading(true);
            match list_plugins().await {
                Ok(list) => set_plugins(list),
                Err(e) => set_error(Some(e)),
            }
            set_loading(false);
        });
    });

    view! {
        <div class="plugin-manager">
            <h2>"Plugins"</h2>

            {move || {
                if loading() {
                    view! { <p>"Loading..."</p> }
                } else if let Some(err) = error() {
                    view! { <p class="error">{err}</p> }
                } else {
                    view! {
                        <ul>
                            {plugins().iter().map(|p| view! {
                                <li>
                                    <span>{p.manifest.name}</span>
                                    <span>" v"{p.manifest.version}</span>
                                </li>
                            }).collect::<Vec<_>>()}
                        </ul>
                    }
                }
            }}
        </div>
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub manifest: Manifest,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
}

async fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    #[cfg(feature = "ssr")]
    {
        // SSR: direct call
        // Use crate::plugins::commands::list_plugins
        todo!()
    }
    #[cfg(not(feature = "ssr"))]
    {
        // CSR: use Tauri invoke
        let tauri = leptos::window().tauri();
        tauri.invoke("list_plugins").await.map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 2: Update mod.rs**

```rust
pub mod app_layout;
pub mod left_panel;
pub mod canvas;
pub mod nodes;
pub mod node_inspector;
pub mod execution_engine;
pub mod execution_trace;
pub mod plugin_manager;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check -p gent-ui`
Expected: Compilation may have warnings about unused code

- [ ] **Step 4: Commit**

```bash
git add src/components/
git add src/gent-plugin/
git commit -m "feat(plugin-ui): add plugin manager component"
```

---

## Task 12: Integration Test

**Files:**
- Create: `src-tauri/src/plugins/integration_test.rs`

- [ ] **Step 1: Write integration test**

```rust
#[cfg(test)]
mod integration_tests {
    use crate::plugins::{
        Manifest, Capability, PluginLoader, PluginRegistry, WasmLoader, RustWasmLoader,
    };

    #[test]
    fn test_load_and_unload_rust_plugin() {
        let registry = PluginRegistry::new();
        let loader = PluginLoader::new();

        // Minimal valid Rust WASM (just header, won't actually run)
        let wasm = vec![
            0x00, 0x61, 0x73, 0x6d, // WASM magic
            0x01, 0x00, 0x00, 0x00, // version 1
        ];

        // Should fail to load since manifest can't be extracted
        let result = loader.load_plugin(&wasm, &[Capability::Context]);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_ Tracks_plugins() {
        let registry = PluginRegistry::new();
        assert!(registry.list_ids().is_empty());
    }
}
```

- [ ] **Step 2: Run integration test**

Run: `cargo test -p gent --test integration`
Expected: PASS (or fail on actual load which is expected)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/plugins/
git commit -m "test(plugin): add integration tests"
```

---

## Plan Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add dependencies | src-tauri/Cargo.toml |
| 2 | Core types (errors, capabilities) | errors.rs, capabilities.rs, mod.rs |
| 3 | Plugin trait and Manifest | plugin.rs, mod.rs |
| 4 | PluginHost trait | host.rs, mod.rs |
| 5 | PluginRegistry | registry.rs, mod.rs |
| 6 | RustWasmLoader (wasmtime) | rust_loader.rs, mod.rs |
| 7 | RuneLoader | rune_loader.rs, mod.rs |
| 8 | PluginLoader registry | loader.rs, mod.rs |
| 9 | Tauri commands | commands.rs, lib.rs |
| 10 | gent-plugin SDK | gent-plugin/Cargo.toml, gent-plugin/src/lib.rs |
| 11 | Frontend UI | plugin_manager.rs, mod.rs |
| 12 | Integration test | integration_test.rs |

---

## Execution Options

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
