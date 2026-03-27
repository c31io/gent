//! gent-plugin SDK - Type-safe bindings for Gent plugin authors
//!
//! Usage:
//!
//! ```rust
//! use gent_plugin::prelude::*;
//!
//! pub fn manifest() -> Manifest {
//!     Manifest {
//!         name: "My Plugin",
//!         version: "1.0.0",
//!         description: "What it does",
//!         capabilities: vec![Capability::Context],
//!     }
//! }
//!
//! pub fn process(input: Input) -> Output {
//!     Output(serde_json::json!({ "result": "ok" }))
//! }
//!
//! #[gent_plugin::gent_main]
//! fn main() {}
//! ```

pub mod prelude {
    pub use crate::{Capability, Context, Input, Manifest, Output};
}

use serde::{Deserialize, Serialize};

/// Plugin manifest - returned by the required `manifest()` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
}

/// Plugin input - passed to the required `process()` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input(pub serde_json::Value);

/// Plugin output - returned by the required `process()` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output(pub serde_json::Value);

/// Capability enum - plugins declare what they need
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Context,
    Tools,
    Memory,
    Nodes,
    Execution,
}

/// Capability-gated context (passed to optional `init()`)
#[derive(Debug, Clone)]
pub struct Context {
    // Placeholder for capability-gated host handle
}

/// WASM export macro - marks the main function for WASM export
///
/// Note: This is a placeholder. Actual WASM binding requires additional tooling.
#[macro_export]
macro_rules! gent_main {
    ($item:item) => {
        $item
    };
}

/// Manifest helper macro for compile-time validation
///
/// Note: This is a placeholder for future compile-time checks.
#[macro_export]
macro_rules! manifest {
    ($($tt:tt)*) => {
        ::serde_json::json!($($tt)*)
    };
}
