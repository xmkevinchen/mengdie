---
id: BL-015
title: "PRAGMA foreign_keys = OFF — make project-wide assumption explicit in run_migrations"
status: closed
created: 2026-04-28
closed: 2026-04-29
closed_by: "F-002 Step 4 (commit pending) — PRAGMA foreign_keys = OFF written in run_migrations after WAL/busy_timeout setup"
origin: F-002 Step 1 code-review (Codex track 2 P2)
trigger: "Any caller of Db::open / Db::open_in_memory enables PRAGMA foreign_keys = ON, OR a future bundled-rusqlite build defaults FK enforcement to ON. Earliest signal: an audit_returned_facts INSERT failing with FK constraint error on a missing memory_entries.id."
trigger_fired: 2026-04-29
trigger_signal: "F-002 Step 4 unit tests on the strict audit helper failed with 'FOREIGN KEY constraint failed' (rusqlite Error code 787) because the bundled SQLite build had FK enforcement ON and the test fact_ids did not reference real memory_entries rows. Confirmed Codex's Step 1 P2 read."
---

# BL-015 — Make `PRAGMA foreign_keys = OFF` explicit in `run_migrations`

## What

Add `conn.execute_batch("PRAGMA foreign_keys = OFF;")?;` early in
`run_migrations` (after the WAL + busy_timeout PRAGMA writes, before any
migration block runs). This makes the project-wide "FKs are documentation
only" assumption explicit and runtime-asserted instead of relying on the
SQLite/rusqlite default behavior.

## Why it matters

Plan F-002 / discussion 029 YAGNI 1 settled "PRAGMA foreign_keys stays OFF
project-wide; FK clauses on `audit_returned_facts` are documentation-only".
But production code never actually writes `PRAGMA foreign_keys = OFF` —
only the test helper `seed_v4_db()` (`schema.rs:678`) does, with a comment
explicitly noting that "rusqlite with the bundled feature compiles SQLite
with FK enforcement ON by default in some builds".

If FK enforcement is ever ON during a `record_search_audit_best_effort`
call against a `memory_entries.id` that has been deleted (or against a
fact-id that refers to a future-tombstoned entry), the audit-write fails
with an FK constraint error. F-002's best-effort wrapper would catch the
error → bump `audit_write_failures` counter → emit `tracing::warn!` —
functionally OK but produces silent under-counting that masks real audit
write paths from being observable.

Wider blast radius: ANY table with FK clauses in the project (currently
`memory_synthesis_links`, `audit_returned_facts`, future tables) would
start enforcing constraints the project never planned for. The orphan-link
cleanup plan (BL-013) explicitly assumes orphans are allowed.

## Trigger

File the implementing plan when ANY of:

1. A caller of `Db::open` or `Db::open_in_memory` is found that needs FK
   enforcement on (e.g., a strict-mode test suite). Then the global
   assertion belongs in the OPPOSITE place — `Db::open` should remain
   ambiguous and per-test setup should set the desired posture.
2. A `bundled` rusqlite version-bump or build flag changes the FK default.
   Watch `Cargo.toml` `rusqlite` version moves and the upstream changelog.
3. An audit-write failure in production turns out to be caused by FK
   enforcement (visible via `tracing::warn!` line stating FK constraint
   error rather than the expected drop-table or disk-full causes).

## Implementation sketch (when triggered)

```rust
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA busy_timeout=5000;")?;
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;  // <-- add this
    // ... rest unchanged
}
```

One-line change. Test impact: `seed_v4_db()` can drop its redundant
`PRAGMA foreign_keys = OFF` (since `run_migrations` will now enforce it
unconditionally). All existing tests should continue to pass.

## Why not now (Step 1 scope)

Plan F-002 Step 1 expected files = `src/core/schema.rs` only. Adding the
PRAGMA line is technically in-scope, but the plan author did not include
it; doing so unilaterally would expand the step beyond what plan-review
absorbed. The change is a project-wide invariant assertion, not a
Step-1-specific one — appropriate to file as its own targeted plan.

## Reviewer note

Codex track 2 in F-002 Step 1 review: `Concern 2 — Foreign key enforcement
(lines 254–256): P2`. The full review is captured in the per-commit
review file under `docs/reviews/per-commit/<short-sha>.md`.
