# Configuration Guide

Local Memory is configured using a `.local-memory/config.json` file located in the project root, or via the `LOCAL_MEMORY_CONFIG` environment variable.

## Configuration Structure

```json
{
  "storage_path": ".local-memory/storage",
  "model_path": ".local-memory/models",
  "embedding": {
    "name": "nomic-ai/nomic-embed-text-v1.5",
    "provider": "huggingface",
    "auto_download": true,
    "dimension": 768
  },
  "llm_extractor": {
    "provider": "huggingface",
    "name": "phi-3-mini-4k-instruct"
  }
}
```

## 1. Storage & Paths

| Option | Default | Description |
|--------|---------|-------------|
| `storage_path` | `.local-memory/storage` | Path where the SQLite database and Graph data are stored. |
| `model_path` | `.local-memory/models` | Path where local embedding model files (BERT/Nomic) are cached. |

## 2. Embedding Model (`embedding`)

This model runs **locally** on your CPU using the Rust `candle` crate. It is responsible for turning text into vectors for semantic search.

| Option | Default | Description |
|--------|---------|-------------|
| `name` | `nomic-ai/nomic-embed-text-v1.5` | The HuggingFace model ID. |
| `provider` | `huggingface` | Where to fetch the model from (`huggingface` or `local`). |
| `auto_download` | `true` | If true, missing model files will be downloaded on startup. |
| `dimension` | `768` | Dimension of the vectors (must match the model architecture). |

## 3. LLM Extractor (`llm_extractor`)

This model is responsible for **GraphRAG Reasoning**: extracting entities and relationships from text.

### Local (Recommended for Privacy)
Uses **Candle** or **Ollama**.
```json
"llm_extractor": {
  "provider": "huggingface",
  "name": "phi-3-mini-4k-instruct"
}
```

### Remote
Requires an API key.
```json
"llm_extractor": {
  "provider": "openai",
  "name": "gpt-4o",
  "api_key": "sk-..."
}
```

## Environment Variables

- `LOCAL_MEMORY_CONFIG`: Path to the config JSON file.
- `OPENAI_API_KEY`: API key for OpenAI extraction (overrides config).
- `ANTHROPIC_API_KEY`: API key for Anthropic extraction.
- `GEMINI_API_KEY`: API key for Google Gemini extraction.
