use crate::plugins::capabilities::Capability;
use crate::plugins::console::ConsoleLine;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::{Input, Manifest, Output, Plugin};
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::pipe::MemoryOutputPipe;
use wasmtime_wasi::preview1::WasiP1Ctx;

/// Captures stdout/stderr from a WASI command invocation
struct CapturedOutput {
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
}

impl CapturedOutput {
    fn new() -> Self {
        Self {
            stdout: MemoryOutputPipe::new(4096),
            stderr: MemoryOutputPipe::new(4096),
        }
    }

    fn into_contents(self) -> (Vec<u8>, Vec<u8>) {
        (self.stdout.contents().to_vec(), self.stderr.contents().to_vec())
    }
}

/// Loader for WASM plugins using wasmtime
pub struct WasmPluginLoader {
    engine: Engine,
}

impl WasmPluginLoader {
    pub fn new() -> Result<Self, PluginError> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

    /// Check if bytes appear to be a valid WASM module
    fn is_wasm(wasm: &[u8]) -> bool {
        // Check for WASM magic number
        if wasm.len() < 4 {
            return false;
        }
        wasm[0..4] == [0x00, 0x61, 0x73, 0x6d] // \0asm
    }
}

fn build_wasi_ctx(
    plugin_id: &str,
    input_json: &str,
    captured: &CapturedOutput,
) -> WasiP1Ctx {
    WasiCtxBuilder::new()
        .args(&[plugin_id, input_json])
        .stdout(captured.stdout.clone())
        .stderr(captured.stderr.clone())
        .build_p1()
}

fn parse_output(captured: CapturedOutput) -> Result<Output, PluginError> {
    let (stdout, _stderr) = captured.into_contents();
    let stdout_str = String::from_utf8(stdout)
        .map_err(|e| PluginError::Runtime(format!("invalid utf-8 from plugin stdout: {}", e)))?;

    serde_json::from_str::<serde_json::Value>(&stdout_str)
        .map(Output)
        .map_err(|e| PluginError::Runtime(format!("invalid JSON from plugin: {}", e)))
}

impl Default for WasmPluginLoader {
    fn default() -> Self {
        Self::new().expect("failed to create WasmPluginLoader")
    }
}

impl super::PluginSource for WasmPluginLoader {
    fn can_load(&self, extension: Option<&str>) -> bool {
        extension == Some("wasm")
    }

    fn load(
        &self,
        wasm: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError> {
        let module = Module::from_binary(&self.engine, wasm)
            .map_err(|e| PluginError::Loader(e.to_string()))?;

        // Create a minimal plugin wrapper
        let plugin = WasmPluginInstance {
            engine: self.engine.clone(),
            module,
            manifest: Manifest::default(),
            capabilities: capabilities.to_vec(),
            console_lines: Arc::new(Mutex::new(Vec::new())),
        };

        Ok(Box::new(plugin))
    }
}

/// Inner plugin instance for WASM plugins
struct WasmPluginInstance {
    engine: Engine,
    module: Module,
    manifest: Manifest,
    capabilities: Vec<Capability>,
    console_lines: Arc<Mutex<Vec<ConsoleLine>>>,
}

impl Plugin for WasmPluginInstance {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn process(&self, input: Input) -> Result<Output, PluginError> {
        let input_json = serde_json::to_string(&input.0)
            .map_err(|e| PluginError::Runtime(format!("failed to serialize input: {}", e)))?;

        let captured = CapturedOutput::new();
        let wasi = build_wasi_ctx(self.id(), &input_json, &captured);

        let mut store = Store::new(&self.engine, wasi);

        // Set up WASI linking
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| cx)
            .map_err(|e| PluginError::Runtime(format!("failed to set up WASI: {}", e)))?;

        // Set up log::println host import for console capture
        let console_lines = self.console_lines.clone();
        linker.func_wrap("log", "println", move |mut caller: wasmtime::Caller<'_, WasiP1Ctx>, ptr: i32, len: i32| {
            use wasmtime::Extern;
            if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
                let mut buffer = vec![0u8; len as usize];
                if memory.read(&mut caller, ptr as usize, &mut buffer).is_ok() {
                    if let Ok(msg) = String::from_utf8(buffer) {
                        console_lines.lock().unwrap().push(ConsoleLine::output(msg));
                    }
                }
            }
        }).map_err(|e| PluginError::Runtime(format!("failed to register log::println: {}", e)))?;

        // Instantiate - WASI imports are auto-linked via the linker
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| PluginError::Runtime(format!("failed to instantiate plugin: {}", e)))?;

        // Find entry point - try __main_argc_argv first (wasip2), then _start (wasip1)
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "__main_argc_argv")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut store, "_start"))
            .map_err(|e| PluginError::Runtime(format!("failed to find main entry: {}", e)))?;

        // Call the entry point - proc_exit(0) succeeds, proc_exit(N) traps with error
        start.call(&mut store, ())
            .map_err(|e| PluginError::Runtime(format!("plugin execution failed: {}", e)))?;

        // Collect console lines from the plugin execution (intentionally unused - deferred integration)
        let mut console_lines = self.console_lines.lock().unwrap();
        let _captured_lines: Vec<ConsoleLine> = console_lines.drain(..).collect();
        drop(console_lines);

        parse_output(captured)
    }

    fn id(&self) -> &str {
        &self.manifest.name
    }
}