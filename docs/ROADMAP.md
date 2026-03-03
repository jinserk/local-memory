# Roadmap: Towards Living Knowledge

## 📌 Summary of Pillars (Simplified)
1.  **Memory Registry**: Organize memories into "folders" (work, personal) and let old facts "fade away" naturally.
2.  **Proactive Sidecars**: Automatically watch your Git commits and Shell commands so you never have to manually "save" again.
3.  **Temporal Evolution**: Track versions of facts so the system knows when old info (like yesterday's weather) is replaced by new info.
4.  **Semantic Intelligence**: Stop splitting text by character count; start splitting by "ideas" and "topics."
5.  **Multimodal Memory**: Add "eyes" to your memory to store and understand screenshots, PDFs, and diagrams.

---

## Phase 1: Memory Registry & Priority Scoring
*Target: Organization and Ranking*
*   [ ] **Namespace Isolation**: Support for "collections" or "folders" (e.g., `work`, `personal`, `archived`) to scope searches.
*   [ ] **Importance Decay**: Implement a `decay_factor` where older, unused facts "sink" in search results while frequently accessed decisions "float" to the top.
*   [ ] **Global Registry**: A centralized index that can manage multiple SQLite database files across different projects.

## Phase 2: Proactive Sidecar Tools
*Target: Invisible Ingestion*
*   [ ] **Git Observer**: A background process that automatically ingests and summarizes git commits.
*   [ ] **Shell Integrator**: Automatically record significant CLI commands and their outcomes.
*   [ ] **MCP Resources**: Implement proactive "Current Context" resources that agents can read without a tool call.

## Phase 3: Temporal Graph Evolution
*Target: Knowledge Accuracy*
*   [ ] **Versioned Documents**: Add `version` and `is_latest` fields to the SQLite document table.
*   [ ] **Relationship Lifecycles**: Implement edge types like `UPDATES`, `EXTENDS`, and `SUPERSEDES`.
*   [ ] **Conflict Detection**: Use the LLM to detect if new information contradicts existing memory.

## Phase 4: Semantic Intelligence
*Target: Retrieval Precision*
*   [ ] **Semantic Chunking**: Split documents based on topic shifts rather than character counts.
*   [ ] **Contextual Summaries**: Store a "parent summary" with every chunk to improve vector search precision.
*   [ ] **Asymmetric Tuning**: Optimize the 3-stage funnel specifically for the `NuExtract` reasoning outputs.

## Phase 5: Multimodal Memory
*Target: Visual Context*
*   [ ] **Vision Extraction**: Utilize `NuExtract-2.0-4B`'s Vision-Language capabilities to "remember" screenshots, diagrams, and PDFs.
*   [ ] **OCR Pipeline**: Integrated local OCR for scanned technical documentation.
