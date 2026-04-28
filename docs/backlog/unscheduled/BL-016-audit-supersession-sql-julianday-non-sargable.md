---
id: BL-016
title: "Audit supersession SQL — JULIANDAY filter is non-sargable; revisit when audit corpus grows"
status: open
created: 2026-04-28
origin: F-002 Step 1 code-review (Gemini track 3 P1)
trigger: "BL-014 (`mengdie audit-stats` CLI) ships AND audit_returned_facts grows past ~100K rows, OR the supersession SQL exceeds ~50ms wall-time on any production query. Earliest signal: BL-014's CLI subcommand prints user-visible latency on the supersession query."
---

# BL-016 — Replace JULIANDAY non-sargable filter in audit supersession SQL

## What

The F-002 supersession SQL (locked in plan AC4) includes:

```sql
WHERE me.valid_until IS NOT NULL
  AND JULIANDAY(me.valid_until) - JULIANDAY(a.searched_at) <= 7
  AND a.searched_at >= DATE('now', '-30 days')
```

The `JULIANDAY(...) - JULIANDAY(...) <= 7` predicate is non-sargable:
SQLite must compute `JULIANDAY()` per joined row and cannot use
`idx_memory_search_audit_searched_id` or `idx_memory_entries_valid_until_id`
for range elimination on this clause. At v0.0.1 corpus scale (a few thousand
audit rows worst case) this is invisible. Past ~100K audit rows, or once
this SQL is wired to a user-visible CLI subcommand (BL-014), the
full-table-scan-after-join cost surfaces.

## Why it matters

The plan settled (R7) on `searched_at TEXT NOT NULL` (RFC3339). Two
remediation options exist if this becomes a real bottleneck:

1. **Generated column + index**: add `searched_at_julian REAL GENERATED
   ALWAYS AS (JULIANDAY(searched_at)) STORED` (and equivalent for
   `valid_until`), index those, then rewrite the predicate to use the
   generated columns. SQLite ≥ 3.31 supports STORED generated columns.
   Backfill required for existing rows.
2. **Schema change**: store timestamps as INTEGER Julian-day or epoch
   seconds. Larger blast radius — touches `memory_entries.valid_from`,
   `valid_until`, `created_at`, `searched_at`, etc. across the codebase.
   Aligns with the Rust ecosystem's epoch-based idiom (`fastembed`,
   `serde-json` time formats).

Option 1 is the surgical fix; Option 2 is a project-wide refactor.

## Trigger

File the implementing plan when ANY of:

1. BL-014 (`mengdie audit-stats` CLI subcommand) ships AND the
   supersession query's user-visible latency exceeds ~50ms on any production
   database.
2. `audit_returned_facts` grows past ~100K rows (signal: a manual
   `SELECT COUNT(*) FROM audit_returned_facts` while scoping out
   maintenance overhead).
3. The downstream A-MEM trigger plan chooses a metric shape that requires
   running the supersession SQL on every search call (rather than
   periodically) — at that point query latency is on the hot path.

Until any of these fire, the JULIANDAY-filter cost is bounded by the
test-only role of the SQL in F-002 (AC4 schema-correctness gate, not a
runtime query).

## Why not now (Step 1 scope)

The plan settled `searched_at TEXT NOT NULL` (R7) after multi-reviewer
discussion. Changing this in Step 1 would invalidate the plan's R7 lock
and require re-review. The performance concern is bounded by v0.0.1
corpus scale — appropriate to file as a backlog item with a measurable
trigger condition.

## Reviewer note

Gemini track 3 in F-002 Step 1 review: `P1: Index Efficiency Gap for AC4
JULIANDAY Filter`. The full review is captured in the per-commit review
file under `docs/reviews/per-commit/<short-sha>.md`. Gemini also flagged
RFC3339 timezone consistency (P2) — that risk is waived because the
project convention is UTC-only `Utc::now().to_rfc3339()` at all timestamp
write sites; if a non-UTC writer is ever introduced, that risk re-surfaces
and a separate BL is appropriate.
