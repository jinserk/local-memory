# API Spec: Rust Local Memory Plugin (OpenSpec)

## 1. Protocol
- **Transport**: JSON-RPC 2.0 over `stdio`.
- **Compliance**: Model Context Protocol (MCP) v1.0.

## 2. Resources
- `memory://all`: Returns all stored memory IDs and snippets.

## 3. Tools (Methods)

### 3.1 `memory/insert`
Inserts a new text into the long-term memory.
- **Input**:
    - `text` (string): The content to remember.
    - `metadata` (object, optional): Arbitrary key-value pairs (source, tags, etc.).
- **Operation**:
    1. Generate 768d embedding via `Candle`.
    2. Quantize to 768-bit binary vector.
    3. Persist metadata and full vector in `Fjall`.
    4. Update in-memory bit-index.

### 3.2 `memory/search`
Searches for relevant memories using the multi-stage funnel.
- **Input**:
    - `query` (string): Search query.
    - `limit` (number, default: 5): Number of results to return.
    - `stages` (object, optional): Override for bit-wise search stages.
- **Output**:
    - `results` (array): Array of objects containing `{text, score, metadata}`.

### 3.3 `memory/delete`
Deletes a memory by ID.
- **Input**:
    - `id` (string): The UUID of the memory.

## 4. Configuration (settings.json)
```json
{
  "search_stages": [
    { "type": "bit_wise", "bits": 768, "candidates": 1000 },
    { "type": "matryoshka", "dims": 256, "candidates": 100 },
    { "type": "re_rank", "dims": 768 }
  ],
  "model_path": "./models/nomic-embed-text-v1.5",
  "storage_path": "./db"
}
```
