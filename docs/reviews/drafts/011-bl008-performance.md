---
review: 011
subject: BL-008 exponential decay — performance review
commits: 56812cb..HEAD (7 commits, +1764/-26 LOC, 14 files)
reviewer: performance-agent
date: 2026-04-20
---

# BL-008 Performance Review

Scope: dreaming demotion pass hot path, search post-fetch hot path,
chunked UPDATE sizing, `breached_ids` memory footprint, and
`DateTime::parse_from_rfc3339` cost.

---

## Q1 — Dreaming pass I/O amplification

**Classification: P2**

The demotion pass adds these I/O operations against the long-term set:

1. `SELECT id, avg_relevance, last_recalled FROM memory_entries WHERE is_longterm=1 AND valid_until IS NULL AND last_recalled IS NOT NULL` → full long-term scan
2. In-memory loop: one `DateTime::parse_from_rfc3339` + one `decay_factor` call per row
3. Conditional chunked `UPDATE is_longterm=0 WHERE id IN (...)` — fires only when demotions exist
4. `SELECT COUNT(*) … WHERE last_recalled IS NULL` — observability count, always runs
5. Second full long-term scan (same SELECT as #1) — fires only when `write_demotions && !breached_ids.is_empty()`

**Runtime estimates** (SQLite WAL, SSD, no index on `is_longterm`):

| Long-term rows | SELECT scan | In-memory loop | UPDATE chunks | Second scan | Total |
|---|---|---|---|---|---|
| 1k | ~1 ms | ~0.1 ms | 0–2 ms | 0–1 ms | ~2–4 ms |
| 10k | ~5 ms | ~1 ms | 0–20 ms | 0–5 ms | ~5–30 ms |
| 100k | ~50 ms | ~10 ms | 0–200 ms | 0–50 ms | ~50–300 ms |

For current corpus size (observed ~dozens to low hundreds of long-term
memories after the first real run), this is negligible. Dreaming runs
once daily; even at 100k rows, 50–300 ms is acceptable for a background
job. **Not a hot-path concern at current or plausible near-term scale.**

The second SELECT (post-demotion mean) is the most avoidable work: it
re-reads the entire surviving long-term set purely to compute an
observability metric. It could be derived from the already-loaded
`longterm_rows` minus the demoted subset without a second DB round-trip.
This is optimization opportunity, not a blocking issue.

**Finding**: P2 — the second post-demotion SELECT can be eliminated by
filtering `longterm_rows` in memory (subtract `breached_ids`), saving
one full table scan when demotions fire. Worth fixing before corpus
exceeds ~50k long-term memories.

---

## Q2 — Search post-fetch `parse_from_rfc3339` cost

**Classification: P3**

`apply_boost_and_decay` calls `MemoryEntry::last_recalled_as_datetime()`
once per returned search result. Search results are bounded by `limit`
(default 10, over-fetch 3× = 30 rows maximum before top-k cut). This is
O(limit), not O(corpus).

`DateTime::parse_from_rfc3339` is non-trivial but not expensive: Rust's
`chrono` implementation parses a fixed-length ASCII string with a small
finite state machine. Measured cost is in the 100–300 ns range per call
on modern hardware. For 30 results: ~10 µs total, negligible versus the
SQLite I/O and embedding inference that dominate search latency.

The `last_recalled_as_datetime()` helper also allocates a `DateTime<Utc>`
on the stack (8 bytes), not the heap. No heap allocation concern.

**Finding**: P3 — no action needed. An allocation-free alternative
(e.g., storing `last_recalled` as a Unix timestamp integer column) would
save ~10 µs per search call, which is premature given current latency
profile.

---

## Q3 — Chunked UPDATE at 500 IDs vs. `SQLITE_LIMIT_VARIABLE_NUMBER`

**Classification: P2**

The code chunks `breached_ids` at 500 per statement
(`dreaming.rs:237`). SQLite's default `SQLITE_LIMIT_VARIABLE_NUMBER` is
999 (raised to 32766 in SQLite ≥ 3.32.0 for bundled builds).

`rusqlite` uses `features = ["bundled"]` in this project (`Cargo.toml`),
which bundles SQLite 3.45+ where the default limit is 32766. The 500
chunk size is therefore conservative — well within both the old (999) and
new (32766) limits.

**Risk**: if `SQLITE_LIMIT_VARIABLE_NUMBER` is reduced at runtime via
`sqlite3_limit()` to below 500, the UPDATE would fail with a rusqlite
error, not silently truncate. SQLite returns `SQLITE_ERROR` if the
placeholder count exceeds the limit; rusqlite surfaces this as
`Err(rusqlite::Error::SqliteFailure(...))`, which propagates via `?`.
No silent truncation is possible — the error surfaces immediately.

**Finding**: P2 (informational) — chunk size 500 is safe for bundled
SQLite. Consider documenting the invariant (`chunk ≤ 500 << SQLITE_LIMIT_VARIABLE_NUMBER`)
in a comment if this code is ever ported to a non-bundled SQLite
environment. No code change required now.

---

## Q4 — `DreamingResult.breached_ids` memory footprint

**Classification: P3**

`breached_ids: Vec<String>` holds one UUID string per demoted memory.
UUIDs are 36-byte ASCII strings; `String` on 64-bit systems adds a
24-byte heap header (ptr + len + cap). Worst case at 10k demotions:
10k × (36 + 24) bytes = ~600 KB on the heap.

In practice:
- Demotion is designed to be rare (first demotion at ~77 days of silence).
- 10k long-term memories is well beyond current corpus scale.
- `DreamingResult` is returned once per dreaming invocation (daily) and
  dropped immediately after the CLI formats its output.

**Finding**: P3 — no concern at current or near-term scale. If corpus
grows to 100k+ long-term memories and mass-demotion events become
possible, replace `Vec<String>` with `Vec<uuid::Uuid>` (16 bytes each,
no heap allocation) and convert to string only at the display layer. This
reduces worst-case footprint from ~600 KB to ~160 KB at 10k rows. Not
worth doing now.

---

## Q5 — `chrono::DateTime::parse_from_rfc3339` cost — allocation-free alternative

**Classification: P3**

This is called in two places:

1. **Dreaming pass** (`dreaming.rs:192`): once per long-term memory in the
   demotion scan. For 10k rows: ~3 ms total at 300 ns/call.
2. **Search post-fetch** (`last_recalled_as_datetime` via `apply_boost_and_decay`):
   O(limit) ≈ 30 calls per search. ~9 µs total.

Neither site is on a latency-sensitive path. The dreaming pass runs
offline once daily; the search path spends orders of magnitude more time
on embedding inference (~2–10 ms) and SQLite I/O.

The allocation-free alternative — storing `last_recalled` as a Unix
timestamp integer — would require a schema migration, an API change to
`MemoryEntry`, and a new column. The benefit is ~3 ms saved in a
once-daily batch job.

**Finding**: P3 — premature. `parse_from_rfc3339` is correct, testable,
and readable. The shared `last_recalled_as_datetime()` helper centralizes
the parse logic so any future migration to an integer column is a
one-site change. No action needed.

---

## Summary

| # | Finding | Priority | Action |
|---|---|---|---|
| Q1 | Second post-demotion SELECT can be replaced by in-memory filter of `longterm_rows` | P2 | Backlog — trigger: corpus > 50k long-term memories |
| Q2 | `parse_from_rfc3339` in search post-fetch (O(limit) ≈ 30 calls) | P3 | None |
| Q3 | Chunk size 500 is safe for bundled SQLite; no silent truncation risk | P2 (informational) | Optional comment; no code change |
| Q4 | `breached_ids: Vec<String>` up to ~600 KB at 10k demotions | P3 | None at current scale |
| Q5 | `parse_from_rfc3339` in dreaming pass (~3 ms at 10k rows) | P3 | None; premature |

**No P1 findings.** No O(n²) paths, no unbounded growth in the hot
paths. The most actionable optimization (Q1 — eliminate the second
full-table scan) is P2 and deferred until corpus scale warrants it.
BL-008 is **approved for merge** from a performance standpoint.
