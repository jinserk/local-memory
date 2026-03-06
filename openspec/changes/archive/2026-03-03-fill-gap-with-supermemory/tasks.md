## 1. Storage & Schema Migration

- [x] 1.1 Add `version`, `is_latest`, and `namespace` columns to `documents` and `entities` tables in `sqlite.rs`
- [x] 1.2 Implement migration logic to populate existing records with default values
- [x] 1.3 Update `insert_document` and `insert_entity` to handle versioning and the `is_latest` flag
- [x] 1.4 [NEW] Implement a "Registry" tracker to manage multiple SQLite project databases (Global Index)

## 2. Temporal Evolution & Ingestion

- [x] 2.1 Update `IngestionPipeline` to detect if new text updates existing entities
- [x] 2.2 Add support for `UPDATES` and `SUPERSEDES` relationship types in extraction logic
- [x] 2.3 Implement logic to flip `is_latest` to false for old versions during new ingestion
- [x] 2.4 [NEW] Implement LLM-based Conflict Detection during ingestion to flag contradictory facts

## 3. Semantic Chunking

- [x] 3.1 Integrate `NuExtract` boundary detection into the `run` method of `IngestionPipeline`
- [x] 3.2 Implement "Parent Summary" generation for each semantic chunk
- [x] 3.3 Update storage to associate chunks with their parent summaries

## 4. Proactive Automation (Sidecars)

- [x] 4.1 Implement `GitObserver` thread in `src/main.rs` to watch for new commits
- [x] 4.2 Implement `ShellObserver` thread to monitor command history (with opt-in)
- [x] 4.3 Add MCP Resource endpoints (`resources/list`, `resources/read`) to the server

## 5. Memory Registry & Retrieval

- [x] 5.1 Update `SearchFunnel` to support `namespace` filtering
- [x] 5.2 Implement importance decay formula in the ranking stage of the funnel
- [x] 5.3 Update `lmcli` to support namespace selection and version inspection

## 6. Multimodal Memory

- [x] 6.1 Extend `CandleProvider` to support VLM architecture (e.g., Qwen2-VL or similar)
- [x] 6.2 Add OCR preprocessing step to the ingestion pipeline
- [x] 6.3 Add image support to `memorize` tool
