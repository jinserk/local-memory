# Configuration Guide

Local Memory is configured using a `.local-memory/config.json` file.

## Configuration Structure

### Asymmetric Ollama Setup (Recommended)
This setup uses one model for high-performance embeddings and another for high-precision reasoning.

```json
{
  "storage_path": ".local-memory/storage",
  "model_path": ".local-memory/models",
  "embedding": {
    "name": "nomic-embed-text-v2-moe",
    "provider": "ollama",
    "auto_download": true,
    "dimension": 768,
    "base_url": "http://localhost:11434"
  },
  "llm_extractor": {
    "provider": "ollama",
    "name": "frob/nuextract-2.0:8b-q8_0",
    "auto_download": true,
    "base_url": "http://localhost:11434"
  }
}
```

### Local-Only Setup (Candle)
This setup runs entirely within the Rust process without external servers.

```json
{
  "embedding": {
    "name": "nomic-ai/nomic-embed-text-v1.5",
    "provider": "huggingface"
  },
  "llm_extractor": {
    "provider": "huggingface",
    "name": "numind/NuExtract-1.5"
  }
}
```

## Configuration Reference

### 1. Storage & Paths

| Option | Default | Description |
|--------|---------|-------------|
| `storage_path` | `.local-memory/storage` | SQLite database and Graph data location. |
| `model_path` | `.local-memory/models` | Local HuggingFace model cache. |

### 2. Embedding Model (`embedding`)

| Option | Description |
|--------|-------------|
| `name` | The model identifier (e.g., `nomic-embed-text-v2-moe`). |
| `provider` | `ollama` (remote local), `huggingface` (native), or `openai`. |
| `dimension` | Vector dimension. **Note**: Nomic models are 768. |
| `base_url` | API endpoint for Ollama or OpenAI compatible servers. |

### 3. LLM Extractor (`llm_extractor`)

| Option | Description |
|--------|-------------|
| `provider` | `ollama`, `huggingface`, or `openai`. |
| `name` | Model identifier (e.g., `frob/nuextract-2.0:8b-q8_0`). |
| `auto_download`| If true, `lmcli init` will pull/download missing models. |

---

## Environment Variables

Local Memory also respects standard environment variables which override the config file:

- `LOCAL_MEMORY_CONFIG`: Custom path to `config.json`.
- `OPENAI_API_KEY`: Required if provider is `openai`.
- `ANTHROPIC_API_KEY`: Required for Anthropic reasoning.
