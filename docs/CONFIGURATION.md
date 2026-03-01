# Configuration Guide

Local Memory is configured using a `.local-memory/config.json` file located in the project root, or via the `LOCAL_MEMORY_CONFIG` environment variable.

## Configuration Structure

```json
{
  "storage_path": ".local-memory/storage",
  "model_path": ".local-memory/models",
  "embedding_model": {
    "name": "nomic-ai/nomic-embed-text-v1.5",
    "provider": "huggingface",
    "auto_download": true
  },
  "llm_extractor": {
    "provider": "ollama",
    "model": "llama3.2:3b",
    "base_url": "http://localhost:11434"
  }
}
```

## 1. Storage & Paths

| Option | Default | Description |
|--------|---------|-------------|
| `storage_path` | `.local-memory/storage` | Path where the SQLite database and Graph data are stored. |
| `model_path` | `.local-memory/models` | Path where local embedding model files (BERT/Nomic) are cached. |

**Tip**: You can point `storage_path` to an absolute path like `/home/user/.config/local-memory` to share memory across multiple projects.

## 2. Embedding Model (`embedding_model`)

This model runs **locally** on your CPU using the Rust `candle` crate. It is responsible for turning text into vectors for semantic search.

| Option | Default | Description |
|--------|---------|-------------|
| `name` | `nomic-ai/nomic-embed-text-v1.5` | The HuggingFace model ID. |
| `provider` | `huggingface` | Where to fetch the model from (`huggingface` or `local`). |
| `auto_download` | `true` | If true, missing model files will be downloaded on startup. |

## 3. LLM Extractor (`llm_extractor`)

This model is responsible for **GraphRAG Reasoning**: extracting entities and relationships from text. It can be a local server (Ollama) or a remote API (OpenAI/Anthropic).

### Local (Recommended for Privacy)
Use **Ollama** to run models locally.
```json
"llm_extractor": {
  "provider": "ollama",
  "model": "llama3.2:3b",
  "base_url": "http://localhost:11434"
}
```

### Remote (Fastest)
Requires an API key (can be set in JSON or as an environment variable).
```json
"llm_extractor": {
  "provider": "openai",
  "model": "gpt-4o",
  "api_key": "sk-..."
}
```

## Environment Variables

Configuration can be overridden or provided via environment variables:

- `LOCAL_MEMORY_CONFIG`: Path to the config JSON file.
- `OPENAI_API_KEY`: API key for OpenAI extraction (overrides config).
- `ANTHROPIC_API_KEY`: API key for Anthropic extraction.
- `GEMINI_API_KEY`: API key for Google Gemini extraction.
