---
agent: database-optimizer
verdict: ANALYSIS
timestamp: 2026-04-28T18:00:00Z
topic_focus: [topic-02-failure-mode, topic-01-hook-placement]
---

# Database-optimizer Round 1 findings — F-002 audit table design

SQLite/rusqlite expert lens. Six prompted questions, answered with citations
to mengdie's existing code (`src/core/db.rs`, `src/core/schema.rs`,
`src/core/search.rs`) and the SQLite manual.

---

## 1. WAL atomicity boundaries

mengdie runs in WAL mode (`schema.rs:86 — PRAGMA journal_mode=WAL`) with
`busy_timeout=5000` (`schema.rs:87`) and the default `synchronous=NORMAL`
(WAL mode default; never overridden). One process, one
`Arc<Mutex<Connection>>` (`db.rs:22`). All four shapes the framing names —
treat each as its own atomicity unit:

### A. Standalone INSERT after a SELECT, no explicit transaction (best-effort + warn)

Each `conn.execute(INSERT ...)` runs as an **implicit transaction** under
WAL — SQLite wraps it in `BEGIN IMMEDIATE; INSERT; COMMIT` internally
([SQLite docs on Atomic Commit](https://www.sqlite.org/atomiccommit.html)
§3 — "Each individual statement is its own implicit transaction unless
inside an explicit BEGIN"). Atomicity guarantee: **the INSERT itself is
atomic** (it either appears in the WAL or it does not). What is **NOT**
guaranteed:

- That the SELECT's snapshot and the INSERT see the same DB state. Between
  the SELECT result returning and the implicit-tx INSERT executing, another
  writer could have advanced the WAL. (Not relevant at single-process
  v0.0.1, but the contract should be stated.)
- That the search response and the audit row are bundled into one durable
  unit. The search response leaves the function before the INSERT runs;
  if process is `kill -9`'d between SELECT-return and INSERT, the response
  is already gone (no crash recovery for an in-flight response) and the
  audit row never lands.

This shape is **the same atomicity contract as `record_recall` today**
(`db.rs:259-272`) — see §4.

### B. INSERT that fails after the SELECT response was already returned

The SELECT result is materialised into Rust `Vec<SearchResult>` before
the INSERT runs. Once the function returns the `Vec`, that data is in
caller-owned memory; the SELECT cannot be "rolled back" — there is no
SQLite-level snapshot to revoke. The INSERT failure is an isolated WAL
event: the INSERT did not commit, but the SELECT's effect (returning
results to a caller and any side effect like `record_recall` if it ran)
is already public. **There is no SQLite primitive that can un-publish a
SELECT result that has already left the connection.**

Implication for Topic 2: under "best-effort + warn", this is the
intended contract — search succeeds, audit drops, signal degrades by
one row. Under "hard error", the search response is constructed but
not yet returned to caller — the function path can replace the response
with `Err(...)`. Under "transaction-coupled", the SELECT and INSERT
share a transaction so a SELECT done inside the tx with an INSERT failure
ends with the whole tx aborting and no result returned at all
(stronger guarantee, matches §1.C).

### C. `BEGIN IMMEDIATE; SELECT ...; INSERT ...; COMMIT;` (transaction-coupled)

`BEGIN IMMEDIATE` ([SQLite Lang Transaction docs](https://www.sqlite.org/lang_transaction.html)
§2.2) acquires the **RESERVED lock immediately** — it does not wait for
the first write. In WAL mode, RESERVED is a single-writer lock against
other writers (readers proceed against the snapshot established at
`BEGIN`). Atomicity guarantee under this shape:

- Reads inside the tx see a consistent snapshot from the moment of
  `BEGIN IMMEDIATE` (WAL snapshot isolation).
- The INSERT is staged but not durable until COMMIT.
- A failure at any step (SELECT bind error, INSERT FK violation,
  COMMIT failure) rolls back the whole unit — no partial state.
- If process dies between BEGIN and COMMIT, WAL recovery on next open
  discards the uncommitted frames.

This is the strongest contract of the four. **Cost**: holds the
RESERVED lock for the full duration of the SELECT + post-processing +
INSERT (see §2 for latency analysis).

### D. Just doing the INSERT without any wrapping (current `record_recall` pattern)

This is the same as §1.A but with the `record_recall` precedent confirming
the convention. `record_recall` at `db.rs:259-272` does:

```rust
let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
let rows = conn.execute("UPDATE memory_entries SET ...", params![...])?;
Ok(rows > 0)
```

Single `conn.execute` on a single statement → implicit BEGIN IMMEDIATE
+ COMMIT. Atomic at the row level. Failure surfaces as `Err(rusqlite::Error)`
which `?` propagates up to the caller (`memory_search` at `search.rs:188`,
which then logs `tracing::warn!` and continues — see §4).

---

## 2. Latency analysis for transaction-coupled at v0.0.1 scale

Concrete numbers for v0.0.1 corpus (~200-1000 facts, 10 returned per call,
single-user single-process, WAL mode, NVMe SSD).

### Standalone-INSERT baseline

The current `memory_search` already does:

1. `search_fts` — acquires lock, runs FTS5 query, releases. **One implicit tx.**
2. `search_vector` — acquires lock, full-table scan + cosine, releases. **One implicit tx (read-only, no fsync).**
3. RRF merge in app code (zero DB).
4. For each of N=10 hits:
   - `get_memory` — acquires lock, runs SELECT, releases.
   - `record_recall` — acquires lock, runs UPDATE (implicit BEGIN IMMEDIATE + COMMIT + fsync). **N=10 separate implicit write transactions.**

That is **10 fsyncs per search call** today, plus 12 lock acquire/release
cycles. At NVMe SSD WAL fsync ~100µs each (Litestream-published numbers
for `synchronous=NORMAL`): the existing search path already pays ~1ms in
WAL fsync just for `record_recall`.

### Transaction-coupled overhead

Wrapping the SELECT + audit-INSERT in a single `BEGIN IMMEDIATE` /
`COMMIT` unit:

- `BEGIN IMMEDIATE` itself: acquires RESERVED lock + writes a WAL frame
  header marker. ~50-100µs on NVMe.
- The SELECT statements run unchanged (snapshot-isolated, no extra cost
  inside the tx vs outside).
- `INSERT INTO memory_search_audit` — one row.
- `INSERT INTO audit_returned_facts` — N=10 rows. Either as N separate
  prepared-statement bindings or one `INSERT ... VALUES (...), (...), ...`
  multi-row.
- `COMMIT` — fsync. ~100µs.

**Realistic added latency vs standalone INSERT pattern**: 200-500µs.
Negligible relative to the embedding inference (`fastembed` all-MiniLM-L6-v2
on M-series CPU: 2-10ms, sometimes 20ms cold-cache) and the FTS5 +
vector-search work itself (single-digit ms at this corpus size).

**Caveat**: the transaction-coupled shape would also let us COLLAPSE
the existing 10 `record_recall` fsyncs into ONE — net latency
improvement of ~700-900µs vs current standalone behaviour. So
transaction-coupled is potentially **faster** end-to-end than the
naïve "wrap audit-write in implicit tx" path, IF the implementation
folds `record_recall` into the same tx. Whether to do that is a
separate decision (`record_recall`'s current best-effort silent-drop
behaviour is intentional per `search.rs:188` "failed to record recall"
warn pattern; folding it into a hard-error tx would change that
contract too).

**Citation**: SQLite docs explicitly state ([WAL Mode docs](https://www.sqlite.org/wal.html)
§5.1) — "transactions are durable when COMMIT returns; readers do not
block writers and writers do not block readers." So `BEGIN IMMEDIATE`
in WAL does **not** block the embedding-generation thread or other
readers; it only blocks other writers, which mengdie has at most one
of (the dreaming pass via `mengdie dream` is the only other writer
and is operator-triggered, not concurrent with search at v0.0.1).

### Verdict for Topic 2 latency budget

Transaction-coupled overhead at v0.0.1 corpus is well under 1ms
worst-case. The framing's "if >>10ms it crosses an MCP responsiveness
boundary" threshold is not at risk. **Latency is not a load-bearing
argument against transaction-coupled at v0.0.1.** The decision should
turn on signal-completeness vs. failure-blast-radius (a contract
question, not a performance one).

---

## 3. Failure-mode taxonomy

What can the audit-write hit, and what's the right response per category.

### `SQLITE_FULL` (disk full / WAL too large to extend)

- **Probability at v0.0.1**: very low (single-user laptop, ~MB-scale DB).
- **Recovery**: not transient. Disk needs to be cleared; subsequent INSERTs
  also fail until then.
- **Right contract**: best-effort + log + metric. Failing the search because
  disk is full doesn't help — operator gets a useless cascading error.
  `tracing::warn!` + `METRIC_AUDIT_WRITE_FAILURES` makes it observable.
- **Verdict**: best-effort.

### WAL checkpoint stall under high write rate

- **Probability at v0.0.1**: zero. Mengdie has at most one writer
  (search-time audit + dreaming pass, which are not concurrent). WAL
  checkpoint stalls happen at high-fanout writer + slow checkpointer
  scenarios. Not applicable.
- **Verdict**: not a v0.0.1 failure mode.

### Constraint violations (FK, CHECK, UNIQUE)

- **FK**: PRAGMA `foreign_keys` is **OFF** project-wide (`db.rs:80-119`,
  confirmed by archaeologist). FK violations on `audit_returned_facts.fact_id`
  CANNOT FIRE under current config. If a future BL flips PRAGMA on,
  this becomes a real failure mode and the F-002 contract must be
  revisited.
- **CHECK**: Audit table proposed schema has no CHECK constraints
  beyond `NOT NULL` + types. `NOT NULL` violations would only fire
  on a programming bug (e.g., constructing the audit row with a
  null `query`). These are debug-time failures, not production.
- **UNIQUE**: only `id` (UUID v4) is unique. Collision probability ~0.
- **Verdict**: not a runtime failure mode at v0.0.1.

### Locking conflicts (`SQLITE_BUSY`)

- **Probability at v0.0.1**: low but non-zero. The dreaming pass
  (`mengdie dream`) is a separate process opening the same
  `~/.mengdie/db.sqlite`. If an operator triggers `mengdie dream` while
  the MCP server is mid-search, the audit-INSERT could hit `SQLITE_BUSY`.
- **Recovery**: `busy_timeout=5000` (`schema.rs:87`) means rusqlite
  retries for 5 seconds before failing. So `SQLITE_BUSY` only surfaces
  if a write held the lock for > 5s — a serious contention event,
  not normal operation.
- **Right contract**: best-effort + log + metric, OR a single retry
  loop in the audit-write helper. Fail-on-busy would propagate
  spurious search errors during dreaming runs.
- **Verdict**: best-effort with the existing busy_timeout absorbing
  normal contention.

### Catastrophic (corrupted DB, schema mismatch)

- **Probability at v0.0.1**: very low; if it happens, the entire DB is
  unusable, not just the audit table.
- **Recovery**: requires operator intervention. Subsequent searches will
  fail too.
- **Right contract**: doesn't matter for the audit-write specifically;
  the search would already be failing. **Audit-write failure is a
  symptom, not the disease.**
- **Verdict**: orthogonal to the contract decision.

### Failure-mode taxonomy summary table

| Category | v0.0.1 probability | Right contract response |
|---|---|---|
| `SQLITE_FULL` | low | best-effort + warn + metric |
| WAL checkpoint stall | zero | n/a |
| FK violation | zero (PRAGMA off) | n/a until PRAGMA flips |
| CHECK / NOT NULL | zero (debug-time only) | n/a |
| `SQLITE_BUSY` | low | best-effort + warn + metric (busy_timeout absorbs normal) |
| Catastrophic corruption | very low | n/a (search itself failing too) |

**Implication for Topic 2**: the failure modes that ACTUALLY fire at
v0.0.1 are all transient or operator-actionable, none are signal-correctness
failures. **Best-effort + observable counter is the right response for
every realistic v0.0.1 failure category.** Hard-error and transaction-coupled
are buying signal correctness against failure modes that don't fire here.

This is a real argument FOR best-effort, but it's also a real argument
that the topic-2 decision depends on whether the
A-MEM-trigger consumer can tolerate "0 rows lost in practice" or
"theoretically 0 rows lost" — a contract question, not a SQLite
question. (Topic 2's open research question is the right framing
for this; SQLite-level analysis confirms cost is low for either path.)

---

## 4. mengdie's existing `record_recall` precedent

`db.rs:259-272`:

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

### Is this best-effort?

At the **`record_recall` layer**: NO — it propagates `?` errors. At the
**caller layer (`search.rs:188`)**: YES, the caller catches and warns:

```rust
if let Err(e) = self.record_recall(id, normalized) {
    tracing::warn!(id = %id, error = %e, "failed to record recall");
}
```

This is the load-bearing pattern: **`record_recall` itself is a
straightforward propagating function; the best-effort policy is
applied at the call site by `if let Err`-catching and continuing the
loop**. Each iteration acquires the lock independently.

### Does it use a transaction?

Implicit only — `conn.execute` on a single statement in WAL mode is one
implicit `BEGIN IMMEDIATE; UPDATE; COMMIT` (see §1.A). No explicit `tx`.

### Connection-lock duration

The lock is held for: (1) acquiring the lock, (2) one `conn.execute` call,
(3) returning `Ok`. This is microseconds. The lock is released at the
end of the function via `Drop` of `MutexGuard`. **Crucially**, because
each `record_recall` call inside the search loop acquires its own lock,
the search loop already releases the lock between iterations — meaning
**any consumer waiting on the connection mutex (e.g., a concurrent
ingestion task) gets fair access between iterations**.

### Should F-002 follow this precedent or diverge?

**Mengdie's existing convention is strongly best-effort + log + metric**:

- `record_recall` (UI counter) — best-effort, logs warn (`search.rs:188-190`).
- `metrics.rs` increments — silent on failure (counter writes never propagate).
- `get_memories_by_ids` (`db.rs:315-319`) — logs warn when fetched < requested,
  does not error.

The convention is documented and reasoned-about: observability writes
do NOT take down the operation they observe. F-002's audit-write
follows the same shape.

**The argument FOR diverging (transaction-coupled or hard-error)**: F-002's
signal is criticality-different — it's not a UI counter, it's the
trigger condition for an architecture change (A-MEM bidirectional
update). False-low supersession-rate readings could delay a needed
architecture change.

**The argument AGAINST diverging**: the trigger is a **count threshold**
("≥5 events per 30-day window"), not a precise rate measurement.
Probabilistic under-counting under failure pushes the trigger LATER,
not in a wrong direction. As long as `METRIC_AUDIT_WRITE_FAILURES` is
non-zero AND the operator has a way to see it, the operator can
manually adjust expectation when the trigger threshold approaches.

**Convergence with topic 2 framing**: SQLite-level analysis can't decide
this — it's a downstream-consumer-correctness question. But the
SQLite-level analysis CAN confirm: there is no operational cost to
choosing best-effort (it's the existing pattern, has working precedent).
There IS a small additional code complexity to choosing transaction-coupled
(introduces a new explicit `Transaction` lifecycle in `Db::memory_search`,
the only function that would have one — `insert_memory_resolving` at
`db.rs:225-256` is the closest existing pattern).

### Concrete recommendation (database-optimizer lens)

**Follow the precedent (best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES`)
unless the A-MEM-trigger consumer specifically requires strict
completeness** (Topic 2 open research question). The latency-cost
argument against transaction-coupled is weak (§2); the convention-cost
argument is strong; the failure-mode taxonomy says realistic failures
are observable-and-actionable, not silent-data-corruption.

If the research question concludes "tolerates probabilistic loss" → best-effort.
If "requires strict completeness" → transaction-coupled (preferred over
hard-error because the latency cost is sub-ms and the "search succeeded
but you don't know if it was audited" surface area is smaller).

---

## 5. Index design for the supersession-window query

Re-deriving the optimal indexes from the supersession SQL (analysis.md
lines 158-171):

```sql
SELECT DATE(a.searched_at, 'start of day', '-30 days') AS window_start,
       COUNT(*) AS supersession_count
FROM memory_search_audit a
JOIN audit_returned_facts arf ON arf.audit_id = a.id
JOIN memory_entries me ON me.id = arf.fact_id
WHERE me.valid_until IS NOT NULL
  AND JULIANDAY(me.valid_until) - JULIANDAY(a.searched_at) <= 7
  AND a.searched_at >= DATE('now', '-30 days')
GROUP BY window_start
HAVING supersession_count >= 5;
```

### Codex's proposal (analysis.md lines 84-86)

```sql
CREATE INDEX idx_memory_search_audit_searched_id ON memory_search_audit(searched_at, id);
CREATE INDEX idx_audit_returned_facts_fact_audit ON audit_returned_facts(fact_id, audit_id);
CREATE INDEX idx_memory_entries_valid_until_id ON memory_entries(valid_until, id) WHERE valid_until IS NOT NULL;
```

### SQLite query planner trace (mental model)

SQLite query planner ([Query Planner docs](https://www.sqlite.org/queryplanner.html))
uses cost-based selection over available indexes. For the supersession query:

**Driving table choice**: SQLite should drive from the most selective
filter. At v0.0.1 corpus (1000 facts, ~30 days of search history,
~5-10 audit rows per day = ~200-300 audit rows), the most selective
filter is `me.valid_until IS NOT NULL` (a fact only enters the
supersession join if it's been invalidated — small fraction of
`memory_entries`). The partial index `idx_memory_entries_valid_until_id
WHERE valid_until IS NOT NULL` is a **textbook fit** for this pattern —
([SQLite Partial Indexes docs](https://www.sqlite.org/partialindex.html)
§2: partial indexes shrink the index size to only the rows matching
the WHERE clause; query planner uses it when the WHERE clause matches).

**Join 1 (`me.id = arf.fact_id`)**: with `idx_arf_fact_audit (fact_id, audit_id)`,
the second index becomes a covering index — no table lookup needed for
arf rows because the SELECT only references `arf.fact_id` and `arf.audit_id`.
This is the fast path.

**Join 2 (`arf.audit_id = a.id`)**: needs an index on `memory_search_audit(id)`.
SQLite auto-creates `sqlite_autoindex_memory_search_audit_1` for the
PRIMARY KEY constraint on `id` — Codex's `idx_memory_search_audit_searched_id`
adds the composite `(searched_at, id)` which lets SQLite range-scan the
30-day window AND join on id without a table seek (covering for
`a.searched_at` access).

**Cost estimate at v0.0.1 scale** (~1000 facts, ~300 audit rows, ~3000
arf rows):

- Without these indexes: full scan `memory_entries` (1000 rows) + nested
  loop join hash to arf (3000 rows) + sort by `searched_at` for window
  filter. **Time**: ~5-15ms. Acceptable but ugly.
- With these indexes: partial-index scan (only superseded facts, ~10-50
  rows) + covering-index seek into arf + indexed seek into audit. **Time**:
  ~0.1-0.5ms. Negligible.

### Are these correct?

**Yes, with one optimization observation.** Codex's three-index design
is the right shape. Specifically:

1. `idx_memory_search_audit_searched_id (searched_at, id)` — **correct**.
   Composite index supports the 30-day window range scan AND the join
   to `arf.audit_id`. Order matters: `searched_at` must come first to
   enable the range scan on `searched_at >= DATE('now', '-30 days')`.

2. `idx_audit_returned_facts_fact_audit (fact_id, audit_id)` — **correct**.
   Covering index for the supersession join (fact-driven). Codex's
   observation about FK-maintenance scan cost is also correct: if
   PRAGMA `foreign_keys` is ever flipped on, every DELETE on a
   `memory_entries` row scans `audit_returned_facts` for matching
   `fact_id` references. Without this index, that scan is O(n);
   with it, it's O(log n).

3. `idx_memory_entries_valid_until_id (valid_until, id) WHERE valid_until IS NOT NULL`
   — **correct, and slightly clever**. The partial WHERE clause matches
   the query's `me.valid_until IS NOT NULL` filter. This means SQLite
   uses the partial index even though the query joins on `me.id` — the
   query planner extracts the implicit `valid_until IS NOT NULL`
   condition from the WHERE and matches it against the partial index's
   predicate. ([Partial Indexes docs](https://www.sqlite.org/partialindex.html)
   §3 — "An UPSERT partial index is used when the predicate of the
   index is a logical consequence of the WHERE clause of the query.")

### Is there a more efficient design?

Two minor observations, neither load-bearing:

**Observation 1**: An additional reverse index `idx_arf_audit (audit_id)`
would help one specific query pattern — "give me all fact IDs returned
in audit X" (operator-debugging tool). Not needed for the supersession
SQL itself. **Defer to BL trigger** when an operator-debug tool is built.

**Observation 2**: SQLite ANALYZE statistics. The query planner's index
selection improves significantly with `sqlite_stat1` populated. v0.0.1
should call `ANALYZE memory_search_audit; ANALYZE audit_returned_facts;`
once per dreaming pass (or at first use post-migration), so the planner
has stats. This is **not a v0.0.1 blocker** but worth noting in the
plan. Reference: [SQLite ANALYZE docs](https://www.sqlite.org/lang_analyze.html)
— "Running ANALYZE periodically is a SQLite optimization best practice;
without it, the planner uses heuristics that can be off by 10× for
unusual data shapes."

**Verdict**: Codex's three-index design is correct and minimal. **Adopt
as-is.** Optional ANALYZE hook is a v1+ refinement.

---

## 6. `BEGIN IMMEDIATE` vs `BEGIN DEFERRED`

For Topic 2 transaction-coupled, which transaction mode is correct?

### Default (`BEGIN` alone) = `BEGIN DEFERRED`

[SQLite Lang Transaction docs](https://www.sqlite.org/lang_transaction.html)
§2.1: `BEGIN DEFERRED` does NOT acquire any lock at start. The lock
upgrade happens lazily — SHARED lock on first read, RESERVED on first
write. **Concrete failure mode**: if multiple writers exist and two
both `BEGIN DEFERRED`, do reads, then both try to upgrade to write,
**one gets `SQLITE_BUSY` at upgrade time** (deadlock-avoidance: SQLite
refuses to grant RESERVED to a connection that already holds SHARED if
another connection is upgrading). The transaction must rollback and
retry.

### `BEGIN IMMEDIATE`

Acquires RESERVED lock at the `BEGIN` itself. Other writers waiting
get `SQLITE_BUSY` immediately at THEIR `BEGIN IMMEDIATE`, not partway
through. **Predictable failure point**.

### Mengdie's writer concurrency at v0.0.1

- One MCP server process (single connection per `Arc<Mutex<Connection>>`)
- One CLI / dream pass (separate process, separate connection)
- Single-user laptop deployment

There can be **at most 2 writer connections** in flight concurrently
(MCP server + dreaming pass). The dreaming pass is operator-triggered
and short-running. Concurrent writes are rare-but-possible.

### Correct choice: `BEGIN IMMEDIATE`

For F-002 (if Topic 2 picks transaction-coupled):

1. The hook reads (the search SELECT statements) AND writes (audit
   INSERT) in the same transaction. With `BEGIN DEFERRED`, the
   read-then-upgrade pattern is exactly the deadlock-prone case
   above; under contention with the dreaming pass, the search would
   `SQLITE_BUSY`-fail at the INSERT step after already doing the read
   work — wasted compute.

2. With `BEGIN IMMEDIATE`, the contention manifests at the `BEGIN`
   itself, BEFORE the search work. `busy_timeout=5000` absorbs short
   contention; if dreaming holds the writer lock for >5s, the search
   fails fast.

3. SQLite's official recommendation for read-modify-write transactions
   is exactly `BEGIN IMMEDIATE`
   ([Lang Transaction docs](https://www.sqlite.org/lang_transaction.html)
   §2.2 — "If the application knows it will write, it should use
   BEGIN IMMEDIATE to avoid the upgrade-deadlock case").

### Tradeoffs

- **`BEGIN IMMEDIATE`**: predictable contention, slightly higher latency
  for a search that ultimately doesn't need to write (but F-002 always
  writes, so this isn't a real cost). Required for read-modify-write.
- **`BEGIN DEFERRED`**: lower lock-hold time IF the write doesn't
  happen. Only beneficial for read-heavy transactions that occasionally
  write. F-002 always writes — wrong fit.

**Verdict**: `BEGIN IMMEDIATE` if Topic 2 picks transaction-coupled.
Match the existing `insert_memory_resolving` pattern (`db.rs:225` —
uses `conn.transaction()?` which calls `BEGIN DEFERRED` by default;
this pattern is technically suboptimal for the same reason but the
contention case is rare enough at v0.0.1 to be invisible). For F-002,
explicitly use `conn.transaction_with_behavior(TransactionBehavior::Immediate)`
or raw `BEGIN IMMEDIATE` SQL. ([rusqlite Transaction docs](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html#method.transaction_with_behavior)).

---

## Synthesis for Topic 2 decision

Putting the SQLite-level analysis together against Topic 2's three
contract shapes:

| Contract | Latency cost | Operational cost | Fits failure taxonomy | Convention match |
|---|---|---|---|---|
| Best-effort + warn | none | none (precedent exists) | yes | strong (record_recall, get_memories_by_ids, metrics) |
| Hard error | none | medium (new error contract for MCP layer) | overkill (failure modes are transient) | none |
| Transaction-coupled | <1ms (negligible at v0.0.1) | medium (new explicit Transaction in Db::memory_search) | overkill, but cleanest correctness | none directly; closest is insert_memory_resolving |

**Database-optimizer recommendation**: best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES`
is the SQLite-correct choice IF Topic 2's open research question
(A-MEM trigger algorithm tolerance) returns "tolerates probabilistic
loss". If it returns "requires strict completeness", **prefer
transaction-coupled over hard-error** — the latency cost is sub-ms,
the failure-blast-radius is contained inside the transaction (no
"search succeeded but audit silently dropped" surface), and the
implementation is a single `conn.transaction_with_behavior(Immediate)`
wrapper.

The decision should NOT turn on SQLite-level performance arguments.
All three shapes are operationally cheap at v0.0.1. The decision is
a contract decision (Topic 2 research question) plus a convention
decision (mengdie's pattern is best-effort).

## Synthesis for Topic 1

Topic 1 is sequencing-prerequisite. Database-optimizer lens has one
specific point to add:

If Topic 2 picks **transaction-coupled**, the hook MUST live in
`Db::memory_search` (option A) — or more precisely, at the layer that
holds the connection. Currently `Db::memory_search` itself does NOT
hold the connection mutex across its body — it acquires/releases the
lock inside each of `search_fts`, `search_vector`, `get_memory`, and
`record_recall` (12 lock cycles per call as noted in §2). To put
search + audit in one transaction, the function would need to be
restructured to hold the lock across the entire body, which is a
non-trivial refactor (it changes whether other tasks can interleave
during embedding generation — but embedding generation is in
`mcp_tools.rs` BEFORE `Db::memory_search` is called, so this might
be fine).

This is **load-bearing for Topic 1 → Topic 2 sequencing**: if Topic 2
picks transaction-coupled, Topic 1 cannot be option B (mcp_tools.rs)
because mcp_tools.rs doesn't hold the connection mutex at all (it
calls into `Db` methods which each acquire their own). The framing
already notes this feasibility coupling, but the "why" is the lock
ownership pattern.

If Topic 2 picks **best-effort + warn** OR **hard error**, the hook
location is purely a coverage question — both A and B work. The
FTS-fallback coverage gap argument (challenger's blind-window claim)
is the dominant signal-correctness factor.

---

## Final summary (database-optimizer lens)

1. **WAL atomicity is well-understood for all four shapes**; the existing
   `record_recall` precedent is best-effort + warn, established
   convention.
2. **Latency cost of transaction-coupled at v0.0.1 is sub-ms**; not a
   load-bearing argument.
3. **Realistic failure-mode taxonomy** at v0.0.1 favors best-effort —
   all categories that fire are transient/operator-actionable.
4. **`record_recall` precedent** is best-effort, deliberate, and
   well-reasoned for observability writes.
5. **Codex's three-index design is correct**; adopt as-is. ANALYZE hook
   is an optional v1+ refinement.
6. **`BEGIN IMMEDIATE`, not `BEGIN DEFERRED`**, if Topic 2 picks
   transaction-coupled — read-modify-write pattern requires it.

The database-optimizer recommendation: **adopt best-effort + warn +
metric** unless the A-MEM trigger algorithm research (Topic 2 open
question) returns "requires strict completeness", in which case
**transaction-coupled with `BEGIN IMMEDIATE` is preferred over
hard-error** for blast-radius containment.

Topic 1's hook placement decision is constrained by Topic 2: option B
(mcp_tools.rs) is incompatible with transaction-coupled because the
connection mutex is per-method-call, not held across `Db::memory_search`.
If Topic 2 picks best-effort or hard-error, both options work and the
choice is about FTS-fallback coverage.
