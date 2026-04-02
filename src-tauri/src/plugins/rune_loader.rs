use crate::plugins::capabilities::Capability;
use crate::plugins::console::ConsoleLine;
use crate::plugins::errors::PluginError;
use crate::plugins::plugin::{Input, Manifest, Output, Plugin};
use crate::scripts::engine::RUNE_ENGINE;
use std::sync::{Arc, Mutex};

/// Rune script plugin - wraps a Rune script source for use as a Plugin
pub struct RuneScriptPlugin {
    manifest: Manifest,
    source: String,
    console_lines: Arc<Mutex<Vec<ConsoleLine>>>,
}

impl RuneScriptPlugin {
    pub fn new(source: String, manifest: Manifest) -> Self {
        Self {
            manifest,
            source,
            console_lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Plugin for RuneScriptPlugin {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn process(&self, input: Input) -> Result<Output, PluginError> {
        let engine = RUNE_ENGINE.get().ok_or_else(|| {
            PluginError::Runtime("Rune engine not initialized".into())
        })?;

        let lines = engine.run(&self.source, input.0)?;

        // Collect console lines
        let mut console = self.console_lines.lock().unwrap();
        console.extend(lines);

        // Find the result output from the script execution
        // The script should have returned #{ result: ... } or similar
        // For now, return the input as-is to allow chaining
        Ok(Output(serde_json::json!({
            "status": "complete",
            "plugin_id": self.id()
        })))
    }

    fn id(&self) -> &str {
        &self.manifest.name
    }
}

/// Loader for Rune script plugins
pub struct RunePluginLoader;

impl RunePluginLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RunePluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl super::PluginSource for RunePluginLoader {
    fn can_load(&self, extension: Option<&str>) -> bool {
        extension == Some("rn")
    }

    fn load(
        &self,
        source: &[u8],
        capabilities: &[Capability],
    ) -> Result<Box<dyn Plugin>, PluginError> {
        let source = std::str::from_utf8(source)
            .map_err(|e| PluginError::Loader(format!("invalid UTF-8: {}", e)))?;

        let manifest = Manifest {
            name: String::new(), // Will be set by caller
            version: "0.1.0".into(),
            description: source.lines().next()
                .map(|l| l.trim_start_matches("//").trim().to_string())
                .unwrap_or_default(),
            capabilities: capabilities.to_vec(),
        };

        Ok(Box::new(RuneScriptPlugin::new(source.to_string(), manifest)))
    }
}