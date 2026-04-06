---
id: "006"
title: "Analysis: SQLite Concurrency Under MCP"
type: analysis
created: 2026-04-05
tags: [sqlite, concurrency, mcp, tokio, arc-mutex, connection-pooling]
---

# Analysis: SQLite Concurrency Under MCP

## Question

How does mengdie handle concurrent MCP tool calls with `Arc<Mutex<Connection>>`? Is connection pooling or a different concurrency model needed?

## Findings

### Prior Art from Project Knowledge Base

- **Tech Stack Selection** (decisional, `docs/discussions/003-tech-stack/conclusion.md`): Rust chosen for agent-centric guardrails. `rusqlite` with bundled SQLite selected as DB layer.
- **Hybrid Search Analysis** (factual, `docs/discussions/005-hybrid-search-analysis/analysis.md`): Brute-force cosine scan acceptable up to ~50K entries; sqlite-vec recommended beyond that threshold.

No prior knowledge on SQLite concurrency patterns specifically.

### Relevant Code

- **`src/core/db.rs:21-23`**: `Db` struct wraps `Arc<Mutex<Connection>>`. Single connection created at startup, shared via `Clone` (clones the Arc).
- **`src/core/db.rs:195-199`**: `lock_conn()` is `pub(crate)` with a comment warning against holding the guard while calling other `Db` methods.
- **`src/core/mcp_tools.rs:124-158`**: `memory_search` tool handler — `spawn_blocking` for embedding, then sync DB calls on async thread.
- **`src/core/mcp_tools.rs:258-263`**: `memory_ingest` — same pattern: `spawn_blocking` for embedding, sync DB for insert.
- **`src/core/mcp_tools.rs:17,379`**: Separate `Arc<Mutex<Embedder>>` for fastembed — independent from DB mutex.
- **`src/core/search.rs:110-128`**: `memory_search` makes 4+ sequential lock acquisitions (FTS → vector → get_memory × N → record_recall × N). No nested locking.
- **`src/core/vector.rs:79-107`**: `search_vector` holds conn mutex during O(N) cosine similarity loop over all embeddings.

### Architecture & Patterns

**Current pattern**: `Arc<Mutex<Connection>>` with `std::sync::Mutex`. All DB calls are synchronous, executed directly on Tokio worker threads. `spawn_blocking` is used only for fastembed inference, not for DB operations.

**What's correct**:
- No `.await` across mutex boundaries — the most dangerous Tokio footgun (deadlock from holding `std::sync::Mutex` across suspension points) is absent.
- Two independent mutexes (DB + Embedder) are never co-acquired — no ABBA deadlock risk.
- `INSERT ... ON CONFLICT DO UPDATE` provides atomic upsert — no write-write race on dedup.

**What's technically imperfect but harmless at current scale**:
- DB calls block Tokio worker threads directly (no `spawn_blocking`). Under stdio's serialized request model, there's nothing else competing for threads.
- `search_vector` holds the mutex during O(N) cosine computation. At MVP scale (<1K memories), this is microseconds. The hybrid search analysis already identified sqlite-vec as the fix for >50K entries.
- Contradiction check-then-insert is two separate lock acquisitions (TOCTOU gap). Advisory-only — missed conflict warning, not data corruption.

### Industry Practice Comparison

Three viable patterns for SQLite in async Rust, ranked by fit:

1. **Dedicated background thread** (`tokio-rusqlite`): Correct by construction — all DB work runs on a single OS thread, communicated via channels. Recommended by Tokio documentation for blocking I/O.
2. **Connection pool** (`deadpool-sqlite`, `r2d2`): Enables read parallelism. Overkill for single-process MCP with one writer.
3. **`spawn_blocking` per call**: Simpler but makes transactions awkward across `spawn_blocking` boundaries.

WAL mode enables concurrent readers + single writer (readers never block writers). Recommended with `PRAGMA busy_timeout` for any deployment expecting write concurrency. Not impactful under current serialized access.

**rmcp dispatch**: rmcp spawns per-request tasks — if the client pipelined requests, handlers would run concurrently. However, MCP stdio transport is inherently request-response sequential (one in-flight request at a time).

### Challenges & Disagreements

**Challenger's core thesis**: The "concurrency problem" is largely imaginary for stdio deployment. All severity ratings should be assessed against the actual deployment model (single-client, serial requests), not hypothetical concurrent scenarios.

Specific challenges:
- **search_vector O(N) cosine loop**: Archaeologist rated Medium; Challenger argues N/A for MVP — at 1000 entries, ~0.2ms of CPU work. The real fix is sqlite-vec (architectural, already planned), not `spawn_blocking` (which just moves the queue).
- **No spawn_blocking for DB**: Standards-expert flagged as best-practice violation; Challenger argues the precondition (competing async work) doesn't exist under stdio. Only becomes relevant when file watcher or parallel transport is added.
- **WAL mode**: Standards-expert recommends it; Challenger notes it adds operational complexity (`-wal`, `-shm` files, backup considerations) with zero benefit when there's no read/write concurrency.

**Cross-family (Codex)**: Confirmed rmcp can dispatch concurrently but mengdie's code is safe (no guard across await). Recommended keeping `Arc<Mutex>` for MVP, with `tokio::sync::Semaphore(1)` + `spawn_blocking` as the minimal escalation path.

**Consensus**: All agents agree the current approach is correct for Phase 1 MVP. Disagreement is only on when/whether to proactively migrate.

## Summary

**mengdie's `Arc<Mutex<Connection>>` is correct and safe for Phase 1 MVP under MCP stdio transport.** The code avoids the critical Tokio footgun (mutex held across `.await`) and there is no real concurrent access to contend over. The technically imperfect patterns (blocking Tokio threads, cosine loop under mutex, non-atomic contradiction check) have zero practical impact at current scale and deployment model.

No changes are warranted now. The current design should be revisited when any of these triggers fire:

| Trigger | What to do |
|---|---|
| File watcher runs on same Tokio runtime as MCP server | Wrap DB calls in `spawn_blocking` or migrate to `tokio-rusqlite` |
| Corpus grows beyond ~10K memories | Migrate vector search to sqlite-vec (already in VectorStore interface) |
| Transport changes to HTTP/SSE (concurrent clients) | Migrate to `tokio-rusqlite` or connection pool + WAL mode |
| Batch import or parallel ingestion added | Add `PRAGMA busy_timeout`, consider WAL mode |

## Possible Next Steps

- If these triggers should be formally tracked → add to `docs/backlog/` with trigger conditions
- If the watcher integration (Phase 2) is imminent → `/ae:discuss` the `tokio-rusqlite` migration as part of that work
- Otherwise → no action needed, proceed with other Phase 1.1 tasks
