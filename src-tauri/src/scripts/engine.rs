use crate::plugins::console::ConsoleLine;
use crate::plugins::errors::PluginError;
use rune::diagnostics::Diagnostic;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Module, Source, Sources, Vm};
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::OnceLock;

thread_local! {
    /// Log output buffer populated by log::println in Rune, drained after execution
    static LOG_OUTPUT: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Rune log module function: `log::println(message)` appends to LOG_OUTPUT
fn log_println(message: String) {
    LOG_OUTPUT.with(|cell| cell.borrow_mut().push(message));
}

/// Build a log module containing the print callback
fn build_log_module() -> Result<Module, PluginError> {
    let mut module = Module::with_item(["log"])
        .map_err(|e| PluginError::Runtime(format!("failed to create log module: {}", e)))?;
    module
        .function("println", log_println)
        .build()
        .map_err(|e| PluginError::Runtime(format!("failed to register log::println: {}", e)))?;
    Ok(module)
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
        let mut context = Context::with_default_modules()
            .map_err(|e| PluginError::Runtime(format!("context error: {}", e)))?;

        let log_module = build_log_module()?;
        context
            .install(log_module)
            .map_err(|e| PluginError::Runtime(format!("log module install error: {}", e)))?;

        Ok(Self { context })
    }

    /// Execute a Rune script and return console lines (compile/runtime errors)
    pub fn run(
        &self,
        source: &str,
        input: serde_json::Value,
    ) -> Result<Vec<ConsoleLine>, PluginError> {
        let mut sources = Sources::new();
        let _ = sources.insert(
            Source::memory(source)
                .map_err(|e| PluginError::Runtime(format!("failed to create source: {}", e)))?,
        );

        let mut diagnostics = Diagnostics::new();
        let mut lines = Vec::new();

        // Build unit from sources (compiles the script)
        let result: Result<_, _> = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .build();

        // Emit compile errors to stderr AND collect as ConsoleLine entries
        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            if let Err(e) = diagnostics.emit(&mut writer, &sources) {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("diagnostic emit failed: {}", e),
                });
            }
            for diag in diagnostics.diagnostics() {
                match diag {
                    Diagnostic::Fatal(f) => {
                        lines.push(ConsoleLine {
                            level: "error".into(),
                            message: f.to_string(),
                        });
                    }
                    Diagnostic::Warning(w) => {
                        lines.push(ConsoleLine {
                            level: "warning".into(),
                            message: w.to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }

        let unit = result.map_err(|e| PluginError::Runtime(format!("vm build error: {}", e)))?;

        let runtime = self
            .context
            .runtime()
            .map_err(|e| PluginError::Runtime(format!("failed to create runtime: {}", e)))?;
        let runtime = Arc::new(runtime);
        let unit = Arc::new(unit);
        let mut vm = Vm::new(runtime, unit);

        let input_value = serde_json::to_string(&input)
            .map_err(|e| PluginError::Runtime(format!("failed to serialize input: {}", e)))?;

        // Clear thread-local log buffer before execution
        LOG_OUTPUT.with(|cell| cell.borrow_mut().clear());

        match vm.call(["main"], (input_value,)) {
            Ok(_) => {
                LOG_OUTPUT.with(|cell| {
                    for msg in cell.borrow_mut().drain(..) {
                        lines.push(ConsoleLine {
                            level: "output".into(),
                            message: msg,
                        });
                    }
                });
            }
            Err(e) => {
                lines.push(ConsoleLine {
                    level: "error".into(),
                    message: format!("runtime error: {}", e),
                });
            }
        }

        Ok(lines)
    }
}
