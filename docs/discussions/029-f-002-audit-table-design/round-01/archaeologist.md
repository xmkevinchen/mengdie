---
agent: archaeologist
round: 01
topics: [01-hook-placement, 02-failure-mode]
---

# Round 1 — archaeologist findings

## Findings (with file:line evidence)

### 1. Mutex/lock model in `Db::memory_search`

`Db::memory_search` at `src/core/search.rs:152` does NOT hold a single lock for its entire duration. It acquires and releases the `Arc<Mutex<Connection>>` multiple times across its body:

- `search.rs:163` — calls `self.search_fts(...)`, which acquires the lock at `search.rs:89` via `self.lock_conn()` and releases it on return.
- `search.rs:164` — calls `self.search_vector(...)`, which acquires the lock at `vector.rs:51` via `self.lock_conn()` and releases it on return.
- Inside the post-RRF loop (`search.rs:183-195`), for each result:
  - `search.rs:184` — calls `self.get_memory(id)`, which acquires the lock at `db.rs:167` via `self.conn.lock()` and releases it.
  - `search.rs:188` — calls `self.record_recall(id, normalized)`, which acquires the lock at `db.rs:261` via `self.conn.lock()` and releases it.

`lock_conn()` is defined at `db.rs:544` as a simple `self.conn.lock()` wrapper returning a `MutexGuard`.

**Critical consequence for framing's transaction-coupled feasibility claim**: the framing states "only available if the hook lands inside `Db::memory_search` (Topic 1 option A); under option B the connection mutex is released before `mcp_tools.rs` can open a wrapping transaction." This claim is **partially correct** but imprecise. Even inside `Db::memory_search`, the lock is **not** held continuously — it is acquired and released per sub-call. A wrapping `BEGIN IMMEDIATE` transaction cannot be trivially added to `memory_search` without restructuring the function to hold one lock guard for the entire body. The transaction-coupled option is harder than the framing implies regardless of whether the hook is in option A or option B.

### 2. FTS-fallback path: does it bypass `Db::memory_search`? Does it have fact IDs?

Confirmed: `mcp_tools.rs:220-244` is indeed the FTS-fallback path. When `query_embedding` is `Err(e)`, execution falls to `mcp_tools.rs:220`, which calls `self.db.search_fts(&params.query, project_id, limit)` directly — bypassing `Db::memory_search` entirely.

`Db::search_fts` returns `Vec<FtsResult>` where `FtsResult { id: String, bm25_score: f64 }` (`search.rs:29-33`). The fact IDs are present in the `id` field of each `FtsResult`.

The fallback loop (`mcp_tools.rs:225-233`) calls `self.db.get_memory(&fts.id)` for each result and builds `Vec<SearchResult>`. After the loop, `results` is a `Vec<SearchResult>` containing `entry.id` for every returned fact.

**Conclusion**: the FTS-fallback path DOES have the fact IDs available (`fts.id` from `FtsResult`, or `entry.id` from the hydrated `SearchResult`). An audit hook placed at `mcp_tools.rs` after the closing brace of the `match query_embedding` block (line ~244) would have access to `results` with all fact IDs from whichever path executed.

### 3. `record_recall` at `db.rs:259-272`

Exact code at `db.rs:258-272`:
```rust
pub fn record_recall(&self, id: &str, relevance_score: f64) -> anyhow::Result<bool> {
    let now = Utc::now().to_rfc3339();
    let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
    let rows = conn.execute(
        "UPDATE memory_entries SET
            avg_relevance = (avg_relevance * recall_count + ?1) / (recall_count + 1),
            recall_count = recall_count + 1,
            last_recalled = ?2
         WHERE id = ?3",
        params![relevance_score, now, id],
    )?;
    Ok(rows > 0)
}
```

The call site in `memory_search` (`search.rs:188-190`):
```rust
if let Err(e) = self.record_recall(id, normalized) {
    tracing::warn!(id = %id, error = %e, "failed to record recall");
}
```

**Framing characterization accuracy**: the framing says `record_recall` is "best-effort + warn." This is **partially correct** but the framing slightly mischaracterizes the mechanism. `record_recall` itself propagates errors (it returns `anyhow::Result<bool>` and uses `?` on the execute). The best-effort treatment is at the **call site** in `memory_search` — the caller uses `if let Err(e)` with a `tracing::warn!` and discards the error. The function itself is not silently best-effort; the caller makes it best-effort. This distinction matters for F-002: if audit writes are in a dedicated function, the best-effort contract must be applied at the call site, not inside the function. The framing's description is functionally accurate (errors are warned and discarded), but the implementation location of the best-effort handling is the call site, not the callee.

### 4. CLI search call site at `src/bin/cli.rs:609`

```rust
let results: Vec<_> = db
    .memory_search(query, &query_embedding, scope, limit)?
    .into_iter()
    .filter(|r| min_score.is_none_or(|ms| r.score >= ms))
    .collect();
```

The CLI uses the `?` operator — any error from `memory_search` propagates and aborts the command. The CLI does NOT use the FTS-fallback path; it calls `memory_search` directly after generating an embedding with `embedder.embed_text(query)` at `cli.rs:607`. If embedding fails, `cli.rs:607` returns an error before reaching the search call.

**Divergence from MCP path**: the CLI has no FTS-only fallback. The MCP path (`mcp_tools.rs:220`) degrades gracefully to FTS when embedding fails; the CLI hard-errors and returns. This means:
- Hook inside `Db::memory_search` (option A): CLI is auto-covered. CLI only ever reaches `memory_search` with a valid embedding, so only hybrid-path searches are audited from CLI — matches the CLI's code path.
- Hook inside `mcp_tools.rs` (option B): CLI is NOT auto-covered. A separate wire-up (shared writer called from `cli.rs`) would be required.

### 5. `rename_project` DELETE coupling at `src/core/db.rs:636`

The DELETE at `db.rs:636` fires inside a `rusqlite` transaction (`tx`) opened at `db.rs:616` via `conn.transaction()`. The transaction is committed at `db.rs:646`. The DELETE statement is:
```rust
tx.execute("DELETE FROM memory_entries WHERE id = ?1", params![id])?;
```

This fires only for **collision rows** — entries where the same `content_hash` already exists under both the old and new `project_id`. Non-collision rows are renamed via `UPDATE`, not deleted.

**FK enforcement**: `rename_project` holds the mutex lock for its entire duration (it calls `self.conn.lock()` at `db.rs:615` and holds the guard through `tx.commit()`). If `PRAGMA foreign_keys = ON` were ever set, this DELETE could violate the FK from `audit_returned_facts.fact_id → memory_entries.id` (if the deleted entry has audit rows). With PRAGMA OFF (current state), the DELETE succeeds regardless and no cascade or restriction fires. The framing's claim that the rename_project path is the load-bearing concern for future FK enforcement is confirmed by evidence.

### 6. Connection-lock duration for the existing MCP search call

The `Arc<Mutex<Connection>>` lock is **not** held across the entirety of `Db::memory_search`. The lock is acquired and released per-operation:

From `mcp_tools.rs:209-218`, the call to `self.db.memory_search(...)` does not itself hold any lock — the lock is managed internally by sub-calls within `memory_search`. After `memory_search` returns, the lock is fully released. `mcp_tools.rs` has no direct access to the lock guard.

Timing of lock acquisition/release within a single `memory_search` call:
1. Lock acquired in `search_fts` → released before `search_vector` starts.
2. Lock acquired in `search_vector` → released before the post-RRF loop.
3. For each result in the loop: lock acquired in `get_memory` → released → lock acquired in `record_recall` → released.

The lock is released between FTS query and vector query. The framing states "Hook runs inside the same `Arc<Mutex<Connection>>` lock as the search statements" as an assumption to validate — this assumption is **false** as stated. No single lock scope contains both the search statements and a potential audit write. Any transaction-coupled implementation would require restructuring to hold one lock guard across the full `memory_search` body.

---

## Agreements (with framing claims, with evidence)

1. **FTS-fallback bypasses `Db::memory_search`**: Confirmed. `mcp_tools.rs:220-244` calls `self.db.search_fts(...)` directly, skipping `search.rs:152`. Evidence: `mcp_tools.rs:223`.

2. **`search_fts` returns fact IDs**: Confirmed. `FtsResult { id: String, bm25_score: f64 }` at `search.rs:29-33`. IDs are available.

3. **`record_recall` is best-effort at the call site**: Confirmed with clarification. The `if let Err(e) ... tracing::warn!` pattern at `search.rs:188-190` is exactly best-effort + warn.

4. **No FK enforcement (PRAGMA OFF)**: Confirmed. `Db::open` at `db.rs:91-101` never sets `PRAGMA foreign_keys = ON`. The framing's pre-decided item 1 is accurate.

5. **`rename_project` DELETE is the only DELETE-from-memory_entries path**: Evidence shows it fires only on collision rows within a transaction. Framing's characterization is correct.

6. **CLI call site is at `cli.rs:609` calling `Db::memory_search` directly**: Confirmed.

7. **No internal callers of `Db::memory_search`**: Confirmed. Only `mcp_tools.rs:211`, `cli.rs:609`, and the ignored `e2e.rs:64` test.

---

## Disagreements (with framing claims, with evidence)

### D1: Transaction-coupled feasibility under option A is harder than framing implies

**Framing claim** (Topic 2): "transaction-coupled is only available if the hook lands inside `Db::memory_search` (Topic 1 option A); under option B the connection mutex is released before `mcp_tools.rs` can open a wrapping transaction."

**Evidence**: `Db::memory_search` does NOT hold one continuous lock. Each sub-call (`search_fts`, `search_vector`, `get_memory`, `record_recall`) acquires and releases the lock independently. There is no single lock scope in `memory_search` that spans the entire search. A `BEGIN IMMEDIATE` transaction wrapping both search reads and the audit write would require holding one `MutexGuard` across ALL sub-calls (restructuring `memory_search` to not delegate to lock-acquiring sub-methods) OR a different approach (e.g., a dedicated transaction function that runs all reads in one lock scope).

**Correction**: transaction-coupled is not simply "available" under option A. It requires non-trivial restructuring of `memory_search`. The feasibility gap between option A and option B is smaller than the framing suggests for transaction-coupled: both require work. Under option B, the caller can still issue a dedicated `BEGIN IMMEDIATE` via a new function that wraps search + audit in one `lock_conn()` scope — but option B's callers don't have direct access to `lock_conn()` (it is `pub(crate)`).

### D2: FTS-fallback fact IDs are available at `mcp_tools.rs` after the match block

**Framing**: states the FTS-fallback excludes fact IDs as a coverage gap if the hook is inside `Db::memory_search`. This is correct. But the framing does not explicitly confirm that fact IDs ARE available at the `mcp_tools.rs` hook location under option B.

**Evidence**: after `mcp_tools.rs:244`, the `results: Vec<SearchResult>` variable contains `entry.id` for every returned fact from both the hybrid path and the FTS-fallback path. The audit hook at this location has the complete `(query, project_id, returned_fact_ids)` tuple from both paths with no additional changes needed. This strengthens the case for option B (mcp_tools hook placement).

---

## Open Questions (for other agents to resolve)

1. **A-MEM trigger algorithm completeness requirement**: does the "≥5 events per 30-day window" trigger algorithm require strict signal completeness (i.e., will a 10% under-count from FTS-fallback audit gaps cause false negatives), or is the threshold set with enough headroom to tolerate probabilistic loss? This is the key question for Topic 2 that cannot be answered from the codebase alone — requires reading the 028 conclusion's trigger specification.

2. **Transaction-coupled implementation path under option A**: given that `Db::memory_search` releases the lock between sub-calls, what is the concrete implementation shape for transaction-coupled? Options: (a) restructure `memory_search` to hold one lock across all sub-calls and add the transaction there; (b) extract a new `Db::memory_search_transactional` variant; (c) accept that transaction-coupled under the current architecture requires architectural change, narrowing the Topic 2 choice to best-effort or hard-error.

3. **Hard-error MCP contract**: under hard-error failure mode, when the audit write fails, the MCP caller receives an error response. The framing notes this but does not specify whether rmcp propagates the `anyhow::Error` as a structured MCP error or an unstructured string. What does the MCP caller (AI agent) actually see, and is it distinguishable from a search failure? Requires reading `mcp_tools.rs` error-propagation code.

4. **Wave 2 migration cost**: BL-009 + BL-010 will refactor the search path. If the audit hook is placed in `mcp_tools.rs` (option B), and BL-009 moves FTS + hybrid paths to a new consolidated location, the hook moves too. Is the expected BL-009 refactor shape already known? If so, placing the hook at option B may mean it moves twice (F-002 → Wave 2 refactor). Other agents should assess whether this migration cost is material or trivial.
