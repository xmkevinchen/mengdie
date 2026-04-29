---
id: BL-018
title: "Move `PRAGMA foreign_keys = OFF` from `run_migrations` to `Db::open`"
status: open
created: 2026-04-29
origin: F-002 /ae:review (architecture-reviewer P2)
trigger: "Any new Db open path is added that does not call run_migrations, OR run_migrations is refactored to add an early-return optimization (e.g. `if current_version == head { return Ok(()) }`). Earliest signal: silent FK enforcement on a connection opened via the new path, manifesting as `audit_returned_facts` insert failures with FK constraint errors."
---

# BL-018 — Move PRAGMA foreign_keys = OFF earlier in the connection lifecycle

## What

`PRAGMA foreign_keys = OFF` currently lives inside `run_migrations`
(`src/core/schema.rs:100`, added at F-002 Step 4 commit c2544ea as the
BL-015 close-out). Move it to `Db::open` and `Db::open_in_memory`
(`src/core/db.rs`) so that the project-wide invariant ("FKs are
documentation-only") is enforced at the **connection lifecycle layer**,
not as a side-effect of schema migration.

Both current `Db` constructors call `run_migrations` immediately after
opening the connection, so the current placement is functionally
correct. The issue is layering: `PRAGMA foreign_keys = OFF` is a
connection-level setting, not a schema migration step. A future
refactor (e.g. early-return optimization in `run_migrations` when
`user_version == head`, or a new `Db::open_for_readonly_query` path
that intentionally skips migration) could silently drop the FK-OFF
guarantee without any compile-time signal.

## Why it matters

Architecture reviewer flagged this as P2 latent fragility during F-002
/ae:review. The argument:

- BL-015 was filed at F-002 Step 1 and closed at Step 4 because the
  bundled rusqlite build had FK enforcement ON (the implicit-OFF
  assumption was wrong for the test environment).
- BL-015's close-out put `PRAGMA foreign_keys = OFF` inside
  `run_migrations` because that was the simplest one-line fix to the
  observed test failures.
- But the SEMANTIC of "PRAGMA OFF" is connection-scoped, not
  schema-scoped. Tying it to migration creates a dependency that's
  invisible to a future maintainer who doesn't know the connection
  default's history.

## Trigger

File the implementing plan when ANY of:

1. A new `Db::open_*` variant is proposed that does not call
   `run_migrations`. Today only `Db::open` and `Db::open_in_memory`
   exist; both call `run_migrations`. A read-only or
   migration-skipping variant is a likely future addition.
2. `run_migrations` is refactored to add an early-return optimization
   on the head-version path. Today `run_migrations` always runs all
   PRAGMA writes; an "already at head, skip" early-return would skip
   the FK PRAGMA too.
3. An audit of bundled rusqlite version bumps reveals a connection
   default change. The current concern is that "default OFF" is
   build-dependent; future bundled builds may differ.

Until any of these fire, the current placement is functionally
correct and BL-018 is a hardening improvement, not a bug fix.

## Implementation sketch (when triggered)

```rust
// src/core/db.rs
impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path).context("failed to open SQLite connection")?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;  // <-- add here
        run_migrations(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory connection")?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;  // <-- add here
        run_migrations(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}
```

Then remove the `PRAGMA foreign_keys = OFF;` from `run_migrations`
(`src/core/schema.rs:100`) — it becomes redundant. Keep the comment as
a cross-reference to discussion 029 YAGNI 1 if helpful.

Test impact: `seed_v4_db()` in `schema.rs mod tests` already does
`PRAGMA foreign_keys = OFF` explicitly (line ~678), so it's idempotent
with the new `Db::open_in_memory` posture. No test changes required.

## Why not now (F-002 scope)

F-002 Step 4's BL-015 close-out chose the minimum-viable fix
(one-line PRAGMA write in `run_migrations`) to unblock the test
failures. Refactoring to `Db::open` is a layering improvement, not a
correctness fix. F-002 /ae:review chose to file as backlog rather than
extend Step 4's scope — keeps the close-out commit surgical and lets
the user prioritize the layering refactor against other backlog items.

## Reviewer note

Architecture reviewer in F-002 /ae:review (commit pending). Codex
Track 4 cross-family review separately confirmed BL-015 close-out is
functionally complete; Architecture's concern is forward-looking, not
a regression on the current commit.
