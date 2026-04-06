use serde::{Deserialize, Serialize};

/// Shared console line structure for both Rune scripts and WASM plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: String,
}

impl ConsoleLine {
    /// Create an output-level console line
    pub fn output(message: impl Into<String>) -> Self {
        Self {
            level: "output".into(),
            message: message.into(),
        }
    }

    /// Create an error-level console line
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: "error".into(),
            message: message.into(),
        }
    }
}
