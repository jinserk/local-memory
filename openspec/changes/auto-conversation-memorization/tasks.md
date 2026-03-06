## 1. Configuration & Scaffolding

- [ ] 1.1 Add `enable_conversation_observer` and `opencode_db_path` to `Config` in `src/config.rs`
- [ ] 1.2 Create `src/engine/conversation.rs` module and export it in `src/engine/mod.rs`

## 2. Conversation Extraction Logic

- [ ] 2.1 Implement `get_new_messages` helper to query OpenCode's `message` table for recent entries
- [ ] 2.2 Implement `reconstruct_conversation_step` to merge `part` entries into a cohesive text block
- [ ] 2.3 Implement extraction of role (user/assistant) and timestamps from the JSON `data` fields

## 3. Conversation Observer Implementation

- [ ] 3.1 Implement `spawn_conversation_observer` polling loop with state for tracking last processed ID/timestamp
- [ ] 3.2 Integrate with `IngestionPipeline` to call `run_with_namespace` using the `conversation` namespace
- [ ] 3.3 Add error handling for database locks and missing files

## 4. System Integration

- [ ] 4.1 Update `src/main.rs` to initialize and spawn the `ConversationObserver` when enabled
- [ ] 4.2 Update `openspec/config.yaml` to document the new proactive automation feature

## 5. Verification & Testing

- [ ] 5.1 Run `cargo test` and `cargo clippy` to ensure system stability
- [ ] 5.2 Verify observer correctly ingests a mock conversation entry into the GraphRAG
- [ ] 5.3 Verify `recall` returns relevant conversation context after ingestion
