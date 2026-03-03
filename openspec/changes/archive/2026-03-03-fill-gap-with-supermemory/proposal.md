## Why

Local Memory currently operates as a high-performance additive retrieval engine. To evolve into a truly intelligent "Second Brain" similar to Supermemory, the system must transition from simply storing facts to managing a "Living Knowledge Graph." This involves understanding knowledge evolution (updates vs. history), respecting semantic boundaries (ideas vs. characters), and becoming proactive rather than reactive.

## Goals
- Transform the retrieval engine into a version-aware knowledge system.
- Implement proactive context collection (Git/Shell) to make memory "invisible."
- Move from fixed-size chunking to semantically coherent concept boundaries.
- Introduce priority-based ranking (decay) to surface relevant, recent decisions.

## Non-goals
- Building a full cloud-sync service (remaining local-first).
- Implementing a full OCR engine from scratch (utilizing local VLM models instead).

## What Changes

- **Temporal SQLite Schema**: Introduction of `version`, `is_latest`, and `namespace` fields to documents and entities.
- **Semantic Ingestion**: Refactoring the ingestion pipeline to use LLM-based boundary detection.
- **Proactive Observers**: Background threads for monitoring Git commits and providing proactive MCP resources.
- **Priority Funnel**: Updating the 3-stage funnel to incorporate `decay_factor` and `importance` scores.
- **Vision-Ready Providers**: Extending `CandleProvider` to support Vision-Language architectures.

## Capabilities

### New Capabilities
- `temporal-evolution`: Logic for tracking fact lifecycles (UPDATES, SUPERSEDES) and versioning in SQLite.
- `semantic-chunking`: Context-aware text splitting based on topic shifts rather than character counts.
- `memory-registry`: Support for project namespaces and priority-based ranking (decay).
- `proactive-automation`: Background collectors for Git history and proactive context injection via MCP Resources.
- `multimodal-extraction`: Support for Vision-Language tasks to ingest screenshots and diagrams.

### Modified Capabilities
- `core-retrieval`: The 3-stage retrieval funnel will be modified to respect `is_latest` flags and `priority` scores.

## Impact

- **Storage**: SQLite schema migration required for existing databases.
- **Performance**: Additional LLM calls during ingestion for semantic boundary detection (offset by better retrieval precision).
- **Architecture**: Addition of background observer threads to the MCP server.
- **Protocols**: Expansion of MCP tools to include Resource endpoints.
