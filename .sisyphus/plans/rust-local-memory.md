# Plan: Rust Local Memory Plugin (MCP-based)

## TL;DR

> **Quick Summary**: Build a Rust-based local memory plugin for OpenCode and Claude-code using the Model Context Protocol (MCP). It features a high-performance 3-stage "funnel" search using Matryoshka Embeddings and Binary Quantization (BQ) for speed without HNSW.
> 
> **Deliverables**:
> - MCP Server Binary (Rust)
> - Multi-stage Vector Search Engine (Hamming -> Matryoshka -> Full)
> - Local Embedding Pipeline (Candle + nomic-embed-text-v1.5)
> - Persistent Storage (Fjall)
> - OpenSpec SDD & TDD Suite
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 5 Waves
> **Critical Path**: MCP Server -> Search Funnel Logic -> Embedding Integration -> Integration Tests

---

## Context

### Original Request
Build a Rust-based local memory plugin for OpenCode/Claude-code. Use Matryoshka Embeddings and multi-stage bit-wise search (N-bit -> 2N-bit -> Re-rank) instead of HNSW. 100% local operation.

### Interview Summary
**Key Discussions**:
- **Interface**: Adopted Model Context Protocol (MCP) for native compatibility with both OpenCode and Claude-code.
- **Search Strategy**: 3-stage funnel: 1-bit BQ Hamming -> 256d Matryoshka Cosine -> 768d Full Float Re-rank.
- **SDD/TDD**: Use "OpenSpec" as the single source of truth for design and testing.
- **Performance**: Use `simsimd` for hardware-accelerated Hamming distance.

### Metis Review
**Identified Gaps (addressed)**:
- **Model Management**: Use `Candle` for local execution of `nomic-embed-text-v1.5`.
- **Latency**: Plugin runs as a persistent MCP daemon to keep model weights in memory.
- **Verification**: Added Recall@10 benchmark tasks to ensure funnel accuracy.

---

## Work Objectives

### Core Objective
Implement a robust, persistent local memory system that AI agents can use via MCP to store and retrieve long-term context with minimal latency and high recall.

### Concrete Deliverables
- `bin/mcp-memory-server`: Rust binary implementing MCP.
- `lib/search-engine`: Core logic for bit-wise funnel search.
- `lib/storage`: Fjall-based persistence layer.
- `openspec/specs/local-memory.md`: Detailed design document.

### Definition of Done
- [ ] `cargo test` passes all units and integrations.
- [ ] `mcp-inspector` validates MCP server compliance.
- [ ] Search Recall@10 > 0.9 against brute-force baseline.
- [ ] Ingestion speed < 20ms per document (excluding embedding time).

### Must Have
- MCP `tools` for `memory_insert` and `memory_search`.
- Configurable funnel stages via JSON.
- SIMD acceleration for Hamming distance.
- Persistent index that survives restarts.

### Must NOT Have (Guardrails)
- No external C++ dependencies (Pure Rust).
- No cloud dependencies for embeddings (Fully Local).
- No HNSW or graph-based indexing (Stick to BQ + Slicing).

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO (will set up).
- **Automated tests**: YES (TDD).
- **Framework**: `cargo test` + `mcp-inspector`.
- **If TDD**: Every task starts with an OpenSpec delta and a failing test case in `tests/`.

### QA Policy
Every task includes agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/`.
- **MCP**: `mcp-inspector` or `curl`-like JSON-RPC calls over stdio.
- **Search**: Statistical comparison of multi-stage results vs brute-force.
- **Persistence**: Kill process -> Restart -> Verify data integrity.

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation & Specs):
├── Task 1: Initialize OpenSpec SDD [writing]
├── Task 2: Project Scaffolding (MCP Boilerplate) [quick]
├── Task 3: Fjall Storage Setup (Schema/Metadata) [unspecified-low]
└── Task 4: JSON Configuration Module [quick]

Wave 2 (Embedding & Ingestion):
├── Task 5: Candle Integration (Nomic model loader) [unspecified-high]
├── Task 6: Binary Quantization (BQ) Encoder [deep]
├── Task 7: Matryoshka Slicing Logic [quick]
└── Task 8: Document Ingestion Pipeline [unspecified-high]

Wave 3 (Search Funnel Implementation):
├── Task 9: Stage 1 Search (Hamming + SIMD) [deep]
├── Task 10: Stage 2 Search (Matryoshka Cosine) [deep]
├── Task 11: Stage 3 Search (Full Re-rank) [unspecified-high]
└── Task 12: Funnel Coordinator (Multi-stage Orchestrator) [artistry]

Wave 4 (Integration & Refinement):
├── Task 13: MCP Tool Wrappers (insert/search) [quick]
├── Task 14: Recall Benchmarking Suite [unspecified-high]
├── Task 15: Memory Tiering (Episodic/Semantic) [deep]
└── Task 16: CLI/TUI Diagnostics Tool [visual-engineering]

Wave 5 (Verification & Cleanup):
├── Task 17: E2E MCP Integration Tests [deep]
├── Task 18: Performance Profiling & SIMD Check [ultrabrain]
└── Task 19: Documentation & Deployment Guide [writing]

Wave FINAL:
├── Task F1: Plan Compliance Audit (oracle)
├── Task F2: Code Quality Review (unspecified-high)
├── Task F3: Integration QA (unspecified-high)
└── Task F4: Scope Fidelity Check (deep)
```

---

## TODOs

---

## Final Verification Wave

- [ ] F1. **Plan Compliance Audit** — `oracle`
- [ ] F2. **Code Quality Review** — `unspecified-high`
- [ ] F3. **Integration QA** — `unspecified-high`
- [ ] F4. **Scope Fidelity Check** — `deep`

---

## Commit Strategy
- `feat(mcp): desc`
- `feat(search): desc`
- `fix(storage): desc`
- `docs(spec): desc`

---

## Success Criteria

### Verification Commands
```bash
cargo test
mcp-inspector ./target/release/mcp-memory-server
```
