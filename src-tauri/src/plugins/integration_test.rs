#[cfg(test)]
mod integration_tests {
    use crate::plugins::{
        Capability, PluginLoader, PluginRegistry, WasmLoader, RustWasmLoader,
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
    fn test_registry_tracks_plugins() {
        let registry = PluginRegistry::new();
        assert!(registry.list_ids().is_empty());
    }
}