---
id: BL-013
title: "audit_returned_facts orphan link-row cleanup (rename_project collision-merge buildup)"
status: open
created: 2026-04-28
origin: "discussion 029 doodlestein-adversarial-post Finding 2"
trigger: "audit_returned_facts row count exceeds 10x memory_search_audit row count (orphan ratio > 10:1)"
depends_on: [F-002]
size: XS
---

# BL-013 — audit_returned_facts orphan link-row cleanup

## Origin

Surfaced by discussion 029 doodlestein-adversarial-post Finding 2 as a slow-burn
problem from F-002's `rename_project` FK coupling decision (R6 in 029
conclusion: PRAGMA foreign_keys is OFF, so collision merges create silent
orphan rows in `audit_returned_facts`).

## Problem

Each `rename_project` collision merge (`db.rs:636 DELETE FROM memory_entries
WHERE id = ?1`) deletes a `memory_entries` row but leaves orphan rows in
`audit_returned_facts` whose `fact_id` no longer matches any live entry. The
supersession query (F-002 acceptance) inner-joins on `memory_entries.id` and
naturally excludes orphans, so correctness is unaffected. However:

- SQLite's ANALYZE-based query planner uses row-count statistics (not
  "live row" statistics) to estimate join cost.
- At v0.0.1 corpus size measured by db-optimizer (1000 facts / 300 audit rows
  / 3000 link rows), accumulating 10× orphans could shift the planner away
  from the F-002 covering index `idx_audit_returned_facts_fact_audit` toward
  a table scan, breaking the supersession query latency.
- The orphan accumulation is invisible until the cardinality flip happens.

## Trigger

Fire when **`audit_returned_facts` row count exceeds 10× `memory_search_audit`
row count** (the orphan-ratio threshold). At that point, the cleanup script
becomes worth running.

## Fix

Periodic cleanup query, run as part of mengdie's existing dreaming pass or as
a separate `mengdie cleanup-orphans` subcommand:

```sql
DELETE FROM audit_returned_facts WHERE fact_id NOT IN (SELECT id FROM memory_entries)
```

This is a cheap indexed scan given F-002's `idx_audit_returned_facts_fact_audit`
covering index. No schema change needed.

## Why deferred (not fixed in F-002)

- Trigger condition is not yet observable; mengdie has not yet shipped F-002 (no audit table exists).
- At v0.0.1 corpus + low rename frequency, the trigger is unlikely to fire in the first 6 months.
- The "PRAGMA enforcement enable" path (separate BL trigger from 029 R6) is
  the long-term fix; this BL is the simpler interim mitigation if PRAGMA flip
  doesn't happen first.
