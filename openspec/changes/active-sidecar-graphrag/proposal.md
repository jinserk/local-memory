# Proposal: Active Sidecar Architecture for GraphRAG

## Why
While Local Memory excels at real-time local retrieval, it lacks the holistic "forest-view" capabilities of Microsoft GraphRAG (Global Search). To bridge this gap without sacrificing real-time performance, we need a reactive system that enriches knowledge in the background.

## Goals
- Transition from a passive storage model to an **Event-Driven Knowledge System**.
- Implement **Incremental Community Detection** to group related entities.
- Automate **Hierarchical Summarization** to enable "Thematic Global Search."
- Maintain **Zero-Latency** for the user by moving heavy graph computations to asynchronous sidecars.

## Success Criteria
- [ ] A search for a broad theme (e.g., "What are the trends in Boston?") returns synthesized summaries.
- [ ] New entities are automatically assigned to "communities" shortly after ingestion.
- [ ] System remains stable and fast even as the background graph grows.
