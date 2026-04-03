# LLM Node Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `llm` node type with multi-format API support (OpenAI, Anthropic, OpenAI-compatible), S/M/L model size selection, per-node API key with env var fallback, and separate output ports.

**Architecture:**
- **Backend (Rust/Tauri):** New `llm.rs` module with a `llm_complete` Tauri command that uses `reqwest` to call the API. Formats OpenAI and Anthropic requests, parses responses, extracts text/tokens/model/finish_reason.
- **Frontend (Leptos/WASM):** New `NodeVariant::Llm` in state, config fields rendered in node body, execution via async Tauri invoke from `app_layout.rs`.

**Tech Stack:** Rust (Tauri backend), Leptos (frontend), `reqwest` (HTTP client), `serde_json` (JSON parsing)

**Spec:** `docs/superpowers/specs/2026-04-03-llm-node-design.md`

---

## Task 1: Add reqwest dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add reqwest with json and rustls-tls features**

```toml
# Add to [dependencies] in src-tauri/Cargo.toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/Cargo.toml && git commit -m "chore: add reqwest for LLM API calls"
```

---

## Task 2: Create LLM backend module

**Files:**
- Create: `src-tauri/src/llm.rs`

- [ ] **Step 1: Write the llm.rs module**

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub format: String,       // "openai" | "anthropic" | "openai-compatible"
    pub model_size: String,   // "S" | "M" | "L"
    pub api_key: String,
    pub custom_url: String,   // for openai-compatible
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInput {
    pub prompt: String,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOutput {
    pub text: String,
    pub tokens_used: u32,
    pub model: String,
    pub finish_reason: String,
    pub error: String,
}

fn get_model_for_size(format: &str, size: &str) -> &'static str {
    match (format, size) {
        ("openai", "S") => "gpt-4o-mini",
        ("openai", "M") => "gpt-4o",
        ("openai", "L") => "gpt-4-turbo",
        ("anthropic", "S") => "claude-3-5-haiku-20241022",
        ("anthropic", "M") => "claude-3-5-sonnet-latest",
        ("anthropic", "L") => "claude-3-5-opus-latest",
        ("openai-compatible", "S") => "gpt-4o-mini",
        ("openai-compatible", "M") => "gpt-4o",
        ("openai-compatible", "L") => "gpt-4-turbo",
        _ => "gpt-4o",
    }
}

fn resolve_api_key(config: &LlmConfig) -> Result<String, String> {
    if !config.api_key.is_empty() {
        return Ok(config.api_key.clone());
    }
    let env_key = match config.format.as_str() {
        "openai" | "openai-compatible" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => return Err("unknown format".to_string()),
    };
    env::var(env_key).map_err(|_| format!("missing env var {}", env_key))
}

// OpenAI response shapes
#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,  // can be null in edge cases
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,  // can be null when content_filter triggers
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: OpenAiUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    total_tokens: u32,
}

// Anthropic response shapes
#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,  // "text" or "thinking"
    text: Option<String>,
    thinking: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<AnthropicContentBlock>,
    usage: AnthropicUsage,
    stop_reason: Option<String>,  // can be null
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

pub async fn llm_complete(config: LlmConfig, input: LlmInput) -> LlmOutput {
    let model = get_model_for_size(&config.format, &config.model_size);
    let api_key = match resolve_api_key(&config) {
        Ok(k) => k,
        Err(e) => {
            return LlmOutput {
                text: String::new(),
                tokens_used: 0,
                model: String::new(),
                finish_reason: String::new(),
                error: e,
            }
        }
    };

    let client = Client::new();

    match config.format.as_str() {
        "openai" | "openai-compatible" => {
            let url = if config.format == "openai-compatible" && !config.custom_url.is_empty() {
                format!("{}/chat/completions", config.custom_url.trim_end_matches('/'))
            } else {
                "https://api.openai.com/v1/chat/completions".to_string()
            };

            let body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": input.prompt}],
                "temperature": input.temperature
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match resp {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return LlmOutput {
                            text: String::new(),
                            tokens_used: 0,
                            model: model.to_string(),
                            finish_reason: String::new(),
                            error: format!("HTTP {}: {}", status, text),
                        };
                    }
                    match response.json::<OpenAiResponse>().await {
                        Ok(data) => LlmOutput {
                            text: data.choices.first().and_then(|c| c.message.content.clone()).unwrap_or_default(),
                            tokens_used: data.usage.total_tokens,
                            model: data.model,
                            finish_reason: data
                                .choices
                                .first()
                                .and_then(|c| c.finish_reason.clone())
                                .unwrap_or_default(),
                            error: String::new(),
                        },
                        Err(e) => LlmOutput {
                            text: String::new(),
                            tokens_used: 0,
                            model: model.to_string(),
                            finish_reason: String::new(),
                            error: format!("parse error: {}", e),
                        },
                    }
                }
                Err(e) => LlmOutput {
                    text: String::new(),
                    tokens_used: 0,
                    model: model.to_string(),
                    finish_reason: String::new(),
                    error: format!("request failed: {}", e),
                },
            }
        }
        "anthropic" => {
            let url = "https://api.anthropic.com/v1/messages";
            let body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": input.prompt}],
                "max_tokens": 1024,
                "temperature": input.temperature
            });

            let resp = client
                .post(url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match resp {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return LlmOutput {
                            text: String::new(),
                            tokens_used: 0,
                            model: model.to_string(),
                            finish_reason: String::new(),
                            error: format!("HTTP {}: {}", status, text),
                        };
                    }
                    match response.json::<AnthropicResponse>().await {
                        Ok(data) => {
                            // Filter to only text blocks, skip thinking blocks
                            let text = data
                                .content
                                .iter()
                                .filter(|b| b.block_type == "text")
                                .filter_map(|b| b.text.clone())
                                .collect::<Vec<_>>()
                                .join("\n");
                            LlmOutput {
                                text,
                                tokens_used: data.usage.input_tokens.saturating_add(data.usage.output_tokens),
                                model: data.model,
                                finish_reason: data.stop_reason.unwrap_or_default(),
                                error: String::new(),
                            }
                        }
                        Err(e) => LlmOutput {
                            text: String::new(),
                            tokens_used: 0,
                            model: model.to_string(),
                            finish_reason: String::new(),
                            error: format!("parse error: {}", e),
                        },
                    }
                }
                Err(e) => LlmOutput {
                    text: String::new(),
                    tokens_used: 0,
                    model: model.to_string(),
                    finish_reason: String::new(),
                    error: format!("request failed: {}", e),
                },
            }
        }
        _ => LlmOutput {
            text: String::new(),
            tokens_used: 0,
            model: String::new(),
            finish_reason: String::new(),
            error: "unknown format".to_string(),
        },
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/llm.rs && git commit -m "feat(llm): add LLM backend module with OpenAI and Anthropic support"
```

---

## Task 3: Register Tauri command

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add llm module, import, and command**

In `src-tauri/src/lib.rs`, add `mod llm;` at the top with the other modules. Then add the command:

```rust
mod llm;

#[tauri::command]
async fn llm_complete(
    config: llm::LlmConfig,
    input: llm::LlmInput,
) -> Result<llm::LlmOutput, String> {
    Ok(llm::llm_complete(config, input).await)
}
```

- [ ] **Step 2: Add `llm_complete` to invoke_handler**

```rust
invoke_handler(tauri::generate_handler![
    // ... existing handlers ...
    llm_complete,
])
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/llm.rs && git commit -m "feat(llm): add llm_complete Tauri command"
```

---

## Task 4: Add LLM state (NodeVariant, ports, defaults)

**Files:**
- Modify: `src/components/canvas/state.rs`

- [ ] **Step 1: Add LlmConfig struct and Llm NodeVariant enum**

Add near the top of the file, after the `PortType` enum:

```rust
/// LLM node configuration
#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub format: String,       // "openai" | "anthropic" | "openai-compatible"
    pub model_size: String,   // "S" | "M" | "L"
    pub api_key: String,
    pub custom_url: String,
}
```

Add to `NodeVariant` enum:

```rust
LLM { config: LlmConfig },
```

- [ ] **Step 2: Add llm ports to default_ports_for_type()**

```rust
"llm" => vec![
    Port { name: "prompt".into(), port_type: PortType::Text, direction: PortDirection::In },
    Port { name: "temperature".into(), port_type: PortType::Text, direction: PortDirection::In },
    Port { name: "text".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "tokens_used".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "model".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "finish_reason".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "error".into(), port_type: PortType::Text, direction: PortDirection::Out },
],
```

- [ ] **Step 3: Add llm to default_variant_for_type()**

```rust
"llm" => NodeVariant::LLM { config: LlmConfig {
    format: "openai".into(),
    model_size: "M".into(),
    api_key: String::new(),
    custom_url: String::new(),
}},
```

- [ ] **Step 4: Commit**

```bash
git add src/components/canvas/state.rs && git commit -m "feat(llm): add NodeVariant::LLM and port definitions"
```

---

## Task 5: Register llm in left panel

**Files:**
- Modify: `src/components/left_panel.rs`

- [ ] **Step 1: Add NodeType to NODE_TYPES array (in Agent category)**

```rust
NodeType {
    id: "llm",
    name: "LLM",
    category: "Agent",
    description: "Call an LLM API (OpenAI, Anthropic, Ollama)",
},
```

- [ ] **Step 2: Commit**

```bash
git add src/components/left_panel.rs && git commit -m "feat(llm): register llm node type in left panel"
```

---

## Task 6: Render LLM node body in frontend

**Files:**
- Modify: `src/components/nodes/node.rs` (add Llm case to `render_variant_body`)

- [ ] **Step 1: Add Llm case to render_variant_body match**

```rust
NodeVariant::LLM { config } => view! {
    <div class="node-variant-fields">
        <div class="node-variant-field">
            <label>"Format"</label>
            <select class="node-variant-select">
                <option value="openai" selected={config.format == "openai"}>"OpenAI"</option>
                <option value="anthropic" selected={config.format == "anthropic"}>"Anthropic"</option>
                <option value="openai-compatible" selected={config.format == "openai-compatible"}>"OpenAI Compatible"</option>
            </select>
        </div>
        <div class="node-variant-field">
            <label>"Size"</label>
            <select class="node-variant-select">
                <option value="S" selected={config.model_size == "S"}>"S"</option>
                <option value="M" selected={config.model_size == "M"}>"M"</option>
                <option value="L" selected={config.model_size == "L"}>"L"</option>
            </select>
        </div>
        <div class="node-variant-field">
            <label>"API Key"</label>
            <input
                type="password"
                class="node-variant-input"
                value={config.api_key.clone()}
                placeholder="key or leave empty for env"
            />
        </div>
        {if config.format == "openai-compatible" {
            view! {
                <div class="node-variant-field">
                    <label>"Custom URL"</label>
                    <input
                        type="text"
                        class="node-variant-input"
                        value={config.custom_url.clone()}
                        placeholder="http://localhost:11434/v1"
                    />
                </div>
            }.into_any()
        } else {
            view! { <></> }.into_any()
        }}
    </div>
}.into_any(),
```

**Note:** The select and input elements above are display-only for MVP. They render but don't yet update the variant when changed. Wiring on_change handlers to call `on_text_change` will be added in a follow-up task.

- [ ] **Step 2: Commit**

```bash
git add src/components/nodes/node.rs && git commit -m "feat(llm): render LLM node body with format/size/key fields"
```

---

## Task 7: Add CSS for node-variant-select

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Add select and field styling**

```css
.node-variant-select {
    width: 100%;
    padding: 4px 8px;
    font-size: 12px;
    border: 1px solid var(--border-color, #374151);
    border-radius: 4px;
    background: var(--bg-secondary, #1f2937);
    color: var(--text-color, #f3f4f6);
}

.node-variant-field label {
    font-size: 11px;
    color: var(--text-muted, #9ca3af);
    margin-bottom: 2px;
    display: block;
}

.node-variant-fields {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 4px;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/styles.css && git commit -m "styling: add CSS for node-variant-select"
```

---

## Task 8: Wire LLM async execution

**Files:**
- Modify: `src/components/app_layout.rs`

This is the most complex task. The current `handle_trigger` is fully synchronous — it loops through nodes and collects results synchronously. The LLM node requires an async HTTP call, so we must handle it via `spawn_local`.

**The approach:**
1. Split the execution loop into two phases: synchronous nodes (all existing types) run first, collecting results; LLM nodes are identified but deferred.
2. After all sync results are collected, spawn a `spawn_local` for each LLM node.
3. When the async result arrives, update `execution_state` and trigger a re-render.

**Important:** Downstream nodes that depend on LLM output will see stale/empty results in the first render cycle. This is a known limitation — fixing it (re-running downstream nodes after LLM completes) is a future enhancement.

- [ ] **Step 1: Add `LlmOutput` struct and `llm_complete` frontend wrapper near the top of `app_layout.rs`**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmOutput {
    pub text: String,
    pub tokens_used: u32,
    pub model: String,
    pub finish_reason: String,
    pub error: String,
}

/// Call Tauri backend for LLM completion
async fn call_llm_complete(
    format: String,
    model_size: String,
    api_key: String,
    custom_url: String,
    prompt: String,
    temperature: f64,
) -> Result<LlmOutput, String> {
    use crate::tauri_invoke;
    let opts = js_sys::Object::new();
    let config = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"config".into(), &config.into())
        .map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&config, &"format".into(), &format.into())
        .map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&config, &"model_size".into(), &model_size.into())
        .map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&config, &"api_key".into(), &api_key.into())
        .map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&config, &"custom_url".into(), &custom_url.into())
        .map_err(|e| e.to_string())?;
    let input = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"input".into(), &input.into())
        .map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&input, &"prompt".into(), &prompt.into())
        .map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&input, &"temperature".into(), &JsValue::from_f64(temperature))
        .map_err(|e| e.to_string())?;
    let js_value = tauri_invoke::invoke("llm_complete".into(), &opts).await?;
    serde_wasm_bindgen::from_value(js_value)
        .map_err(|e| format!("deserialization failed: {:?}", e))
}
```

- [ ] **Step 2: In `handle_trigger`, add the `llm` case BEFORE the catchall `_`**

Find the match on `node.node_type.as_str()` in `handle_trigger`. The LLM case must be added before the `_` catchall:

```rust
"llm" => {
    // Extract config from variant
    let config = if let crate::components::canvas::state::NodeVariant::LLM { config } = &node.variant {
        config.clone()
    } else {
        crate::components::canvas::state::LlmConfig {
            format: "openai".into(),
            model_size: "M".into(),
            api_key: String::new(),
            custom_url: String::new(),
        }
    };

    // Get prompt from upstream: look for the connection whose target is this node's "prompt" port.
    // upstream is HashMap<u32, String> keyed by source node_id -> result.
    // We need to find which upstream node is connected to our "prompt" input.
    let prompt_text = upstream
        .values()
        .next()
        .cloned()
        .unwrap_or_default();

    // Temperature: read from upstream values (currently all values are strings)
    // For MVP, temperature is always 1.0. A future task will read from the
    // temperature input port specifically.
    let temperature = 1.0;

    // Push a "waiting" task for this node
    let mut llm_task = crate::components::execution_engine::Task::new(
        exec_node_id, "llm", parent_id.clone(),
    );
    llm_task.status = crate::components::execution_engine::TaskStatus::Waiting;
    llm_task.waiting_on = Some(exec_node_id);
    llm_task.add_message(
        &format!("LLM call: {} / {} / prompt_len={}", config.format, config.model_size, prompt_text.len()),
        crate::components::execution_engine::TraceLevel::Info,
    );
    exec.tasks.push(llm_task);

    // Spawn the async call — it will update execution_state when done
    let exec_state_clone = execution_state;
    spawn_local(async move {
        let result = call_llm_complete(
            config.format.clone(),
            config.model_size.clone(),
            config.api_key.clone(),
            config.custom_url.clone(),
            prompt_text.clone(),
            temperature,
        ).await;

        match result {
            Ok(output) => {
                let status = if output.error.is_empty() {
                    crate::components::execution_engine::TaskStatus::Complete
                } else {
                    crate::components::execution_engine::TaskStatus::Error
                };
                let trace_msg = if output.error.is_empty() {
                    format!("LLM result: {} ({} tokens)", output.text, output.tokens_used)
                } else {
                    format!("LLM error: {}", output.error)
                };
                exec_state_clone.update(|exec| {
                    if let Some(task) = exec.tasks.iter_mut().find(|t| t.node_id == exec_node_id) {
                        task.status = status;
                        task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                        task.result = Some(output.text.clone());
                        task.add_message(
                            &trace_msg,
                            crate::components::execution_engine::TraceLevel::Info,
                        );
                    }
                });
            }
            Err(e) => {
                exec_state_clone.update(|exec| {
                    if let Some(task) = exec.tasks.iter_mut().find(|t| t.node_id == exec_node_id) {
                        task.status = crate::components::execution_engine::TaskStatus::Error;
                        task.finished_at = Some(crate::components::execution_engine::Timestamp::now());
                        task.add_message(
                            &format!("LLM call failed: {}", e),
                            crate::components::execution_engine::TraceLevel::Error,
                        );
                    }
                });
            }
        }
    });

    // Return placeholder — actual result is async
    String::new()
}
```

**Note on upstream lookup:** The current `upstream` HashMap uses source node IDs as keys. For the LLM node's `prompt` port, we take the first available upstream value. This works when there's a single upstream node. If multiple upstream nodes exist, the prompt will be ambiguous — this is a known limitation to address in a future task.

- [ ] **Step 3: Commit**

```bash
git add src/components/app_layout.rs && git commit -m "feat(llm): wire async LLM execution via spawn_local in handle_trigger"
```

---

## Task 9: End-to-end test

**Files:** None (manual test)

- [ ] **Step 1: Build Tauri backend**

```bash
cd src-tauri && cargo check 2>&1
```

Fix any compilation errors before proceeding.

- [ ] **Step 2: Run dev server**

```bash
trunk serve
```

- [ ] **Step 3: Verify in browser**
1. Drag "LLM" node from Agent palette onto canvas
2. Connect Trigger → LLM → Text Output
3. Wire a text/prompt source to LLM's `prompt` port
4. Click Run — LLM node should show in right panel trace as "Waiting" then update to "Complete" or "Error"
5. Check right panel trace for LLM call result
6. Verify error port shows message if API key is missing / env var not set

---

## File Summary

| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Modify — add reqwest |
| `src-tauri/src/llm.rs` | Create — LLM API client |
| `src-tauri/src/lib.rs` | Modify — register `llm_complete` command |
| `src/components/canvas/state.rs` | Modify — `NodeVariant::LLM`, ports, defaults |
| `src/components/left_panel.rs` | Modify — register `llm` in `NODE_TYPES` |
| `src/components/nodes/node.rs` | Modify — render `LLM` variant body |
| `src/components/app_layout.rs` | Modify — execute `llm` node type via spawn_local |
| `src/styles.css` | Modify — add select and field styles |

---

## Known Limitations (Out of Scope for This Plan)

1. **Temperature port value not read**: The `temperature` input port exists but is hardcoded to `1.0`. Reading port values into scalar parameters requires a lookup from `connections` + `node_results` keyed by port name, not just by node ID.
2. **Config field changes not persisted**: The format/size/key dropdowns and inputs render but don't call `on_text_change` to update the variant. They display defaults until a future task wires the change handlers.
3. **Downstream nodes don't re-run after LLM completes**: Nodes wired to LLM output ports see empty results on the first execution pass because the LLM result arrives asynchronously. A future task will add a re-trigger mechanism.
4. **`execution_engine.rs` is unused**: The `execute_node_sync` function in that file is not called by `handle_trigger` (execution is inline in `app_layout`). The `llm` case does not need to be added there.
