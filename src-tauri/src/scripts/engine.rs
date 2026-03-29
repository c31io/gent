use crate::plugins::errors::PluginError;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Source, Sources, Vm};
use serde::Serialize;
use std::sync::OnceLock;

use std::sync::Arc as StdArc;

/// Unique run ID for correlating console output
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
    pub run_id: String,
}

/// Global Rune engine singleton
pub static RUNE_ENGINE: OnceLock<StdArc<RuneEngine>> = OnceLock::new();

#[derive(Debug)]
pub struct RuneEngine;

impl RuneEngine {
    /// Create a new RuneEngine
    pub fn new() -> Result<Self, PluginError> {
        Ok(Self)
    }

    /// Execute a Rune script and return console lines (compile/runtime errors)
    /// Phase 1: result value is discarded, only console output matters
    pub fn run(
        &self,
        source: &str,
        input: serde_json::Value,
        run_id: &str,
    ) -> Result<Vec<ConsoleLine>, PluginError> {
        let mut sources = Sources::new();
        let _ = sources.insert(Source::memory(source)
            .map_err(|e| PluginError::Runtime(format!("failed to create source: {}", e)))?);

        let mut diagnostics = Diagnostics::new();
        let context = Context::with_default_modules()
            .map_err(|e| PluginError::Runtime(format!("context error: {}", e)))?;

        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        // Collect console lines
        let mut lines = Vec::new();

        // Emit compile errors
        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            if let Err(e) = diagnostics.emit(&mut writer, &sources) {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("diagnostic emit failed: {}", e),
                    run_id: run_id.into(),
                });
            }
        }

        let unit = result.map_err(|e| {
            lines.push(ConsoleLine {
                level: "error".into(),
                message: format!("vm build error: {}", e),
                run_id: run_id.into(),
            });
            PluginError::Runtime(format!("vm build failed: {}", e))
        })?;

        // Create a new runtime for this execution
        let runtime = context.runtime()
            .map_err(|e| PluginError::Runtime(format!("failed to create runtime: {}", e)))?;
        let runtime = StdArc::new(runtime);
        let unit = StdArc::new(unit);
        let mut vm = Vm::new(runtime, unit);

        // Convert serde_json::Value to a Rune Value
        // For Phase 1, we pass a simple string representation
        let input_value = serde_json::to_string(&input)
            .map_err(|e| PluginError::Runtime(format!("failed to serialize input: {}", e)))?;

        match vm.call(["process"], (input_value,)) {
            Ok(_output) => {
                // Phase 1: ignore output value, only console lines matter
            }
            Err(e) => {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("runtime error: {}", e),
                    run_id: run_id.into(),
                });
            }
        }

        Ok(lines)
    }
}