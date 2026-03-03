# Gap Analysis: Local Memory vs. Microsoft GraphRAG

## Executive Summary
**Local Memory** is designed for real-time, low-latency, and evolving knowledge (the "Living Knowledge" paradigm), excelling at **Temporal Evolution** and **Local Search**. 

**Microsoft GraphRAG** focuses on holistic understanding of static datasets, leveraging **Community Detection (Leiden)** and **Hierarchical Summarization** to enable **Global Search** (answering broad, thematic questions).

Currently, `local-memory` is a superior "Second Brain" for specific facts and updates but lacks the "Forest View" capabilities of Microsoft's architecture.

## Feature Comparison Matrix

| Feature | Microsoft GraphRAG | Local Memory (Current) | Gap / Advantage |
| :--- | :--- | :--- | :--- |
| **Indexing Strategy** | Batch processing (Heavy) | Incremental / Real-time (Light) | **Advantage:** LM is zero-latency. |
| **Graph Topology** | Hierarchical Communities (Leiden) | Flat Graph (Nodes & Edges only) | **Critical Gap:** LM lacks structure. |
| **Summarization** | Pre-computed "Community Reports" | "Parent Summaries" for chunks only | **Critical Gap:** No thematic summaries. |
| **Global Search** | Map-Reduce over summaries | Not supported (Vector only) | **Critical Gap:** Cannot answer "What are the trends?" |
| **Local Search** | Entity Linking + 2-hop traversal | 3-Stage Funnel + 1-hop traversal | **Parity:** LM is faster; MS is deeper. |
| **Data Evolution** | Static snapshots | **Temporal Versioning & Conflict Detection** | **Advantage:** LM handles updates natively. |
| **Drift Search** | Local -> Global reasoning drift | None | **Gap:** Limited reasoning hops. |

## The "Global Search" Problem
In `local-memory`, if you ask: *"What are the main events happening in Boston?"*
1.  **Vector Search** might find specific event chunks if they share keywords.
2.  **Graph Search** might find the "Boston" node and its neighbors.
3.  **But:** It cannot synthesize a high-level answer if the individual facts are scattered across 50 different documents without a unifying summary.

In **MS GraphRAG**:
1.  The "Boston Events" community (cluster) already has a pre-generated summary.
2.  Global Search reads that summary and answers immediately.

## Proposed Roadmap: "Incremental GraphRAG"

To bridge this gap without sacrificing the "Local/Real-time" nature, we must invent an **Incremental** version of MS GraphRAG's batch features.

### Phase 1: Dynamic Communities
*   **Goal:** Group entities without running expensive Leiden on the whole graph every insert.
*   **Mechanism:** Use **Label Propagation** or **Streaming Clustering** to assign `community_id` to entities.
*   **Storage:** Add `communities` table to SQLite (`id`, `summary`, `level`).

### Phase 2: Background Summarization
*   **Goal:** Create the "Community Reports" lazily.
*   **Mechanism:** A background thread (like `GitObserver`) watches for "dirty" communities (where >N new nodes were added).
*   **Action:** It prompts the LLM to Generate/Update the summary for that community.

### Phase 3: Global Search Tool
*   **Goal:** Answer holistic questions.
*   **Mechanism:** New tool `memory_global_search`.
*   **Logic:** Instead of embedding the query, it asks the LLM to select relevant *Community Summaries* from the `communities` table, then synthesizes an answer from them.

### Phase 4: DRIFT / Hybrid Search
*   **Goal:** Connect specific facts to general themes.
*   **Mechanism:** When doing a Local Search, also check if the entity belongs to a Community. Inject that Community's summary into the context window.
