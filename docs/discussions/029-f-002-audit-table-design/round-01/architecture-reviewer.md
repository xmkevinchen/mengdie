---
id: "029-round-01-architecture-reviewer"
role: architecture-reviewer
round: 1
created: 2026-04-28
topics: [topic-01-hook-placement, topic-02-failure-mode]
verdict: ""
---

# Round 1 — Architecture Reviewer

## 1. Topic 1 — Module Boundary Implications

### Option A: Hook inside `Db::memory_search` (search.rs:152)

`Db` in this codebase is a storage primitive: it manages connection state, schema
migrations, and low-level read/write operations. The methods on `impl Db` in `search.rs`
are retrieval operations — they execute SQL, return rows, update recall stats. They are
not instrumentation points for domain-level observability.

Placing the audit hook inside `Db::memory_search` crosses the boundary: it would mean
the storage primitive knows about the domain-level concept of "a search invocation that
needs to be recorded for A-MEM trigger calculation." The audit table is **not** a
consequence of storage-layer behavior — it is a consequence of the operator invoking
search, which is a domain event. Storage primitives should not have opinions about which
of their callers constitute auditable events.

Additionally, `Db::memory_search` does not have access to `took_ms` in a meaningful
way — the timing should start at the point where the user's request begins, which is in
the protocol or application layer, not inside a DB method that has no knowledge of
embedding inference time (which is computed in `mcp_tools.rs` before the call is made).

**Verdict on Option A**: violates the "Db is a storage primitive" boundary. Also
structurally incomplete — it cannot capture full `took_ms` spanning embedding + search.

### Option B: Hook inside `mcp_tools.rs` after the `match query_embedding` block

`mcp_tools.rs` is the MCP protocol handler — it is the point where operator intent is
resolved (query string, scope → resolved project_id, embedding inference, result
filtering). This is the **protocol layer**, not the domain layer.

Placing audit semantics here puts domain knowledge (what constitutes an auditable search
event, what the supersession-rate trigger requires, how to write a normalized scope into
the audit row) into the protocol handler. This is the symmetric violation: domain logic
in the protocol layer.

However, this violation is **weaker** than Option A's violation, for two reasons:
1. `mcp_tools.rs` already has domain knowledge embedded in it (scope-to-project_id
   resolution, result filtering by min_score). It is not a pure protocol layer — it is
   a thin domain adapter that happens to use MCP as transport.
2. The FTS-fallback path exists in `mcp_tools.rs` because of a structural defect
   (BL-009) that will be fixed in Wave 2. The fallback's existence at this layer is
   temporary.

**Verdict on Option B**: acceptable violation at v0.0.1 given Wave 2 intent. The
protocol layer already has domain leakage (scope resolution). Adding audit-write here
does not materially worsen the boundary; it co-locates the leakage at the same boundary
point that Wave 2 BL-009/BL-010 will clean up.

### What the existing module structure suggests

Per CLAUDE.md "Project Structure":
- `mcp_tools.rs` — MCP tool implementations (search, ingest, invalidate)
- `search.rs` — Hybrid FTS5 + vector + RRF merge, score normalization
- `db.rs` — SQLite connection, schema, migrations

The natural location for audit-write is **neither** of the two options as stated —
it belongs in a thin audit module or as a helper that sits between the protocol layer
and the storage layer. But that is a Wave 2 shape. For Wave 1, Option B (`mcp_tools.rs`)
is the correct pragmatic placement because:
1. The data required (resolved project_id, took_ms including embedding time, result set)
   is fully assembled there and only there.
2. It co-locates with BL-009's future refactor site, so migration cost is code-move only.

### How Wave 2 BL-009/BL-010 reshapes this question

BL-009 (mcp_tools two-ingest-paths defect fix) + BL-010 (search.rs free-functions
refactor) will consolidate the FTS-fallback into the search layer. Once that happens,
a single `search::memory_search(db, query, embedding_or_none, project_id, limit)` free
function can own both the hybrid and FTS-only paths. The audit hook then has a natural
home as a wrapper:

```
search::memory_search_audited(db, query, project_id, ...) {
    // compute embedding (or skip)
    // call memory_search (handles both paths)
    // write audit row
}
```

This means Wave 1's placement in `mcp_tools.rs` is explicitly temporary — the function
that currently exists at `mcp_tools.rs:207-244` will be collapsed into `search.rs` by
Wave 2, and the audit hook moves with it. **The Wave 1 decision is a code-move decision,
not a schema decision.** Schema does not change between Wave 1 and Wave 2.

## 2. Third Option Analysis — Consolidating Fallback into Db

The framing presents the choice as binary (inside `Db::memory_search` vs inside
`mcp_tools.rs`), but option C exists: a `Db::memory_search_with_fallback` or
`Db::memory_search_audited` wrapper that internalizes the path-selection logic.

This is architecturally interesting but not the right shape for Wave 1:

1. **Embedding is not a Db concern.** The FTS-fallback fires because
   `query_embedding: Result<Vec<f32>, _>` failed. The embedding computation happens
   outside Db (in the MCP tool handler, via `spawn_blocking`). To move this into Db,
   Db would need to know about the embedding model — that violates the storage
   primitive boundary more severely than Option B.

2. **BL-009 is the correct home for this consolidation.** Moving the fallback into
   Db now would either (a) require Db to depend on the embeddings module, or (b)
   require the caller to pass `Option<Vec<f32>>` and let Db decide which path to use.
   Pattern (b) is structurally equivalent to what BL-010 will produce as a free
   function — but as a method it inherits all of Db's existing coupling.

3. **Wave 2 free-function shape is strictly better.** `search::memory_search_audited`
   as a free function over `&Db` (per 028 Topic 1 decision: free functions, not trait
   methods) avoids Db bloat and is the natural extension of BL-010's refactor scope.

**Conclusion on option C**: the right version of option C is a Wave 2 free function,
not a Wave 1 Db method. For Wave 1, accept Option B (mcp_tools.rs) as the temporary
home; Wave 2 collapses it into `search::memory_search_audited`. This is explicitly
noted in the framing's constraint: "location may move, schema does not."

## 3. Failure-Mode Architectural Implications

### Contractual shape analysis

**Best-effort**: the audit write is a Side Effect of search. The search API contract
is "return matching facts." The audit contract is "try to record that this happened."
These are separate concerns. Best-effort explicitly separates the concerns: search
succeeds independently of whether the side effect completed.

**Hard-error**: audit becomes part of the search API contract. The MCP
`memory_search` tool's contract becomes "return matching facts AND record the audit
row." This is a stronger contract — it means the operator can trust that every
successful MCP call has a corresponding audit row. But it also means audit-write
infrastructure failures (disk full, WAL stall) degrade search availability, which is
a worse failure mode for an operator.

**Transaction-coupled**: same contractual shape as hard-error from the caller's
perspective. The distinction is implementation: the search and audit-write share a
WAL-commit boundary, so there is no partial state (search results returned but audit
not written). This is only available under Option A (Topic 1) per the framing's
feasibility coupling note.

### Which shape fits mengdie's MCP API design philosophy?

From 028 conclusion: mengdie's MCP tools are operator-facing. The operator is an AI
agent (not a human at a keyboard) that uses `memory_search` to retrieve context. The
operator's recovery path for a failed `memory_search` is: retry, or proceed with
reduced context. The operator's recovery path for "search succeeded but audit failed"
is: nothing — the operator doesn't know the audit table exists.

This asymmetry is decisive: **the operator cannot act on audit-write failure
information even if it's surfaced as an error.** Hard-error or transaction-coupled
contract delivers a failure signal to a caller that has no meaningful response to it.
Best-effort is architecturally correct: the operator gets search results (the thing
it asked for), and audit-write failures are observable to the operator-developer via
`METRIC_AUDIT_WRITE_FAILURES` (not via MCP error codes).

The `record_recall` precedent at `db.rs:259-272` is the existing best-effort pattern
for exactly this reason — it records observability data without coupling retrieval
success to observability write success. F-002 should follow the same shape.

**Verdict on failure mode**: best-effort + `tracing::warn!` + `METRIC_AUDIT_WRITE_FAILURES`.
The supersession-rate signal degrades gracefully (under-counts under failure, never
wrong-direction). This matches 028's A-MEM trigger definition ("≥5 events per 30-day
window") which is a count threshold, not a completeness guarantee.

### Atomicity model per contract shape

The framing's implicit assumption to validate: "hook runs inside the same
connection-lock as the search" vs "hook runs as a separate write outside the
search lock."

Under best-effort with Option B placement (mcp_tools.rs):
- The `Arc<Mutex<Connection>>` is released after `Db::memory_search` returns.
- The audit write then acquires the lock separately.
- There is a logical gap (search returned, audit not yet written) but this is
  acceptable under best-effort semantics — the gap is sub-millisecond at single
  operator QPS and the failure case is "audit write fails" not "audit write
  partially succeeds."

Under best-effort with Option A placement (Db::memory_search, inside the lock):
- Audit write happens before the connection lock is released.
- Still best-effort (error is caught + warned, not propagated).
- No atomicity guarantee beyond "both operations happen inside the same mutex
  scope," which does not constitute a transaction.

In neither case does best-effort provide write atomicity in the SQLite WAL sense
unless an explicit `BEGIN TRANSACTION` wraps both operations. For best-effort,
this distinction is irrelevant — the contract does not promise atomicity.

**Atomicity model for best-effort**: separate writes (no explicit transaction),
sequential within the same mutex scope under Option A or across two mutex
acquisitions under Option B. Both are acceptable for best-effort semantics.

## 4. Coupling to Wave 2

The hook location decision's coupling to Wave 2 is **code-only, not schema-level**.

- Schema (`memory_search_audit`, `audit_returned_facts`, indexes) is fixed at v6
  and does not change when Wave 2 refactors the search path.
- The audit-write code will move from `mcp_tools.rs` to a Wave 2 free function
  (likely `search::memory_search_audited`) when BL-009/BL-010 ship.
- This move is a pure refactor: same SQL, same connection acquisition, same
  error-handling pattern. No behavior change visible to callers.

The one lasting structural consequence of the Wave 1 hook placement decision is:
**if the audit-write is placed inside `Db::memory_search` (Option A), it is harder
to remove or relocate in Wave 2** because it's embedded in a function that Wave 2
is moving to a free function. The Option A hook must be extracted from the `impl Db`
block and moved to the free function during Wave 2 — an extra surgical step that
Option B avoids (Option B's hook is already at the call site Wave 2 will restructure).

**Wave 2 migration cost comparison**:
- Option A (inside Db::memory_search): Wave 2 must extract the audit hook out of
  `Db::memory_search`, which itself is being moved to `search::memory_search` free
  function. Two concerns are entangled in one move.
- Option B (inside mcp_tools.rs): Wave 2 extracts both the search dispatch logic
  AND the audit hook into `search::memory_search_audited`. The concerns are already
  co-located at the refactor site.

Option B produces cleaner Wave 2 migration.

## 5. Reversibility Audit

| Placement | Reversibility cost if switched later | Dominant factor |
|-----------|--------------------------------------|-----------------|
| Option A (Db::memory_search) | Medium — audit hook is entangled with search impl; Wave 2 refactor must disentangle | Code-move cost |
| Option B (mcp_tools.rs) | Low — hook is at the top-level dispatch point; Wave 2 naturally absorbs it | Code-move cost |
| Schema changes | Low-medium — ALTER TABLE is cheap for adding columns; index changes are cheap; no behavior change to callers | Schema migration |
| Failure-mode contract change (best-effort → hard-error) | Medium — callers (AI agents) do not currently handle audit-write error signals; adding propagation requires both MCP-side error code changes and caller-side handling | Behavior change |
| Failure-mode contract change (best-effort → transaction-coupled) | High under Option B (requires moving to Option A first or introducing a transaction wrapper that crosses the mcp_tools↔Db boundary); Low under Option A | Placement coupling |

Schema reversibility dominates only if the column shape changes. All three contract
shapes share the same schema; failure-mode is a code-only decision. Schema migration
from v6 → v7 (e.g., adding a `caller_kind` column) is cheap and well-precedented.

## Summary Positions

| Question | Position | Confidence |
|----------|----------|------------|
| Topic 1 — Option A vs B | Option B (`mcp_tools.rs`) is correct for Wave 1 | HIGH |
| Topic 1 — third option (Db wrapper) | Reject for Wave 1; correct shape is Wave 2 free function | HIGH |
| Topic 2 — failure mode contract | Best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES` | HIGH |
| Topic 2 — atomicity model | Separate writes acceptable under best-effort; no explicit transaction needed | HIGH |
| Wave 2 migration cost | Option B produces cleaner Wave 2 migration than Option A | HIGH |
| Reversibility | All reversibility costs are code-move (cheap); schema is stable across placement options | HIGH |
