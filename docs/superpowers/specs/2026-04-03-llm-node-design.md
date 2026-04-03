# LLM Node Design (Model / ModelConfig Architecture)

## Status
Approved — 2026-04-03

---

## 1. Overview

The LLM node functionality is split into two node types:

- **`model_config`** — an Agent-category node that holds API configuration (format, model name, API key, custom URL) and outputs a JSON config blob via a port.
- **`model`** — an Agent-category node that receives the config blob on a port, combines it with runtime inputs (prompt, temperature), calls a language model API, and returns the generated text along with metadata.

This separation lets multiple `model` nodes share a single `model_config` without duplicating credentials. Execution happens on the Rust/Tauri backend to keep API keys secure.

---

## 2. Node Identity

### model_config

| Field | Value |
|-------|-------|
| Type ID | `model_config` |
| Category | Agent |
| Label | "Model Config" |

### model

| Field | Value |
|-------|-------|
| Type ID | `model` |
| Category | Agent |
| Label | "Model" |

---

## 3. model_config Node

### 3.1 Config Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `openai` / `anthropic` / `openai-compatible` | `openai` | API request/response format |
| `model_name` | string | `""` | Model name passed directly to the API (e.g. `gpt-4o-mini`, `claude-3-5-sonnet-latest`) |
| `api_key` | string | `""` | Per-node key; falls back to global env var if empty |
| `custom_url` | string | `""` | Base URL for `openai-compatible` (e.g. `http://localhost:11434/v1`) |

### 3.2 Ports

#### Output Ports

| Port | Type | Description |
|------|------|-------------|
| `config` | Text | JSON blob containing `{ format, model_name, api_key, custom_url }`; connect to `model` node's `config` input |

---

## 4. model Node

### 4.1 Config Fields

None — all configuration comes through the `config` input port.

### 4.2 Ports

#### Input Ports

| Port | Type | Description |
|------|------|-------------|
| `config` | Text | JSON blob from `model_config` node (format, model_name, api_key, custom_url) |
| `prompt` | Text | Final prompt assembled by upstream nodes |
| `temperature` | Number | Sampling temperature (float, e.g. `0.7`, `1.0`) |

#### Output Ports

| Port | Type | Description |
|------|------|-------------|
| `text` | Text | Response content from the model |
| `tokens_used` | Number | Total tokens consumed (input + output) |
| `model` | Text | Actual model name that fulfilled the request |
| `finish_reason` | Text | Why generation stopped: `stop`, `length`, `content_filter`, `error` |
| `error` | Text | Error message string if the call failed; empty on success |

---

## 5. API Request/Response Mapping

The `model` node resolves the format from the incoming `config` JSON and routes accordingly.

### OpenAI (`POST /v1/chat/completions`)

**Request body:**
```json
{
  "model": "<model_name from config>",
  "messages": [{"role": "user", "content": "<prompt>"}],
  "temperature": <temperature>
}
```

**Response extraction:**
- `text` ← `choices[0].message.content`
- `tokens_used` ← `usage.total_tokens`
- `model` ← `model`
- `finish_reason` ← `choices[0].finish_reason`

### Anthropic (`POST /v1/messages`)

**Request body:**
```json
{
  "model": "<model_name from config>",
  "messages": [{"role": "user", "content": "<prompt>"}],
  "max_tokens": 1024,
  "temperature": <temperature>
}
```

**Response extraction:**
- `text` ← `content[0].text`
- `tokens_used` ← `usage.input_tokens + usage.output_tokens`
- `model` ← `model`
- `finish_reason` ← `stop_reason`

### OpenAI-Compatible

Same request/response structure as OpenAI, using `custom_url` from config as the base URL.

---

## 6. API Key Resolution

1. If `api_key` in the config JSON is non-empty, use it.
2. Otherwise, check for a global env var named after the format:
   - `openai` → `OPENAI_API_KEY`
   - `anthropic` → `ANTHROPIC_API_KEY`
   - `openai-compatible` → `OPENAI_API_KEY`
3. If neither exists, return an error on the `error` port.

---

## 7. Backend Execution

- A new Tauri command (e.g. `model_complete`) in the Rust backend receives the config JSON and runtime inputs.
- Uses a Rust HTTP client (e.g. `reqwest`) to make the API call.
- The call is async; the node UI shows "running" status while awaiting response.
- On success: results are written to the node's output port values in the execution state.
- On failure: error string is written to the `error` port; other output ports are cleared.

---

## 8. Files to Modify

| File | Change |
|------|--------|
| `src/components/canvas/state.rs` | Add `ModelConfig` and `Model` variants to `NodeVariant`; add `ModelConfigData` struct |
| `src/components/canvas/state.rs` | Add `default_ports_for_type("model_config")` and `default_ports_for_type("model")` entries |
| `src/components/nodes/node.rs` | Add model_config and model node body rendering |
| `src/components/left_panel.rs` | Add `model_config` and `model` to `NODE_TYPES` const |
| `src-tauri/src/main.rs` or new `src-tauri/src/llm.rs` | Add `model_complete` Tauri command |
| `src-tauri/Cargo.toml` | Add `reqwest` dependency |
| `src/components/execution_engine.rs` | Handle `ModelConfig` and `Model` node types in `execute_node_sync` |

---

## 9. Error Handling

| Condition | Behavior |
|-----------|----------|
| Missing API key | Write `"missing API key"` to `error` port |
| HTTP error (4xx/5xx) | Write HTTP status text to `error` port |
| JSON parse failure | Write `"failed to parse response"` to `error` port |
| Network timeout | Write `"request timed out"` to `error` port |

---

## 10. Out of Scope (Future)

- Streaming responses (text chunks as they're generated)
- Vision/multimodal input (images, audio)
- Token budget / cost tracking
- Retry logic
- System prompt as separate port