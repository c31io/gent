use crate::plugins::errors::PluginError;
use rune::diagnostics::Diagnostic;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Source, Sources, Vm};
use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;

/// Unique run ID for correlating console output
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
    pub run_id: String,
}

/// Global Rune engine singleton
pub static RUNE_ENGINE: OnceLock<Arc<RuneEngine>> = OnceLock::new();

#[derive(Debug)]
pub struct RuneEngine {
    context: Context,
}

impl RuneEngine {
    /// Create a new RuneEngine
    pub fn new() -> Result<Self, PluginError> {
        let context = Context::with_default_modules()
            .map_err(|e| PluginError::Runtime(format!("context error: {}", e)))?;
        Ok(Self { context })
    }

    /// Execute a Rune script and return console lines (compile/runtime errors)
    /// Phase 1: result value is discarded, only console output matters
    pub fn run(
        &self,
        source: &str,
        input: serde_json::Value,
        run_id: &str,
    ) -> Result<Vec<ConsoleLine>, PluginError> {
        eprintln!("[DEBUG engine] run() called, run_id={}", run_id);
        let mut sources = Sources::new();
        let _ = sources.insert(Source::memory(source)
            .map_err(|e| PluginError::Runtime(format!("failed to create source: {}", e)))?);

        let mut diagnostics = Diagnostics::new();

        // Collect console lines
        let mut lines = Vec::new();

        // Build unit from sources (compiles the script)
        let result: Result<_, _> = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .build();

        // Emit compile errors to stderr (for logging) AND collect as ConsoleLine entries
        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            if let Err(e) = diagnostics.emit(&mut writer, &sources) {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("diagnostic emit failed: {}", e),
                    run_id: run_id.into(),
                });
            }
            // Also collect diagnostics as ConsoleLine entries so they appear in frontend
            for diag in diagnostics.diagnostics() {
                match diag {
                    Diagnostic::Fatal(f) => {
                        lines.push(ConsoleLine {
                            level: "error".into(),
                            message: f.to_string(),
                            run_id: run_id.into(),
                        });
                    }
                    Diagnostic::Warning(w) => {
                        lines.push(ConsoleLine {
                            level: "warning".into(),
                            message: w.to_string(),
                            run_id: run_id.into(),
                        });
                    }
                    _ => {
                        // Non-exhaustive enum, ignore unknown variants
                    }
                }
            }
        }

        // Return early if build failed, we already collected diagnostics
        let unit = result.map_err(|e| PluginError::Runtime(format!("vm build error: {}", e)))?;

        // Create a new runtime from the cached context
        let runtime = self.context.runtime()
            .map_err(|e| PluginError::Runtime(format!("failed to create runtime: {}", e)))?;
        let runtime = Arc::new(runtime);
        let unit = Arc::new(unit);
        let mut vm = Vm::new(runtime, unit);

        // Convert serde_json::Value to a String for rune.
        // Note: rune 0.13's ToValue is not implemented for serde_json::Value,
        // so we pass the input as a JSON string. Scripts receive input as a String
        // and should call input.to_string() or parse with serde_json::from_str.
        // This will be improved in a future phase.
        let input_value = serde_json::to_string(&input)
            .map_err(|e| PluginError::Runtime(format!("failed to serialize input: {}", e)))?;

        lines.push(ConsoleLine {
            level: "info".into(),
            message: "--- calling main ---".into(),
            run_id: run_id.into(),
        });

        eprintln!("[DEBUG engine] vm.call starting, input={}", input_value);

        match vm.call(["main"], (input_value,)) {
            Ok(_output) => {
                eprintln!("[DEBUG engine] vm.call succeeded");
                lines.push(ConsoleLine {
                    level: "info".into(),
                    message: "--- main returned ---".into(),
                    run_id: run_id.into(),
                });
                // Phase 1: ignore output value, only console lines matter
            }
            Err(e) => {
                eprintln!("[DEBUG engine] vm.call error: {}", e);
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
