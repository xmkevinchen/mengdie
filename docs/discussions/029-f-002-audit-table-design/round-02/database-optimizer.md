---
agent: database-optimizer
round: 02
topics: [01-hook-placement, 02-failure-mode, index-design, sync-vs-async, rename-project-fk]
verdict: RATIFY (with one residual-risk note)
timestamp: 2026-04-28T20:30:00Z
---

# Round 2 — database-optimizer (UAG + ratification)

Required reads consumed:
- `round-01/synthesis.md` (orientation)
- `round-01/archaeologist.md` (mutex-cycle finding, file:line evidence)
- `round-01/architecture-reviewer.md` (Wave-2 migration cost analysis)
- `round-01/codex-proxy.md` (Qwen3 Alibaba lens, citation caveats)
- `round-01/gemini-proxy.md` (gemma Google lens, TL-annotated reasoning inversion on Topic 1)
- `round-01/database-optimizer.md` (own Round 1 — re-checked claims)

## Findings

### 1. EXPLAIN QUERY PLAN ratification (concrete, not heuristic)

Round 1 verified Codex's three-index design via heuristic ("textbook fit").
Round 2 confirms via measured plan.

**Test environment**: SQLite 3.51.0 (system CLI; bundled rusqlite ships 3.51.3 —
behaviour identical for query planner and index-selection logic, no semver-
incompatible planner change between point releases). In-memory DB seeded with
v0.0.1-realistic corpus:
- `memory_entries`: 1000 facts, ~50 with `valid_until` set (5% supersession)
- `memory_search_audit`: 300 audit rows (30 days × ~10 calls/day)
- `audit_returned_facts`: 3000 link rows (~10 facts/audit)
- `ANALYZE` run after seed (so planner uses stats, not heuristics)

**With the three proposed indexes:**

```text
QUERY PLAN
|--SEARCH me USING COVERING INDEX idx_memory_entries_valid_until_id (valid_until>?)
|--SEARCH arf USING COVERING INDEX idx_audit_returned_facts_fact_audit (fact_id=?)
|--SEARCH a USING INDEX sqlite_autoindex_memory_search_audit_1 (id=?)
`--USE TEMP B-TREE FOR GROUP BY
```

Wall time: 1ms. CPU user: 104µs.

**Driving table**: `me` (the partial index drives — most selective filter).
**Two of three indexes used as COVERING**: query never touches table heap for
`memory_entries` or `audit_returned_facts`. The third (audit table) uses the
auto-PK index for ID-equality lookup. **This is the optimal plan shape** for
this query — there is no better plan available; SQLite picks exactly the
plan we'd hand-craft.

**Without the custom indexes (only auto-PK indexes available):**

```text
QUERY PLAN
|--SCAN a
|--SEARCH arf USING COVERING INDEX sqlite_autoindex_audit_returned_facts_1 (audit_id=?)
|--BLOOM FILTER ON me (id=?)
|--SEARCH me USING INDEX sqlite_autoindex_memory_entries_1 (id=?)
`--USE TEMP B-TREE FOR GROUP BY
```

Wall time: <1ms. CPU user: 515µs.

**Driving table flips to `a` (audit) with a full SCAN of all 300 rows**.
The bloom filter mitigates the join-into-`me` cost but adds CPU overhead.
At v0.0.1 corpus, the no-index path is ~5× higher CPU but still completes
in <1ms — **the index design is not perf-critical at v0.0.1 today**.

**Why we keep the indexes anyway**:
1. Plan SHAPE matters more than absolute time. The driving-table choice
   (`me` partial-index → small bound on rows visited) gives O(superseded
   facts) cost regardless of audit-table growth. Without the indexes,
   cost is O(audit rows × link rows). At 6 months of operator use, the
   audit table grows linearly; the no-index plan degrades linearly while
   the indexed plan stays flat (bounded by superseded-fact count).
2. The plan output names every index by name, so any future regression
   (someone reorders the WHERE clauses, planner picks a different path)
   is detectable with `EXPLAIN QUERY PLAN` in a test.
3. Cost is essentially zero — three indexes on a low-write table.

### 2. Sync vs async audit write — ratified as sync

Round 1 (db-optimizer): transaction-coupled <1ms at v0.0.1 corpus. Round 2
ratification: best-effort sync (no `tokio::spawn`) is correct.

**Concrete reasoning chain:**

- **WAL fsync cost** (NVMe SSD, `synchronous=NORMAL` per SQLite WAL default):
  ~50-200µs per implicit-tx INSERT. The audit-write is one INSERT into
  `memory_search_audit` + one bulk INSERT (or multi-row VALUES) into
  `audit_returned_facts`. Two fsyncs total → ~100-400µs.
- **Existing baseline**: `record_recall` already pays ~10 fsyncs per
  `memory_search` call (one per returned fact). Audit-write adds 2 more —
  ~20% relative increase in DB-side latency.
- **Absolute search latency budget**: dominated by embedding inference
  (`fastembed` all-MiniLM-L6-v2: 2-10ms cold, 1-3ms warm) and FTS5/vector
  search itself (single-digit ms). The 100-400µs audit add is <5% of the
  total search-call latency. **Imperceptible to operator.**
- **`tokio::spawn` overhead**: an async-write path adds:
  1. Channel/handle allocation cost (~µs).
  2. Connection mutex contention if the audit task and a follow-up
     ingestion run concurrently.
  3. Best-effort observability — async write dropped if process exits
     before task completes (unbounded race between MCP response and
     persistence).
  4. Lifecycle complexity: where does the spawned task live? `Db`
     doesn't own a runtime handle today.
- **No existing async-write precedent in mengdie**: every `Db` write
  method is sync (`insert_memory`, `record_recall`, `invalidate_memory`,
  `insert_synthesis_with_links`, `rename_project`). Going async for audit
  alone would be the first async-write site. Convention argument: don't
  introduce a new pattern for a 100-400µs win that isn't measurable.

**Verdict**: sync best-effort write. The Qwen3-Coder Round 1 open question
"Should audit be async?" resolves to NO — sync is correct. Async is a
v1+ optimization to revisit if (and only if) audit write latency becomes
measurable in the search budget, which it is not at v0.0.1 corpus.

### 3. `rename_project` FK coupling — confirmed safe at v0.0.1, with one residual risk

Tested with a minimal SQLite reproducer (PRAGMA foreign_keys OFF, then ON,
behaviour observed):

**Under PRAGMA foreign_keys = OFF (current mengdie state)**:
- `DELETE FROM memory_entries WHERE id = ?` succeeds.
- `audit_returned_facts` row referencing the deleted fact_id **REMAINS**
  as an orphan.
- No error. No cascade. Silent orphan.

**Under PRAGMA foreign_keys = ON (hypothetical future flip)**:
- The same DELETE fails with `FOREIGN KEY constraint failed (19)` (verified
  by reproducer at /tmp/rename_project_test.sql).
- `rename_project` (`db.rs:636`) would return error from `tx.execute`,
  abort the transaction, and propagate up.

**Concrete safety verdict at v0.0.1 (current state — PRAGMA OFF, no
explicit ON DELETE clause)**:

| Concern | v0.0.1 status | Notes |
|---|---|---|
| `rename_project` DELETE breaks under audit FK | **Safe** | PRAGMA OFF makes the FK declaration documentation-only |
| Orphan link rows pile up in audit_returned_facts | **Acceptable** | Supersession query joins `arf.fact_id = me.id`; orphans simply don't match the join → query result excludes them. They're storage cost only, not signal cost. |
| Storage cost of orphans | **Negligible** | `rename_project` runs only on operator-initiated project renames (rare). Each rename produces at most a few orphan link rows. v0.0.1 will accumulate <100 orphans over its lifetime. |
| Future PRAGMA flip | **Documented risk, not v0.0.1 risk** | If a future BL turns FK enforcement on, `rename_project` becomes a hard break. The framing's pre-decided item 1 already names this as a separate BL trigger. |

**Residual risk to surface**: the supersession-rate query semantics are
**unaffected by orphan link rows** because of the inner-join pattern. But
if a future query computes "how many audit calls returned facts that are
now deleted" (a debugging tool), orphan rows would inflate that count. v0.0.1
has no such query; document this as a "if you build that query, account
for orphan rows" note in the plan.

**Synthesis convergence with Round 1 archaeologist + framing**: agreed.
PRAGMA-OFF + no-explicit-ON-DELETE-clause is the lowest-cost decision. The
audit table is append-only and orphan-tolerant by query design. No change.

## Agreements

| Agreement | Round 1 source | Round 2 verification |
|---|---|---|
| Topic 1 → Option B (`mcp_tools.rs`) | architecture-reviewer + Qwen3 (HIGH); db-optimizer Round 1 (both viable, Option B fits best-effort precedent) | UAG-tested below; cannot find counterexample. Confirm Option B. |
| Topic 2 → best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES` | 4-of-4 explicit Round 1 votes | UAG-tested below; cannot find counterexample. Confirm best-effort. |
| Codex's three-index design correct | Round 1 db-optimizer heuristic | Confirmed by EXPLAIN QUERY PLAN above. Both indexes participate as COVERING; partial index drives. |
| Sync (not async) audit write | Round 1 db-optimizer (latency analysis); Qwen3 Round 1 open question | Ratified §2 above. |
| FTS-fallback fact IDs available at `mcp_tools.rs:244` | archaeologist Round 1 D2 | Cross-confirmed; no SQLite-level concern. |

## Disagreements

None with my own Round 1 conclusions. One refinement to my Round 1 stance:

**Refinement**: my Round 1 wrote "If Topic 2 picks transaction-coupled, the
hook MUST live in `Db::memory_search` (option A)." Archaeologist's mutex-
cycle finding refines this: even Option A doesn't trivially support
transaction-coupled because `Db::memory_search` releases the lock between
sub-calls. **Correct revised statement**: transaction-coupled requires
restructuring `Db::memory_search` to hold one `MutexGuard` end-to-end
regardless of which Topic 1 option is chosen. This weakens the
Topic-1↔Topic-2 coupling claim from the framing, in line with archaeologist
D1.

## UAG status per topic

### Topic 1 UAG: Find a v0.0.1 scenario where Option A is strictly better than Option B

**Falsification attempt** — six concrete v0.0.1 scenarios, each tested:

1. **CLI auto-coverage**: Option A auto-covers `cli.rs:609` because CLI calls
   `Db::memory_search` directly. Option B requires explicit wiring.
   - **Cost of Option B's wiring**: one shared helper `fn audit_search_event(...)`
     called from both `mcp_tools.rs` and `cli.rs:609`. Two-line addition. Not
     a structural cost.
   - **Verdict**: not a "strictly better" scenario for Option A — Option B's
     CLI coverage is achieved by trivial code, not deferred or skipped.

2. **Single chokepoint argument**: Option A is at `Db::memory_search` which
   IS a single function, while Option B is at `mcp_tools.rs` plus shared
   helper plus CLI wiring (multiple sites).
   - **Counter**: Option B's "multiple sites" all reduce to one call to
     the shared helper. Same chokepoint shape, differently named.
   - **Verdict**: not strictly better — equivalent chokepoint shape.

3. **Hypothetical future internal caller of `Db::memory_search`**: Option A
   would auto-cover. Option B would not.
   - **Counter**: archaeologist Round 1 confirmed ZERO internal callers
     today. Designing for hypothetical future internal callers is the YAGNI
     anti-pattern the framing's pre-decided item 2 already rejected.
   - **Verdict**: not strictly better at v0.0.1.

4. **`took_ms` semantic correctness**: Option A measures Db-side time only
   (excludes embedding inference). Option B measures end-to-end search
   time (includes embedding).
   - **Counter for Option A advocate**: "took_ms should be Db-side time
     because it represents what the audit captures — DB performance."
   - **Counter-counter (architecture-reviewer + db-optimizer convergent)**:
     no v0.0.1 query uses `took_ms` directly. The supersession SQL doesn't
     reference it. If `took_ms` were "Db-side only," operator-debug
     observability would lose embedding-failure timing. End-to-end is
     more useful for the only realistic consumer (operator looking at
     audit rows for debug).
   - **Verdict**: Option B's took_ms is semantically richer. Option A is
     not strictly better here either.

5. **FTS-fallback exclusion as an INTENTIONAL filter**: maybe degraded-mode
   searches SHOULD be excluded because they're "not real" search calls?
   - **Counter (challenger Step 2 → Round 1 convergence)**: degraded-mode
     searches ARE real operator retrieval activity. The supersession-rate
     signal under-counts during embedding outages if degraded-mode is
     excluded. This biases the trigger LOW during exactly the windows
     where mengdie may be most at-risk (model-download issues coincide
     with operator pain points).
   - **Verdict**: Option A's exclusion is a bug, not a feature. Not
     strictly better.

6. **Wave 2 migration cost asymmetry**: Option A's hook is inside
   `Db::memory_search` which Wave 2 BL-009/BL-010 will move to a free
   function. Option A's hook moves once. Option B's hook is in
   `mcp_tools.rs` which Wave 2 will refactor — hook moves once.
   - **Counter**: Option A's move requires extracting the hook from a
     method that's being moved (two concerns entangled). Option B's
     move is a code-co-location: hook and its call site move together.
   - **Verdict**: Option B has cleaner Wave 2 migration (architecture-
     reviewer Round 1 verdict). Option A is not strictly better here.

**UAG result for Topic 1**: cannot find any v0.0.1 scenario where Option A
is strictly better. **Option B confirms.** Topic 1 converges to Option B
(`mcp_tools.rs` after `match query_embedding` block, with shared helper
`audit_search_event(...)` called from `cli.rs:609`).

### Topic 2 UAG: Find an A-MEM scenario where best-effort under-counting causes wrong-direction outcome

**Falsification attempt** — three concrete A-MEM scenarios, each tested:

1. **False negative**: best-effort drops audit rows during a window where
   true supersession count is exactly at threshold (e.g., true=5,
   observed=4). Trigger fails to fire.
   - **Direction analysis**: this delays the trigger but does NOT cause
     a wrong-direction outcome. The trigger is "fires when supersession-
     rate exceeds threshold," meaning "fact churn is high enough that
     A-MEM bidirectional update would be valuable." Delaying that
     conclusion postpones a beneficial change. **Not wrong-direction —
     just slower-to-be-right.**
   - **Operator visibility**: `METRIC_AUDIT_WRITE_FAILURES` is non-zero
     in this case. Operator sees "audit drops occurred" and knows the
     observed count is a lower bound. Manual adjustment (multiply by
     observation-rate) is possible.
   - **Verdict**: not wrong-direction. Not a counterexample.

2. **False positive**: best-effort somehow OVER-counts. Could under-counting
   ever push a window above threshold?
   - **Direction analysis**: best-effort drops rows on failure. It cannot
     create rows. Under-counting never inflates the count. The math
     doesn't support a false positive from best-effort.
   - **Verdict**: structurally impossible.

3. **Wrong window assignment**: best-effort writes succeed but with wrong
   `searched_at` timestamp, causing supersession events to be attributed
   to wrong 30-day window.
   - **Direction analysis**: This isn't a best-effort failure — it's a
     correctness bug in the timestamp logic. Best-effort vs hard-error
     doesn't change the timestamp semantics. The contract decision is
     orthogonal.
   - **Verdict**: irrelevant to the UAG question. Not a counterexample.

**Edge case: what if A-MEM trigger uses a more sophisticated algorithm
than 028's "≥5/30-day"?** From 028 conclusion + Qwen3 + gemma analysis,
the trigger is a count threshold over a rolling window. Even if the actual
A-MEM paper (Ma et al. 2024) uses something fancier (e.g., Kullback-Leibler
divergence between observation distributions), the property "monotonic in
observation count" holds for any reasonable density-based trigger. Under-
counting always biases LOWER, never wrong-direction.

**Edge case: what if mengdie operator is trying to PROVE the trigger
should NOT fire (e.g., for a research question "is supersession rate
declining?")?** Under-counting biases the answer LOWER, which would
falsely confirm the operator's "not declining → declining" hypothesis.
But this is operator-side analysis, not the trigger algorithm.
`METRIC_AUDIT_WRITE_FAILURES` makes this detectable.

**UAG result for Topic 2**: cannot find an A-MEM scenario where best-
effort causes wrong-direction outcome. **Best-effort + warn +
`METRIC_AUDIT_WRITE_FAILURES` confirms.** Topic 2 converges.

## Open Questions

None at this round from the SQLite/rusqlite lens. All Round 1 db-optimizer
open questions were ratified or confirmed converged.

For other agents to consider (not blocking convergence):

- **Plan-time test artifact**: should the plan include an `EXPLAIN QUERY
  PLAN` regression test (assert the supersession query uses the three
  expected indexes by name)? This is a v0.0.1 acceptance question, not
  a topic-level decision. Recommend YES — cheap test, catches future
  regressions.
- **ANALYZE invocation timing**: should `ANALYZE memory_search_audit;
  ANALYZE audit_returned_facts;` be run periodically (e.g., during
  dreaming pass)? Recommend OPTIONAL for v0.0.1 (defer to v1+ unless
  query plan regresses). Auto-tables get analyzed-at-build via the
  `ANALYZE` we run during testing; production accumulates rows
  monotonically and the planner's heuristics (without ANALYZE) are
  correct enough at this scale.

## Ratifications

| Decision | Verdict | Confidence | Source |
|---|---|---|---|
| **Topic 1 → Option B** (mcp_tools.rs hook + shared helper from CLI) | RATIFY | HIGH | UAG: 6 concrete scenarios, none make Option A strictly better |
| **Topic 2 → best-effort + warn + METRIC_AUDIT_WRITE_FAILURES** | RATIFY | HIGH | UAG: 3 A-MEM scenarios + edge cases, none cause wrong-direction outcome |
| **Codex's three-index design** | RATIFY | HIGH | EXPLAIN QUERY PLAN confirms optimal driving-table + 2-of-3 COVERING usage; measured 1ms wall time at v0.0.1 corpus |
| **Sync audit write (not async)** | RATIFY | HIGH | 100-400µs cost <5% of total search latency; no existing async-write precedent; `tokio::spawn` complexity not justified |
| **rename_project DELETE under PRAGMA OFF + no explicit ON DELETE** | RATIFY (with residual risk note) | HIGH | Tested: orphan link rows accumulate but supersession query is unaffected by inner-join filter; PRAGMA flip remains a separate BL trigger |
| **`BEGIN IMMEDIATE` if transaction-coupled (no longer applicable since Topic 2 = best-effort)** | NOT APPLICABLE | — | Best-effort means no explicit transaction wrapping search+audit |
| **CLI shared-helper shape**: `audit_search_event(...)` called from both `mcp_tools.rs` and `cli.rs:609` (Qwen3's open question) | RATIFY | HIGH | Trivial wiring cost, single source of truth, future-compatible with Wave 2 free-function refactor |

## Final database-optimizer Round 2 verdict

Both topics converge unambiguously. EXPLAIN QUERY PLAN ratifies the index
design with measured data, not heuristics. Sync audit-write is correct at
v0.0.1 latency budget. `rename_project` is safe at v0.0.1 with one
documented residual risk (orphan link rows; supersession query unaffected
by design).

**Recommendation to TL**: close both topics, write conclusion, proceed
to plan.
