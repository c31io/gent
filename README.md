# Gent

Chat is hyped.

The authentic harness interface is a node editor.

## Configuration

Gent loads a TOML config file at startup for default LLM settings. Create it at the platform path below and leave fields empty in the **Model Config** node to fall back to these values.

| Platform | Path |
|----------|------|
| Linux / macOS | `~/.config/gent/config.toml` |
| Windows | `%APPDATA%\gent\config.toml` |

### Example

```toml
default_format = "openai"

[providers.openai]
model = "gpt-4o-mini"
api_key = "sk-..."
endpoint = "https://api.openai.com/v1"

[providers.anthropic]
model = "claude-3-5-sonnet-latest"
api_key = "sk-ant-..."
endpoint = "https://api.anthropic.com/v1"
```

- `default_format` — used when the node leaves the format field empty.
- `providers.<format>.model` — default model name for that provider.
- `providers.<format>.api_key` — default API key.
- `providers.<format>.endpoint` — default base URL (maps to the node's *Custom URL* field).
