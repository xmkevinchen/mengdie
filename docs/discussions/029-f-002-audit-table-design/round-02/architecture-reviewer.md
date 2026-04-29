---
id: "029-round-02-architecture-reviewer"
role: architecture-reviewer
round: 2
created: 2026-04-28
topics: [topic-01-hook-placement, topic-02-failure-mode, cli-wiring, wave2-migration, module-boundary]
verdict: CONVERGE
---

# Round 2 — Architecture Reviewer

## Findings

### UAG-1: Topic 1 — find a concrete v0.0.1 scenario where Option A is strictly better

Attempt to falsify Option B by finding a scenario where Option A (hook inside
`Db::memory_search`) produces a better outcome than Option B (`mcp_tools.rs`).

**Candidate scenario 1: CLI coverage without separate wiring**

Under Option A, `cli.rs:609` calls `Db::memory_search` and the hook fires
automatically — no extra wiring needed. Under Option B, CLI must be wired
separately (shared writer called from `cli.rs`). Does this make Option A better?

Evaluation: the CLI-coverage cost under Option B is one shared-function call at
`cli.rs:609`. The archaeologist confirmed this (round-01/archaeologist.md, §4):
CLI calls `Db::memory_search` directly, so the shared writer would be placed
adjacent to that call. The wiring cost is approximately 3 lines: extract
result IDs from the returned `Vec<SearchResult>`, call
`audit_search_event(db, query, project_id, took_ms, &fact_ids)`. This is not
a hidden structural cost; it is a trivial call-site addition.

Furthermore, the CLI does NOT have an FTS-fallback path (archaeologist.md §4:
"if embedding fails, `cli.rs:607` returns an error before reaching the search
call"). So under Option A, CLI auto-coverage is real but only covers hybrid
searches. Under Option B with a shared writer at `cli.rs:609`, coverage is
identical in scope (CLI only ever reaches `memory_search` with a valid
embedding). The "auto-coverage" advantage of Option A for CLI is therefore
**vacuous** — both options produce identical CLI audit coverage.

**Candidate scenario 2: Preventing Option B's temporal gap between search and audit**

Under Option A, the audit write happens inside the `Db::memory_search` call
(before the function returns results to `mcp_tools.rs`). Under Option B, the
audit write happens after `memory_search` returns but before `mcp_tools.rs`
constructs its MCP response. Could a process crash in the sub-millisecond
window between these two events create a materially different outcome?

Evaluation: the archaeologist confirmed (round-01/archaeologist.md §1) that
`Db::memory_search` already releases the mutex 12 times per call. Option A's
hook inside `memory_search` would fire between two of those releases — not at
the function return boundary but at some mid-body point. Option B's hook fires
after the function returns. Both options have the same atomicity profile under
best-effort (which is the converged Topic 2 answer): neither guarantees that
the audit row and the search response land in the same durability unit. The
temporal gap between Option A and Option B is approximately the time for
`mcp_tools.rs` to assign `results` from the return value — nanoseconds. This
is not a material difference.

**Candidate scenario 3: Option A as prerequisite for future transaction-coupled**

If a future BL wants transaction-coupled semantics, Option A (hook inside
`Db::memory_search`) is nominally closer to where a wrapping transaction would
live. Does this make Option A a better foundation for the future?

Evaluation: the archaeologist and database-optimizer both confirmed (round-01
§1 and database-optimizer.md §1.C) that transaction-coupled under EITHER
option requires restructuring `Db::memory_search` to hold one lock guard
end-to-end across all sub-calls. This restructuring is equally non-trivial
under Option A. The framing's "only available under Option A" claim was an
overstatement. Option A does not provide a materially cheaper path to
transaction-coupled; it just places the hook closer to where the restructuring
would happen — but the restructuring itself is the same size of work either way.
Moreover, under best-effort consensus (Topic 2), transaction-coupled is not a
v0.0.1 requirement, so this forward-looking advantage has zero v0.0.1 weight.

**UAG-1 verdict**: no counterexample found. All three candidate scenarios
either produce identical outcomes under both options or the Option-A advantage
dissolves under scrutiny. Topic 1 **converges to Option B** (mcp_tools.rs).

---

### UAG-2: Topic 2 — find an A-MEM scenario where best-effort under-counting causes wrong-direction trigger outcome

The trigger from 028 conclusion: "≥5 superseded-within-7-days events per
rolling 30-day window."

The under-counting failure mode: audit writes fail during transient disk or
locking events, causing some search calls to have no audit row. The
supersession-rate signal under-counts during those windows.

**Wrong-direction scenarios to attempt**:

**False negative (trigger should fire but doesn't)**: suppose the operator
conducts exactly 5 searches in a 30-day window, all resulting in superseded
facts, and exactly 1 audit write fails. The count drops to 4 — below the
threshold. The A-MEM trigger doesn't fire when it "should."

Is this wrong-direction? Analysis: the trigger threshold is a calibration
parameter ("numbers within operator calibration discretion at BL filing time"
per 028 conclusion). A threshold of "≥5" with a corpus of 5 actual events is
operating at the edge case of the threshold itself. If the operator has
only 5 qualifying searches in 30 days, their corpus is likely below the
1,000-fact floor that is the other A-MEM trigger precondition — the trigger
would not fire regardless. The volume metric has built-in headroom by design.

More importantly: the failure direction is always "trigger fires late or not
at all" — it never fires early because under-counting makes the count look
higher than it is. The trigger cannot produce a false positive under
best-effort audit loss. Wrong-direction = a false positive, which best-effort
cannot produce by construction.

**False positive (trigger fires when it shouldn't)**: not possible under
audit-write failures. Missing audit rows reduce the count; they cannot inflate
it. Best-effort failure mode is strictly one-directional (under-count), which
means the signal is always conservative.

**Pathological case: embedding-outage window creates systematic blind spot**:
if the embedding model is broken for several days, ALL searches during that
window are FTS-fallback searches. Under Option A, zero audit rows would be
written for that entire period. Under Option B (converged), all searches
including FTS-fallback are audited — this case does not apply to Option B.
The Topic 2 question is about audit-write failures specifically (disk full,
WAL stall), not about coverage gaps (Topic 1 already addressed those by
choosing Option B). Under Option B, FTS-fallback searches ARE audited; audit
failures are transient (disk full) not systematic. A systematic disk-full
condition would also prevent dreaming, ingestion, and contradiction updates —
the operator would have bigger problems, and the audit table's under-counting
would be the least of their concerns.

**UAG-2 verdict**: no counterexample found. Best-effort under-counting cannot
produce a wrong-direction (false positive) trigger outcome by construction.
False negatives require operating at the exact threshold edge, which is already
guarded by the corpus-floor precondition. Topic 2 **converges to best-effort +
warn + `METRIC_AUDIT_WRITE_FAILURES`**.

---

## Agreements

1. **Option B is correct for Wave 1** (with Qwen3-Coder, archaeologist, db-optimizer).
   Consensus across all architecturally-motivated Round 1 reports.

2. **Best-effort + warn + metric is correct for Topic 2** (unanimous 5-of-5 in Round 1).
   Database-optimizer's failure-mode taxonomy is the load-bearing evidence:
   every realistic v0.0.1 failure category is transient or operator-actionable;
   none require hard-error propagation.

3. **The `record_recall` pattern is the right shape for the call site**
   (archaeologist round-01 §3). The best-effort contract is applied at the
   call site via `if let Err(e) = ... { tracing::warn!(...) }`, not inside
   the helper function itself. F-002's audit writer should follow this pattern.

4. **Sync audit write is acceptable** (database-optimizer §2). Transaction-coupled
   overhead is <1ms at v0.0.1 corpus; standalone implicit-tx INSERT is even
   lower. The Qwen3-Coder "should audit be async?" question has a clear answer:
   no. Sync-with-mutex-acquire is fine at single-operator QPS. Async introduces
   backpressure, error reporting, and task-spawn overhead that is not justified
   at <10 QPS.

5. **Archaeologist's mutex-cycle finding is load-bearing** (cross-confirmed by
   database-optimizer). `Db::memory_search` does 12 lock acquire/release cycles.
   This makes transaction-coupled equally expensive under both option A and B —
   not just "harder under B." The framing's Topic 1↔Topic 2 feasibility coupling
   was overstated.

---

## Disagreements

None remaining. All Round 1 disagreements have been resolved by the UAG
falsification attempts above or by the archaeologist's codebase verification.

---

## UAG Status Per Topic

| Topic | UAG attempt | Counterexample found | Status |
|-------|-------------|---------------------|--------|
| Topic 1 — hook placement | 3 candidate scenarios tested | None survived scrutiny | **CONVERGE: Option B** |
| Topic 2 — failure mode contract | 2 scenario classes tested (false positive impossible; false negative requires edge-case calibration already guarded by corpus-floor) | None survived scrutiny | **CONVERGE: best-effort + warn + metric** |

---

## Open Questions

### OQ-1: Async audit — resolved

Qwen3-Coder (codex-proxy.md) asked whether audit should be async.
Resolution: no. Database-optimizer confirmed sync overhead is <1ms. Async
`tokio::spawn` introduces a task allocation, a potential backchannel for
errors that the caller ignores anyway, and complicates the "what does the
operator see" story. Best-effort sync write with `if let Err` at the call site
is simpler, has the same observable behavior, and avoids the async-task
lifecycle complexity. Ratified.

### OQ-2: Index design — ratified

Codex original three-index design (from F-002 analysis, validated by
database-optimizer round-01):

1. `CREATE INDEX idx_memory_search_audit_searched_id ON memory_search_audit(searched_at, id)` — composite for window + ID lookup.
2. `CREATE INDEX idx_audit_returned_facts_fact_audit ON audit_returned_facts(fact_id, audit_id)` — reverse FK direction, critical for FK-maintenance scan performance if PRAGMA is ever flipped.
3. `CREATE INDEX idx_memory_entries_valid_until_id ON memory_entries(valid_until, id) WHERE valid_until IS NOT NULL` — partial index on supersession join filter column.

Architecture call: ratified. Index 3 is on `memory_entries`, not on the new
audit tables — it is a query-performance index for the supersession SQL, not
an audit-table concern. All three are correct. No EXPLAIN QUERY PLAN
verification was run, but database-optimizer's structural analysis is
sufficient for v0.0.1 corpus scale.

---

## Ratifications

### R1: CLI wiring shape

**Question**: should `audit_search_event(...)` helper live in `mcp_tools.rs`,
in a new instrumentation module, or in `Db`?

**Evidence base**:
- `metrics.rs` analogy (brief prompt): `metrics.rs` lives at `src/core/metrics.rs`
  and exports `METRIC_*` string constants. `mcp_tools.rs` calls
  `self.db.increment_metric(metrics::METRIC_SEARCH_COUNT)` — the metric
  increment is a `Db` method, but the constant is defined in a peer module.
  The pattern is: constants in a dedicated module, operation as a Db method.
- `mcp_tools.rs` is the current consumer of both `METRIC_SEARCH_COUNT` and
  `METRIC_SEARCH_NONEMPTY` (confirmed at lines 267 and 269). CLI likely does
  not call these directly (it uses the same `Db` methods but may not increment
  the same metrics).
- The audit write is not a metric increment — it is a multi-row DB operation
  (INSERT into two tables + collect fact IDs). The metrics analogy applies to
  the CONSTANT naming (`METRIC_AUDIT_WRITE_FAILURES`) but not to the write
  helper's home.

**Architecture call**:

`audit_search_event(...)` should live in `src/core/db.rs` (or equivalently,
as a free function in a new thin `src/core/audit.rs` module if db.rs grows
large) — **NOT** in `mcp_tools.rs`. Rationale:

1. The function's signature is `(db: &Db, query: &str, project_id: Option<&str>, took_ms: i64, fact_ids: &[String]) -> anyhow::Result<()>`. It is a storage operation against the audit tables — it belongs in the storage layer, even though the *call site policy* (best-effort via `if let Err`) lives in the protocol layer.

2. `cli.rs` needs to call the same function. If the function is defined in
   `mcp_tools.rs`, `cli.rs` would import from `mcp_tools` — that's the wrong
   dependency direction. `cli.rs` should depend on `core::db` (or `core::audit`),
   not on `core::mcp_tools`.

3. Wave 2 compatibility: when BL-010 moves `search::memory_search` to a free
   function in `search.rs`, the caller of `audit_search_event` changes from
   `mcp_tools.rs` to `search::memory_search_audited` free function. A helper
   defined in `mcp_tools.rs` would create an awkward import from `search.rs` →
   `mcp_tools.rs`. A helper defined in `db.rs` (or `audit.rs`) is imported by
   both `mcp_tools.rs` and `search.rs` equally — the correct dependency shape.

4. The `record_recall` analogy is exact: `record_recall` is a Db method called
   from `search.rs:188` with best-effort handling at the call site. The audit
   write is the same pattern, just with a richer signature.

**Ratified placement**: `Db::write_search_audit(...)` method on `impl Db` in
`db.rs` (or extracted to `src/core/audit.rs` if db.rs is already large). The
method propagates errors; call sites apply best-effort via `if let Err`.

### R2: Wave 2 migration cost under Option B

**Question**: under Option B, when BL-009+BL-010 land, where does the audit
hook go?

**Answer**:

The Wave 2 refactor produces a free function `search::memory_search_audited`
(per 028 Topic 1 decision: free functions over `&Db`, not `impl Db` methods).
This function:
1. Computes embedding (or receives it as `Option<Vec<f32>>` pre-computed).
2. Dispatches to hybrid or FTS-only path (the BL-009 fix consolidates this).
3. Calls `db.write_search_audit(...)` (or `Db::write_search_audit`).
4. Returns `Vec<SearchResult>`.

`mcp_tools.rs` then calls `search::memory_search_audited(...)` and removes
its current audit hook. `cli.rs` also calls `search::memory_search_audited`
(or keeps calling `Db::memory_search` + `Db::write_search_audit` separately —
the Wave 2 BL can decide which surface is cleaner for CLI).

**Migration cost**: the audit hook in `mcp_tools.rs` is deleted; a new call
to `db.write_search_audit(...)` is added inside `search::memory_search_audited`.
The schema is untouched. The `Db::write_search_audit` method is unchanged.
This is a pure code-move. Cost is low.

**Stays or moves?** The hook does NOT stay in `mcp_tools.rs` after Wave 2 —
it moves to `search::memory_search_audited`. But `mcp_tools.rs`'s complexity
drops correspondingly. Net: cleaner separation after Wave 2.

### R3: Module-boundary precedent

**Question**: does mengdie's existing pattern give a clear answer for where
the audit helper lives?

**Evidence**: `metrics.rs` at `src/core/metrics.rs` defines constants; `Db`
methods call `self.increment_metric(key)` which writes to the `metrics` table.
`mcp_tools.rs` calls `self.db.increment_metric(metrics::METRIC_SEARCH_COUNT)`.

The pattern is: storage operations on mengdie's DB tables belong on `impl Db`
(or as free functions over `&Db`). Cross-cutting observability concerns
(incrementing counters, writing audit rows) are DB-layer operations because
they write to `~/.mengdie/db.sqlite`. The CALLER (mcp_tools, cli) decides the
error-handling policy (best-effort vs propagate), not the helper itself.

`METRIC_AUDIT_WRITE_FAILURES` follows the same pattern: a `metrics::` constant
incremented via `self.db.increment_metric(...)` at the best-effort call site
in `mcp_tools.rs` when `db.write_search_audit(...)` returns `Err`.

**Ratified**: the module-boundary precedent from `metrics.rs` confirms that
`Db::write_search_audit` is the correct placement. No new module needed for
v0.0.1 — fit it into `db.rs` alongside the other write helpers.

---

## Summary

| Item | Decision | Confidence |
|------|----------|------------|
| Topic 1 UAG — Option A counterexample | None found | HIGH |
| Topic 1 — converge to Option B | CONVERGE | HIGH |
| Topic 2 UAG — best-effort wrong-direction counterexample | None found | HIGH |
| Topic 2 — converge to best-effort + warn + metric | CONVERGE | HIGH |
| Async audit (OQ-1) | Reject — sync is correct | HIGH |
| Index design (OQ-2) | Ratify codex three-index design | HIGH |
| CLI wiring shape | `Db::write_search_audit` method; called from both mcp_tools.rs and cli.rs | HIGH |
| Wave 2 migration cost | Hook moves to `search::memory_search_audited` free fn; schema unchanged | HIGH |
| Module-boundary precedent | metrics.rs confirms Db-layer placement for write helpers | HIGH |
