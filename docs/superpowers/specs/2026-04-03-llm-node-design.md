# LLM Node Design

## Status
Approved — 2026-04-03

---

## 1. Overview

The LLM node is an Agent-category node that calls a language model API (OpenAI, Anthropic, or OpenAI-compatible endpoint) with a prompt and returns the generated text along with metadata. Execution happens on the Rust/Tauri backend to keep API keys secure.

---

## 2. Node Identity

| Field | Value |
|-------|-------|
| Type ID | `llm` |
| Category | Agent |
| Label | "LLM" |

---

## 3. Config Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `openai` / `anthropic` / `openai-compatible` | `openai` | API request/response format |
| `model_size` | `S` / `M` / `L` | `M` | Size tier; selects model from format defaults |
| `api_key` | string | `""` | Per-node key; falls back to global env var if empty |
| `custom_url` | string | `""` | Base URL for `openai-compatible` (e.g. `http://localhost:11434/v1`) |

### Model Defaults

| Size | OpenAI | Anthropic | OpenAI-compatible |
|------|--------|-----------|-------------------|
| S | `gpt-4o-mini` | `claude-3-5-haiku-20241022` | `gpt-4o-mini` |
| M | `gpt-4o` | `claude-3-5-sonnet-latest` | `gpt-4o` |
| L | `gpt-4-turbo` | `claude-3-5-opus-latest` | `gpt-4-turbo` |

---

## 4. Ports

### Input Ports

| Port | Type | Description |
|------|------|-------------|
| `prompt` | Text | Final prompt assembled by a prompt concat node |
| `temperature` | Number | Sampling temperature (float, e.g. `0.7`, `1.0`) |

### Output Ports

| Port | Type | Description |
|------|------|-------------|
| `text` | Text | Response content from the model |
| `tokens_used` | Number | Total tokens consumed (input + output) |
| `model` | Text | Actual model name that fulfilled the request |
| `finish_reason` | Text | Why generation stopped: `stop`, `length`, `content_filter`, `error` |
| `error` | Text | Error message string if the call failed; empty on success |

---

## 5. API Request/Response Mapping

### OpenAI (`POST /v1/chat/completions`)

**Request body:**
```json
{
  "model": "<resolved_model>",
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
  "model": "<resolved_model>",
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

Same request/response structure as OpenAI, using `custom_url` as the base URL.

---

## 6. API Key Resolution

1. If `api_key` config field is non-empty, use it.
2. Otherwise, check for a global env var named after the format:
   - `openai` → `OPENAI_API_KEY`
   - `anthropic` → `ANTHROPIC_API_KEY`
   - `openai-compatible` → `OPENAI_API_KEY`
3. If neither exists, return an error on the `error` port.

---

## 7. Backend Execution

- A new Tauri command (e.g. `llm_complete`) in the Rust backend receives the node config and inputs.
- Uses a Rust HTTP client (e.g. `reqwest`) to make the API call.
- The call is async; the node UI shows "running" status while awaiting response.
- On success: results are written to the node's output port values in the execution state.
- On failure: error string is written to the `error` port; other output ports are cleared.

---

## 8. Files to Modify

| File | Change |
|------|--------|
| `src/components/canvas/state.rs` | Add `Llm` variant to `NodeVariant`; add `LlmConfig` struct |
| `src/components/canvas/state.rs` | Add `default_ports_for_type("llm")` entries |
| `src/components/nodes/node.rs` | Add LLM node body rendering (config fields as UI) |
| `src/components/left_panel.rs` | Add `llm` to `NODE_TYPES` const |
| `src-tauri/src/main.rs` or new `src-tauri/src/llm.rs` | Add `llm_complete` Tauri command |
| `src-tauri/Cargo.toml` | Add `reqwest` dependency |
| `src/components/execution_engine.rs` | Handle `llm` node type in `execute_node_sync` |

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
