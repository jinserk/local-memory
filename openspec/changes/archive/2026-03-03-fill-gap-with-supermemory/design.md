## Context

Local Memory is currently a high-performance retrieval engine using a 3-stage funnel (BQ, Matryoshka, Full Vector) and a Knowledge Graph. However, it lacks time-awareness (historical facts remain indistinguishable from current ones) and proactive ingestion (users must manually insert facts). This design outlines the transition to a "Living Knowledge" system inspired by Supermemory.

## Goals / Non-Goals

**Goals:**
- Implement temporal versioning in SQLite to track fact lifecycles.
- Implement proactive background observers for Git and Shell history.
- Move from character-based chunking to LLM-based semantic boundary detection.
- Add importance-based decay to retrieval ranking.

**Non-Goals:**
- Real-time multi-user synchronization.
- Building a custom VLM model (utilizing existing architectures like NuExtract-2.0 instead).

## Decisions

### 1. SQLite Schema Evolution
**Decision:** Add `version` (INT), `is_latest` (BOOLEAN), and `namespace` (TEXT) to `documents` and `entities` tables.
**Rationale:** Standard relational columns are the most efficient way to filter historical context during Stage 1 of the retrieval funnel.
**Alternatives:** Storing history in a separate `history` table (complex JOINs) or using Git-style blobs (high overhead).

### 2. Proactive Sidecar Threads
**Decision:** Spawn background `tokio` tasks in the MCP server to monitor `.git/` and local shell history files.
**Rationale:** Makes memory "invisible" without requiring project-specific configuration files.
**Alternatives:** External cron jobs or git hooks (requires manual user setup).

### 3. Semantic Boundary Detection
**Decision:** Use the `NuExtract` model during ingestion to detect "concept boundaries" and generate per-chunk parent summaries.
**Rationale:** Increases retrieval precision by ensuring vectors represent coherent ideas rather than arbitrary text slices.
**Alternatives:** Fixed character overlap (current state, often loses context).

### 4. Hybrid Decay Scoring
**Decision:** Multiply cosine similarity by a `time_decay` factor: `score = similarity * exp(-lambda * delta_t)`.
**Rationale:** Natural language often contains transient facts (e.g., "current version is 1.0") that should deprioritize over time unless explicitly refreshed.

## Risks / Trade-offs

- **[Performance]** LLM-based chunking increases ingestion time. → **Mitigation**: Run summarization in background threads; provide "Fast Heuristic" vs "Deep Semantic" modes.
- **[Database Size]** Versioning will increase SQLite file size. → **Mitigation**: Implement a `vacuum` command and optional historical pruning.
- **[Privacy]** Background monitoring of shell history could ingest sensitive data. → **Mitigation**: Implement an explicit `exclude_patterns` list in `config.json`.
