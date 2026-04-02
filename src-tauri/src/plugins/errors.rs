use serde::Serialize;
use std::io;
use thiserror::Error;

#[derive(Clone, Error, Debug, Serialize)]
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

    #[error("extension no found: {0}")]
    ExtensionNotFound(String),

    #[error("initialization failed: {0}")]
    InitFailed(String),

    #[error("loader error: {0}")]
    Loader(String),

    #[error("capability denied: {0}")]
    CapabilityDenied(String),
}

impl From<io::Error> for PluginError {
    fn from(e: io::Error) -> Self {
        PluginError::Loader(format!("I/O error: {}", e))
    }
}
