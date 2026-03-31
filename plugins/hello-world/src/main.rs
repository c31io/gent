//! Hello World Plugin for Gent
//!
//! This plugin demonstrates a minimal WASM plugin that:
//! - Receives JSON input via command line arguments
//! - Outputs JSON result to stdout
//!
//! Build: cargo build --release --target wasm32-unknown-unknown

use serde::{Deserialize, Serialize};

/// Input structure - parsed from command line argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input(pub serde_json::Value);

/// Output structure - written to stdout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output(pub serde_json::Value);

/// Main entry point - called by Gent's wasmtime runtime via _start
#[no_mangle]
pub extern "C" fn _start() {
    // Get command line arguments (arg[0] = plugin_id, arg[1] = input_json)
    let args: Vec<String> = std::env::args().collect();

    // Parse input JSON from args[1]
    let input_json = args.get(1).map(|s| s.as_str()).unwrap_or("{}");
    let input: serde_json::Value = serde_json::from_str(input_json).unwrap_or_default();

    // Build output
    let output = Output(serde_json::json!({
        "greeting": "Hello, World!",
        "input_received": input,
        "plugin": "hello-world",
    }));

    // Write JSON output to stdout
    serde_json::to_writer(std::io::stdout(), &output).unwrap();
}

/// Dummy main for binary crate
fn main() {}