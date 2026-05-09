---
id: BL-032
title: audit_returned_facts — add (audit_id) index for query-5 supersession JOIN
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "`memory_search_audit` row count >10k OR `mengdie audit-stats` execution time >20ms (verifiable via SQLite EXPLAIN QUERY PLAN)"
related: [F-005, F-002]
source: F-005 feature-completion review (performance-reviewer P2 finding)
---

# BL-032: `audit_returned_facts` — add `(audit_id)` index for query-5 JOIN

## What

Add an index on `audit_returned_facts(audit_id)` (or restructure the existing index from `(fact_id, audit_id)` to `(audit_id, fact_id)`) so that the supersession JOIN query (`AUDIT_STATS_SUPERSESSION_COUNT_SQL`, query 5 in `Db::audit_stats()`) can use an index seek for the `arf.audit_id = a.id` lookup, instead of falling back to a full table scan.

## Why it matters

The supersession-count query JOIN path is:

```
memory_search_audit a
  → JOIN audit_returned_facts arf ON arf.audit_id = a.id   ← driven by audit_id
  → JOIN memory_entries me ON me.id = arf.fact_id
```

The existing index `idx_audit_returned_facts_fact_audit ON audit_returned_facts(fact_id, audit_id)` has `fact_id` as the leading column. SQLite B-tree indices can only use the leading column for prefix matches; `audit_id` in the second position cannot satisfy the JOIN's `arf.audit_id = ?` lookup. The query planner therefore falls back to a full table scan of `audit_returned_facts`, doing a primary-key lookup on `memory_entries` for each row.

At v0.0.1 personal-KB scale (<5k audit rows, <20k link rows), the full-table-scan path is sub-millisecond — no observable user-facing impact. But the audit table grows monotonically with every search; a year of daily usage could realistically push it past 10k-50k rows. At that scale, query 5 changes from O(1) JOIN to O(n) scan, with execution time growing from <1ms to 10-50ms.

## Why deferred

The trigger threshold (>10k audit rows OR >20ms latency) is comfortably above current personal-KB scale. Adding an index now is a schema-migration write that the v0.0.1 user does not benefit from — the BL captures the fix and the trigger so we add the index when it's needed and not before.

## Trigger condition

Move this BL to a sprint when EITHER:

- The `memory_search_audit` row count exceeds 10,000 in the operator's production DB (verify via `SELECT COUNT(*) FROM memory_search_audit` or the existing `mengdie audit-stats` `audit_count` field), OR
- An operator runs `EXPLAIN QUERY PLAN` against `AUDIT_STATS_SUPERSESSION_COUNT_SQL` and observes a `SCAN audit_returned_facts` plan (full-table scan), OR
- The `mengdie audit-stats` command takes >20ms wall-clock to execute against the operator's production DB (timing measurement: `time mengdie audit-stats --format json`).

## Hint at fix shape

Two equivalent options; pick whichever fits the migration discipline:

**Option A — additive (preferred for safety):**
```sql
-- New migration vN+1:
CREATE INDEX IF NOT EXISTS idx_audit_returned_facts_audit
    ON audit_returned_facts(audit_id);
```
Keeps the existing `(fact_id, audit_id)` index for any other query that needs `fact_id` lookup; just adds the missing `audit_id` index.

**Option B — restructure:**
```sql
-- Drop and recreate with reversed column order:
DROP INDEX IF EXISTS idx_audit_returned_facts_fact_audit;
CREATE INDEX idx_audit_returned_facts_audit_fact
    ON audit_returned_facts(audit_id, fact_id);
```
Smaller index footprint (one index instead of two), but requires verification that no other query relies on the `fact_id`-leading shape.

Verify the fix via `EXPLAIN QUERY PLAN` showing `SEARCH audit_returned_facts USING INDEX idx_audit_returned_facts_audit` (option A) instead of `SCAN audit_returned_facts`.
