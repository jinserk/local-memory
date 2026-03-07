# Proposal: Knowledge Decay and Explicit Forgetting

## Problem
Knowledge graphs grow indefinitely. Without a mechanism to expire stale or irrelevant facts, retrieval performance can degrade, and old information may conflict with new realities.

## Solution
Implement a "Living Knowledge" mechanism where entities have a lifespan based on their utility.

### 1. Decay Factor
Each entity in the Knowledge Graph is assigned a `decay_factor` (float 0.0 to 1.0).
- **Initial State**: New entities start with 1.0.
- **Linear Decay**: A background routine reduces the factor daily. The target lifespan is 180 days (6 months) without interaction.
- **Recall Survival**: Any retrieval operation (search or exploration) that accesses an entity resets its `decay_factor` to 1.0.

### 2. Explicit Forgetting
A new operation `forget` allows agents to manually mark a fact as irrelevant by setting its `decay_factor` to 0.0.

### 3. Automated Maintenance
A 24-hour background service (`DecayService`) performs:
- `decay_factor` updates for all survivors.
- Deletion of entities with `decay_factor <= 0.0`.
- Cascaded deletion of relationships and orphaned communities.

### 4. Relevance Ranking
Retrieval and listing operations reorder results by `decay_factor DESC`, prioritizing "fresh" and frequently recalled knowledge.
