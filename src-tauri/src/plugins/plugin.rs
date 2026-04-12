use crate::plugins::capabilities::Capability;
use crate::plugins::errors::PluginError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".into(),
            description: String::new(),
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input(pub serde_json::Value);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output(pub serde_json::Value);

/// Core plugin trait - implemented by both Rune and Rust loaders
pub trait Plugin: Send + Sync {
    /// Returns the plugin manifest
    fn manifest(&self) -> &Manifest;

    /// Process an input and return output
    fn process(&self, input: Input) -> Result<Output, PluginError>;

    /// Returns the plugin ID
    fn id(&self) -> &str;
}

// Capability-gated context reserved for future plugin initialization API
