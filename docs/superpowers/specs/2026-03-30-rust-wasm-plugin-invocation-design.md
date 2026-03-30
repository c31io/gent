# Rust WASM Plugin Invocation via wasmtime

## Status
Approved

## Overview

Implement `RustWasmPlugin::process` to invoke wasm32-wasip2 compiled plugins using wasmtime 22, following the standard WASI args/stdout pattern.

## Architecture

### Invocation Model

Plugin is a standalone wasm32-wasip2 binary. On invocation:
1. Serialize `Input` to JSON string
2. Spawn plugin via wasmtime with JSON as command-line argument
3. Capture stdout
4. Parse stdout as JSON into `Output`

### Plugin Structure

```
Plugin WASM (wasm32-wasip2)
├── main() reads args[1] as JSON input
├── Processes logic
└── Writes JSON result to stdout, exits 0
```

### Error Handling

- **JSON error from stdout**: Plugin writes structured error JSON → return as `PluginError::Runtime`
- **Plugin panic/exit non-zero**: Wrap exit code in `PluginError::Runtime`
- **JSON parse failure**: Return `PluginError::Runtime`

### Manifest Extraction

Plugin exports `__gent_plugin_manifest` function returning JSON manifest string. Called once on `load()` to populate `RustWasmPlugin.manifest`.

Manifest format:
```json
{"name": "my-plugin", "version": "1.0.0", "description": "...", "capabilities": ["filesystem"]}
```

## Components

### `RustWasmPlugin` struct (existing)

```rust
struct RustWasmPlugin {
    module: Module,
    manifest: Manifest,
    capabilities: Vec<Capability>,
}
```

### `process()` implementation

1. Serialize `input.0` to JSON string
2. Create fresh `Store` with WASI context per invocation
3. Set `args[0]` = plugin name/id, `args[1]` = JSON input string
4. Instantiate module and call `_start` (wasip2 main entry)
5. Capture stdout as string
6. Deserialize stdout as `Output`
7. Handle errors per error handling strategy above

### `load()` manifest extraction

After loading module binary, attempt to:
1. Instantiate module
2. Call `__gent_plugin_manifest` export if present
3. Parse returned string as JSON into `Manifest`
4. Fall back to `Manifest::default()` if export missing

## Data Flow

```
Input(JSON) → process() → serialize to string → wasmtime spawn
                                                      ↓
Output(JSON) ← deserialize stdout ← capture stdout ← plugin runs
```

## Implementation Notes

- Use `wasmtime_wasi::WasiCtxBuilder` to configure args and capture stdout
- Per-invocation Store/Instance: creates fresh isolation per call, simpler and safer
- Exit code 0 = success, non-zero = error (plugin can write error details to stdout before exiting)

## TODO

- [ ] Implement `process()` with args/stdout invocation
- [ ] Add manifest extraction on `load()`
- [ ] Add error handling for stderr and non-zero exits
- [ ] Remove `// TODO: Implement actual WASM invocation via wasmtime` comment

## File Changes

- `src-tauri/src/plugins/rust_loader.rs` — implement `process()`, update `load()` for manifest extraction
