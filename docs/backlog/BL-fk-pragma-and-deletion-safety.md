---
id: BL-fk-pragma-and-deletion-safety
status: open
origin: BL-007 /ae:review (security-reviewer + architecture-reviewer)
created: 2026-04-18
---

# Enable `PRAGMA foreign_keys = ON` (or guard deletion paths)

## Finding

`src/core/schema.rs` declares FOREIGN KEY clauses on
`memory_synthesis_links (source_memory_id, synthesis_memory_id)` but never
enables `PRAGMA foreign_keys = ON` at connection open. SQLite treats the
declarations as documentation only — no runtime enforcement.

Today this is latent: nothing in the code path `DELETE`s from
`memory_entries`. Invalidation uses `valid_until`. Synthesis insertions
only add rows.

The risk materializes the first time any plan introduces a hard delete
(e.g. a future `mengdie prune` command, a retention policy, a
user-requested "forget this memory"). Dangling `memory_synthesis_links`
rows would result, breaking any future audit / provenance query that
joins on them.

## Trigger

Fires when:
- A plan adds a `DELETE FROM memory_entries` path, OR
- A plan adds an audit/provenance feature that depends on link integrity
  (BL-009 search surface could plausibly join syntheses to sources), OR
- `PRAGMA foreign_keys` gets toggled by mistake elsewhere (defensive
  check during a schema refactor).

## Fix options

**Option A (preferred, one-liner)**: Add
`conn.execute_batch("PRAGMA foreign_keys = ON;")?;` to
`run_migrations` alongside the existing `PRAGMA journal_mode=WAL` /
`PRAGMA busy_timeout` calls. Any future `DELETE` that would orphan a
link row will error cleanly.

**Option B (belt-and-suspenders)**: Also add an ON DELETE CASCADE or
ON DELETE RESTRICT clause to the link table FK when first enabling
pragma (requires a v5 migration to drop + recreate the table, which is
fine since BL-007 just shipped v4 with no production data yet).

Do both in the same plan. Trigger the work when the first real
deletion-introducing plan lands.
