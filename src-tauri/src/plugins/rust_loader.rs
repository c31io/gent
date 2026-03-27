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