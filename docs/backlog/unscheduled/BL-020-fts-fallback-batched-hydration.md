---
id: BL-020
title: "FTS-fallback path: batch-hydrate memory_entries to eliminate N+1 lock pattern"
status: open
created: 2026-04-30
origin: F-003 /ae:review (Performance reviewer P2)
trigger: "FTS fallback used in production with concurrency > 1, OR audit table grows past N rows where the per-call N+1 lock cycle creates observable tail latency. Earliest signal: a `mengdie stats` or `mengdie audit-stats` (BL-014) report shows degraded-mode latency exceeding the hybrid path by more than 2x."
---

# BL-020 — Batched hydration in `fts_only_with_normalization`

## What

`src/core/search.rs::fts_only_with_normalization` (introduced by F-003
Step 2) calls `db.search_fts(...)` to get a `Vec<FtsResult>` (1 lock
acquisition), then iterates and calls `db.get_memory(&fts.id)` per
result (N more lock acquisitions). At `limit = 10` this is 11 lock
acquisitions per FTS-fallback search call.

Pre-F-003, this same pattern existed only on the MCP path (the
inline FTS fallback in `mcp_tools::search`). F-003 makes it the
canonical fallback for both MCP and CLI surfaces — every embed-fail
on either surface now hits this path.

Add a batched method:

```rust
impl Db {
    pub(crate) fn get_memories_by_ids(&self, ids: &[String]) -> anyhow::Result<Vec<MemoryEntry>> {
        // Single SELECT ... WHERE id IN (?, ?, ...) ... query.
        // Returns entries in the order they appear in `ids` (or the
        // caller re-sorts by hashing the result Vec).
    }
}
```

Then `fts_only_with_normalization` reduces to 2 lock acquisitions
(one for FTS, one for batched hydration).

## Why it matters

`Arc<Mutex<Connection>>` contention is bounded today by mengdie's
single-writer model. As the operator workload scales (e.g., concurrent
MCP callers from multiple Claude Code sessions, or an embedding-model
outage causing every search to hit the FTS-fallback path), the N+1
pattern amplifies tail latency. Hybrid path is unaffected (uses
`db.memory_search` which already batches internally).

## Trigger

File the implementing plan when ANY of:

1. Production telemetry shows FTS-fallback path P95 latency exceeding
   hybrid path P95 by more than 2x (signal: BL-014 `mengdie audit-stats`
   ships and reports this; or operator-side benchmark on a real corpus).
2. Concurrent MCP caller count grows past 1 (e.g., multiple Claude
   Code sessions sharing the mengdie MCP server) AND embedding outages
   become a recurring scenario (signal: stderr `tracing::warn!` on
   embed-fail occurs frequently enough that operators notice tail
   latency).
3. Audit table reaches a size where `audit_returned_facts` JOIN cost
   in BL-014 / BL-016 queries becomes dominant (signal: BL-016 trigger
   fires concurrently — the perf concerns share the same operating
   regime).

Until any of these fire, the N+1 cost is bounded by FTS-fallback
frequency (only fires on embed-fail) and v0.0.1 single-writer
concurrency.

## Implementation sketch (when triggered)

1. Add `Db::get_memories_by_ids(&[String]) -> Vec<MemoryEntry>` in
   `src/core/db.rs` (next to `get_memory`).
2. Use parameterized SQL with `?` placeholders for each id (rusqlite
   supports `params_from_iter`).
3. Replace the loop in `fts_only_with_normalization`:
   ```rust
   // Before (current F-003):
   for (idx, fts) in fts_results.iter().enumerate() {
       if let Some(entry) = db.get_memory(&fts.id)? {
           results.push(SearchResult { entry, score: normalized_scores[idx] });
       }
   }

   // After:
   let ids: Vec<String> = fts_results.iter().map(|f| f.id.clone()).collect();
   let entries = db.get_memories_by_ids(&ids)?;
   for (entry, score) in entries.into_iter().zip(normalized_scores) {
       results.push(SearchResult { entry, score });
   }
   ```
4. Tests: extend `test_memory_search_audited_hybrid_or_fts_only_falls_back_with_normalized_scores`
   to verify the batched-hydration path returns the same results in the
   same order as the per-result loop.

## Why not now (F-003 scope)

F-003 plan AC1-AC11 cover the orchestrator + ingest consolidation. The
N+1 pattern is a pre-existing implementation detail of the FTS-fallback
path; F-003 inherits it, doesn't introduce it. Refactoring it inline
during F-003 Step 2 would have expanded the BL-010 PR scope without
addressing a triggered concern. Backlog with explicit trigger keeps
the BL-010 PR surgical.

## Reviewer note

Performance reviewer in F-003 /ae:review. The full review is captured
in `.ae/features/active/F-003-retrieval-and-ingest-layer-consolidation/review.md`
under "P2-1 — N+1 lock pattern in `fts_only_with_normalization`".
