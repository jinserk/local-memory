## 1. Event Infrastructure & Schema

- [x] 1.1 Define `KnowledgeEvent` enum and initialize `tokio::sync::broadcast` channel in `main.rs`
- [x] 1.2 Update `IngestionPipeline` to emit events on successful document/entity/relationship insertion
- [x] 1.3 Add `communities` table to `sqlite.rs` and update entities to include a `community_id` column
- [x] 1.4 Implement `Storage` methods to fetch all relationships for graph bootstrapping

## 2. Graph Sidecar (Clustering)

- [x] 2.1 Add `petgraph` dependency to `Cargo.toml`
- [x] 2.2 Implement `GraphObserver` thread that maintains an in-memory graph of the database
- [x] 2.3 Implement **Label Propagation** (incremental clustering) within the `GraphObserver`
- [x] 2.4 Update the `entities` table in SQLite with assigned `community_id` in the background

## 3. Summarizer Sidecar (Synthesis)

- [x] 3.1 Implement `CommunityService` that watches for "dirty" or new communities
- [x] 3.2 Create LLM prompts for high-level thematic summarization of entity clusters
- [x] 3.3 Implement persistence logic to store generated summaries in the `communities` table

## 4. Global Retrieval

- [x] 4.1 Implement `memory_global_search` MCP tool that queries the `communities` table
- [x] 4.2 Update `SearchFunnel` to support "Thematic Fallback" (Local Search -> Global Search if scores are low)
- [x] 4.3 Update `lmcli` to support community listing and inspection
