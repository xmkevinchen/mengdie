---
type: adversarial
plan: "017"
reviewer: doodlestein
created: 2026-04-23
---

# Adversarial: Plan 017 — Where Does It First Fail?

## First Failure: Step 1, Transaction Wrapping

**The plan instructs**: "Wrap v4→v5 block in an explicit transaction: `let tx = conn.transaction()?; ... tx.commit()?;`"

**What actually exists**: `run_migrations` in `src/core/schema.rs` uses `conn.execute_batch()` calls throughout, with no `Connection` borrowed as a transaction — it takes `&Connection`, not `&mut Connection`. `rusqlite::Connection::transaction()` requires `&mut self`. The existing function signature is `pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()>`.

**The land mine**: To wrap the v5 block in a real `rusqlite` transaction, the implementor must change the function signature to `&mut Connection` (or switch to a raw `SAVEPOINT` via `execute_batch`). Neither path is documented in the plan. The plan cites `conn.transaction()?` as if it just works — it won't compile against the existing signature. The implementor will hit a borrow-check error on first compile.

The existing migration pattern (v2, v3, v4 blocks) all use bare `conn.execute_batch()` with no transactions — so there is no in-codebase precedent to copy from. The plan calls this "a new pattern for this codebase" and says "document at code time," but gives no guidance on which of the two approaches to take:

- Option A: change `run_migrations` to `&mut Connection` — ripples to every call site (`Db::open`, `Db::open_in_memory`, and the existing tests that pass `&conn`).
- Option B: use `SAVEPOINT v5_migration; ... RELEASE v5_migration;` via `execute_batch` — stays on `&Connection` but is less idiomatic and has different semantics on rollback.

The pre-checks (orphan links, zero-link syntheses, legacy duplicate coalescing) all issue queries that themselves require the connection — meaning they must run *inside* the same transaction to be safe against concurrent writes. But if the transaction is a `rusqlite::Transaction`, it exclusively borrows `conn`, and the pre-checks can't also use the shared `Arc<Mutex<Connection>>` Db handle simultaneously. This isn't a deadlock (migration runs at startup before Db is shared), but the implementor needs to confirm this at code time or restructure.

**Blast radius**: All of Step 1. Steps 2–5 depend on Step 1 completing correctly. If the implementor sidesteps the transaction (e.g., drops it and runs bare `execute_batch` sequences), the plan's rollback guarantee is silently violated — the migration can leave the DB in a partial v5 state (column added, backfill half-done, index missing) if the process is killed mid-run.

---

## Second Failure: Step 1, Pre-check 3 — Duplicate Cluster Detection SQL is Not Specified

The plan describes the coalesce logic in prose ("for each distinct sorted+dedup source set... detect whether the v4 DB contains multiple synthesis rows for that same cluster") but gives no SQL. The implementor must independently derive:

1. A query that groups `memory_synthesis_links` rows by `synthesis_memory_id`, constructs the sorted source-id set per synthesis, and identifies which syntheses share the same set.
2. This requires either: (a) fetching all link rows into Rust memory and grouping there (acceptable for 27 rows), or (b) a recursive/window-function CTE in SQLite that aggregates and compares string sets — non-trivial SQL that SQLite supports poorly compared to Postgres.

The plan says "cover this path with a migration test" but does not define what "same cluster" means in SQL at the v4 level where `synthesis_cluster_hash` does not yet exist. The implementor must write the detection logic from scratch. This is the most algorithmically novel piece of the migration and is the most likely place for a subtle bug (e.g., failing to normalize the set order before comparison, producing false-no-duplicates when duplicates exist with different link insertion order).

---

## Third Failure: Step 1, `ALTER TABLE ADD CONSTRAINT CHECK` — Acknowledged but No Concrete Path

The plan correctly notes "SQLite does not support `ALTER TABLE ... ADD CONSTRAINT CHECK` on existing tables" and says "use trigger if ALTER CHECK is unsupported." But the trigger fallback has a correctness difference the plan does not address: a `BEFORE INSERT/UPDATE` trigger that raises on invalid `source_type` values will fire for ALL inserts, including the backfill loop that writes `source_type = 'synthesis'` (which is valid) but also the pre-existing rows that might have `source_type` values outside the planned allowlist.

The plan's allowlist is `('conclusion', 'review', 'plan', 'analysis', 'retrospect', 'synthesis')`. But the existing codebase has tests inserting `source_type = 'conclusion'` and production code using the same. If any production DB row has a `source_type` not in that list (e.g., a `'note'` or `'memo'` type that existed before the enum was formalized), the trigger would reject the entire migration's UPDATE path.

The plan does not specify: (1) whether to verify all existing `source_type` values are in the allowlist before adding the constraint/trigger, and (2) what to do if they aren't.

---

## Fourth Failure: Step 3, `tests/dream_synthesis.rs` — All Existing Tests are `#[ignore]`

The plan says "append integration tests NOT `#[ignore]` per dep-analyst Q4." The existing `tests/dream_synthesis.rs` has exactly one test, `end_to_end_dream_synthesis_writes_one_row_with_six_links`, which is `#[ignore]` because it requires an authenticated `claude` CLI.

The new tests (synthesis-audit subcommand, cluster-hash dedup, order-independence) do NOT require the claude CLI — they use `Db::open_in_memory()` and `CARGO_BIN_EXE_mengdie`. So NOT marking them `#[ignore]` is correct. However, the plan says "Pattern from `tests/decay_contract.rs`: hold `NamedTempFile` past `Command::output()`" — `NamedTempFile` is not currently imported in `dream_synthesis.rs` (which has no subprocess invocations). The implementor must add the `tempfile` crate dependency or verify it's already in `[dev-dependencies]`.

Minor but concretely blocking: if `tempfile` is not in dev-dependencies, the import fails and the test file won't compile.

---

## Fifth Failure: Step 2 Line Reference is Off

The plan says "Rewrite `insert_synthesis_with_links` at `src/core/db.rs:332-389`." The current function is at those lines. However, Step 2 instructs: "Change the INSERT to include `synthesis_cluster_hash` column and the value." The column does not exist in the schema until Step 1's migration runs at RUNTIME. The Rust INSERT statement referencing the column will compile fine (SQLite column names are strings, not typed at compile time). But the unit tests in Step 2 (`test_insert_synthesis_with_links_upserts_on_same_cluster`) open an in-memory DB via `Db::open_in_memory()`. If `Db::open_in_memory()` calls `run_migrations()`, and `run_migrations()` now runs the v5 migration on a fresh DB, the column will exist. This is fine. But if any Step 2 unit test seeds the DB at v4 and then calls `insert_synthesis_with_links` (which now writes to `synthesis_cluster_hash`), the INSERT will fail at runtime with "table memory_entries has no column named synthesis_cluster_hash." The plan does not flag this for Step 2 tests — the implementor must ensure all Step 2 unit tests open the DB via the v5-migrated path, not a manually seeded v4 state.

---

## Summary

**Clearest first-failure point**: Step 1, transaction wrapping. The plan instructs `conn.transaction()?` but `run_migrations` takes `&Connection` — `transaction()` requires `&mut self`. This is a compile error on the first line of implementation. The fix choice (signature change vs. SAVEPOINT) cascades to call sites and to the architecture of the pre-check queries. Everything else in the plan is executable, with varying degrees of implementor judgment required.
