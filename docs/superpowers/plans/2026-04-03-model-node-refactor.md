# Model Node & Model Config Node Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename LLM node to Model node, extract config fields to a separate Model Config node, and replace the Size (S/M/L) field with a freeform `model_name` text field.

**Architecture:**
- `model_config` node: holds format/api_key/custom_url/model_name, outputs `config` as a JSON string via a single Text-type port
- `model` node: receives config via `config` input port (connected to Model Config's output), has prompt/temperature inputs and text/tokens_used/etc outputs, no inline config
- Backend `llm_complete` command accepts `model_name` string directly (no more S/M/L size tier mapping)

**Tech Stack:** Leptos 0.8, Tauri/Rust, reqwest

---

## Task 1: Update `src/components/canvas/state.rs`

- [ ] **Step 1: Add `model_config` to `NodeVariant` enum** (before the `LLM` variant or rename LLM to `model`)

Add a new `ModelConfig` variant alongside the existing `LLM` variant (we'll remove LLM in a later step):
```rust
ModelConfig {
    format: String,       // "openai" | "anthropic"
    model_name: String,  // e.g., "gpt-4o-mini", "claude-3-5-sonnet-latest"
    api_key: String,
    custom_url: String,
},
Model {
    // No inline config — config comes via port connection
},
```

- [ ] **Step 2: Rename `LlmConfig` struct comment** to `ModelConfig` (or keep LlmConfig — the backend still uses it; frontend state can have its own naming). Actually, let's keep `LlmConfig` in state.rs as `ModelConfig` alias or just rename to `ModelConfig` for consistency. Rename it to `ModelConfig`.

- [ ] **Step 3: Update `default_ports_for_type`** for `"model_config"` and rename `"llm"` to `"model"`:

```rust
"model_config" => vec![
    Port { name: "config".into(), port_type: PortType::Text, direction: PortDirection::Out },
],
"model" => vec![
    Port { name: "config".into(), port_type: PortType::Text, direction: PortDirection::In },
    Port { name: "prompt".into(), port_type: PortType::Text, direction: PortDirection::In },
    Port { name: "temperature".into(), port_type: PortType::Text, direction: PortDirection::In },
    Port { name: "text".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "tokens_used".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "model".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "finish_reason".into(), port_type: PortType::Text, direction: PortDirection::Out },
    Port { name: "error".into(), port_type: PortType::Text, direction: PortDirection::Out },
],
// Remove the old "llm" entry
```

- [ ] **Step 4: Update `default_variant_for_type`** — rename `"llm"` key to `"model"` and change the variant to `NodeVariant::Model`:

```rust
"model" => NodeVariant::Model,
"model_config" => NodeVariant::ModelConfig {
    format: "openai".into(),
    model_name: String::new(),
    api_key: String::new(),
    custom_url: String::new(),
},
// Remove the old "llm" entry
```

- [ ] **Step 5: Update `get_output_ports`** — rename `"llm"` to `"model"` in the match.

---

## Task 2: Update `src/components/nodes/node.rs`

- [ ] **Step 1: Update imports** — remove `LlmConfig` if renamed, update any references.

- [ ] **Step 2: Add `ModelConfig` variant rendering** in `render_variant_body`:

```rust
NodeVariant::ModelConfig { config } => view! {
    <div class="node-variant-fields">
        <div class="node-variant-field">
            <label>"Format"</label>
            <select class="node-variant-select">
                <option value="openai" selected={config.format == "openai"}>"OpenAI"</option>
                <option value="anthropic" selected={config.format == "anthropic"}>"Anthropic"</option>
            </select>
        </div>
        <div class="node-variant-field">
            <label>"Model Name"</label>
            <input
                type="text"
                class="node-variant-input"
                value={config.model_name.clone()}
                placeholder="gpt-4o-mini"
            />
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
        <div class="node-variant-field">
            <label>"API Endpoint"</label>
            <input
                type="text"
                class="node-variant-input"
                value={config.custom_url.clone()}
                placeholder="http://localhost:11434/v1"
            />
        </div>
    </div>
}.into_any(),
```

- [ ] **Step 3: Update `NodeVariant::LLM` → `NodeVariant::Model`** — replace with Model body (no config fields, just prompt/temperature display or just static text "Config via port"):

```rust
NodeVariant::Model => view! {
    <div class="node-variant-fields">
        <div class="node-variant-field">
            <span class="node-variant-static">"Config via port connection"</span>
        </div>
    </div>
}.into_any(),
// Remove the old LLM variant body rendering
```

- [ ] **Step 4: Update GraphNode port rendering** — the `config` input port should be visually distinguished or just shown as a normal Text input port (it is just a Text port).

---

## Task 3: Update `src/components/left_panel.rs`

- [ ] **Step 1: Rename `llm` entry to `model`** and update description:

```rust
NodeType {
    id: "model",
    name: "Model",
    category: "Agent",
    description: "Call an LLM API with config from Model Config node",
},
```

- [ ] **Step 2: Add `model_config` entry** (after `model` or in a new "Config" category — put it in Agent for now):

```rust
NodeType {
    id: "model_config",
    name: "Model Config",
    category: "Agent",
    description: "Holds API configuration for Model node",
},
```

---

## Task 4: Update `src/components/node_inspector.rs`

- [ ] **Step 1: Update `NodeVariant::LLM` → `NodeVariant::ModelConfig`** (rename the match arm, keep the field editors):

The inspector already shows format/model_size/api_key/custom_url. Update it to show model_name instead of model_size and rename the LLM label to Model Config.

```rust
NodeVariant::ModelConfig { config } => view! {
    <div class="property-groups">
        <div class="property-group">
            <label class="property-label">"Format"</label>
            <input type="text" class="property-input" value={config.format.clone()} />
        </div>
        <div class="property-group">
            <label class="property-label">"Model Name"</label>
            <input type="text" class="property-input" value={config.model_name.clone()} />
        </div>
        <div class="property-group">
            <label class="property-label">"API Key"</label>
            <input type="password" class="property-input" value={config.api_key.clone()} />
        </div>
        <div class="property-group">
            <label class="property-label">"Custom URL"</label>
            <input type="text" class="property-input" value={config.custom_url.clone()} />
        </div>
    </div>
}.into_any(),
// Add NodeVariant::Model case (no properties to show):
NodeVariant::Model => view! {
    <div class="property-group">
        <span class="property-label">"Config via connection"</span>
    </div>
}.into_any(),
// Remove the old NodeVariant::LLM arm
```

---

## Task 5: Update `src-tauri/src/llm.rs`

- [ ] **Step 1: Rename `LlmConfig.model_size` to `model_name`** (keep `LlmConfig` struct name since it's the Tauri command's input):

```rust
pub struct LlmConfig {
    pub format: String,       // "openai" | "anthropic"
    pub model_name: String,    // e.g., "gpt-4o-mini" — used directly
    pub api_key: String,
    pub custom_url: String,
}
```

- [ ] **Step 2: Remove `get_model_for_size` function** — model_name is passed directly, no mapping needed. Update callers to use `input.model_name.clone()` instead of `get_model_for_size(&config.format, &config.model_size)`.

- [ ] **Step 3: Update `llm_complete` function body** — instead of `let model = get_model_for_size(&config.format, &config.model_size);`, use `let model = config.model_name.clone();`

- [ ] **Step 4: Update resolve_api_key** — no changes needed, format is still passed.

---

## Task 6: Update `src/components/app_layout.rs`

- [ ] **Step 1: Update `LlmOutput` struct** — no changes needed (model is already a string field).

- [ ] **Step 2: Update `call_llm_complete` signature** — change `model_size: String` to `model_name: String`, update the js reflect calls:

```rust
async fn call_llm_complete(
    format: String,
    model_name: String,
    api_key: String,
    custom_url: String,
    prompt: String,
    temperature: f64,
) -> Result<LlmOutput, String> {
```

And update the Reflect.set calls accordingly.

- [ ] **Step 3: In the execution match, rename `"llm"` to `"model"`** and update the config extraction:

```rust
"model" => {
    // Extract config from the upstream "config" port connection
    // upstream is HashMap<u32, String> keyed by source node_id -> result
    // We need to find the value from a node that has a "config" port connection to this node
    let config_json = upstream
        .values()
        .next()
        .cloned()
        .unwrap_or_else(|| {
            // fallback: empty config JSON
            r#"{"format":"openai","model_name":"","api_key":"","custom_url":""}"#.to_string()
        });

    let config: llm::LlmConfig = serde_json::from_str(&config_json).unwrap_or_else(|_| llm::LlmConfig {
        format: "openai".into(),
        model_name: String::new(),
        api_key: String::new(),
        custom_url: String::new(),
    });

    // prompt and temperature from other upstream connections
    let prompt_text = upstream.values().next().cloned().unwrap_or_default();
    let temperature = 1.0;

    // rest of the LLM call logic...
}
```

- [ ] **Step 4: Also handle `"model_config"` in the execution match** — it just passes through (no outputs, mark complete immediately):

```rust
"model_config" => {
    task.status = crate::components::execution_engine::TaskStatus::Complete;
    task.add_message("Model Config node", crate::components::execution_engine::TraceLevel::Info);
}
```

- [ ] **Step 5: Update the spawn_local call** to pass `model_name` instead of `model_size`:

```rust
let result = call_llm_complete(
    config.format.clone(),
    config.model_name.clone(),  // was config.model_size
    config.api_key.clone(),
    config.custom_url.clone(),
    prompt_text.clone(),
    temperature,
).await;
```

- [ ] **Step 6: Update the trace message** that logs the call to use `model_name`:

```rust
&format!("Model call: {} / {} / prompt_len={}", config.format, config.model_name, prompt_text.len()),
```

---

## Task 7: Update spec document

- [ ] Update `docs/superpowers/specs/2026-04-03-llm-node-design.md` to reflect the new Model/ModelConfig design, or create a new spec `2026-04-03-model-node-design.md`.

---

## Files Summary

| File | Change |
|------|--------|
| `src/components/canvas/state.rs` | Add `ModelConfig`/`Model` variants, rename ports, update `default_variant_for_type` |
| `src/components/nodes/node.rs` | Render both new variants, remove old LLM rendering |
| `src/components/left_panel.rs` | Add `model_config` node type, rename `llm`→`model` |
| `src/components/node_inspector.rs` | Handle `ModelConfig`/`Model` variants |
| `src-tauri/src/llm.rs` | Rename `model_size`→`model_name`, remove `get_model_for_size` |
| `src/components/app_layout.rs` | Handle `model`/`model_config` in execution, update `call_llm_complete` |
| `docs/superpowers/specs/2026-04-03-llm-node-design.md` | Update to reflect new design |

## Notes

- The `config` port uses `PortType::Text` and carries a JSON string between Model Config and Model nodes
- When a `model` node executes, it reads the config JSON from its upstream connections (the first value is used as the config bundle)
- The Model Config node itself doesn't execute an LLM call — it just holds config. Its output is consumed by Model nodes.
- The old `llm` node type ID is fully replaced by `model` — no backwards compatibility needed
