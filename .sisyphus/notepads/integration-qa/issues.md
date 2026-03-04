# Integration QA Issues

- **Stage 3 Funnel Missing**: `src/engine/funnel.rs` only implements Stage 1 and Stage 2. Stage 3 (Full Precision) is described in `docs/ARCHITECTURE.md` but not called in `search_with_namespace`.
- **Community Summaries "string"**: Many community summaries contain the placeholder "string" for title and summary. This is likely due to the LLM (NuExtract) interpreting the prompt as a schema definition rather than a request for content.
- **Graph Neighborhood Unidirectional**: `graph_get_neighborhood` only returns outgoing relationships (where the entity is the source). It should ideally return both incoming and outgoing edges.
- **Shell Observer History Access**: The shell observer may not have access to the actual user history in some environments, leading to no shell commands being ingested.
