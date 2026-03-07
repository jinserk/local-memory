# Tasks: Knowledge Decay and Explicit Forgetting

- [x] T101: Add `decay_factor` and `last_recalled_at` to `entities` table schema.
- [x] T102: Initialize new entities with `decay_factor = 1.0`.
- [x] T103: Implement `recall_entity` to reset `decay_factor` to 1.0 on retrieval.
- [x] T104: Implement `process_decay` routine to update factors and prune zero-factor entities.
- [x] T105: Ensure cascaded deletion of relationships for decayed entities.
- [x] T106: Add maintenance routine for pruning orphaned communities.
- [x] T107: Implement `forget_entity` method in `SqliteDatabase`.
- [x] T108: Create `DecayService` background task in `src/engine/decay.rs`.
- [x] T109: Register `forget` tool in MCP provider.
- [x] T110: Update all entity listing methods to filter and order by `decay_factor`.
- [x] T111: Add unit and integration tests for decay and forgetting.
- [x] T112: Update `ARCHITECTURE.md` and `Core.md` specifications.
