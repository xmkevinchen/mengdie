---
id: BL-031
title: audit-stats integration test — cross-layer assertion on nonzero supersession_count_30d
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "operator alerts based on `supersession_count_30d` start firing in production v0.0.1+ usage AND (a) need higher confidence in CLI-layer serialization of the field, OR (b) a future SQL refactor (CTE-extracting the WHERE clause) is proposed"
related: [F-005, F-002]
source: F-005 Step 4 accumulated Doodlestein checkpoint (commit 14590b4)
---

# BL-031: audit-stats integration test — cross-layer nonzero supersession assertion

## What

Add a `tests/audit_stats.rs` integration test that seeds 5+ supersession events (an `audit_returned_facts` row with a real `memory_entries` row whose `valid_until` is set within 7 days after the linked audit's `searched_at`), spawns the `mengdie audit-stats --format json` binary, and asserts the JSON `supersession_count_30d` field is `5` (not just type-checked as i64).

Today the unit test `core::db::test_audit_stats_supersession_count` exercises the SQL accessor side and the integration test `tests::audit_stats::test_json_format_schema` only asserts that the field is an i64 in the JSON output.  There is a thin cross-layer gap between SQL accessor correctness and CLI serialization for this specific field.

## Why it matters

If a future change ever drops or renames the `supersession_count_30d` field in the JSON output (e.g., a serde rename, a struct refactor, a typo in `AuditStatsOutput`), the unit tests would still pass and the JSON-schema test would still pass (the assertion is `obj["supersession_count_30d"].as_i64().is_some()` — a missing field would fail, but a value-shape divergence would not).  An operator alert wired off `status == "degraded" || supersession_count_30d > N` could miss a non-trivial regression.

Codex's accumulated-checkpoint review on commit `14590b4` framed this as a "thin cross-layer blind spot between SQL accessor correctness and CLI serialization/format output."

## Why deferred

Seeding a supersession event requires inserting into `memory_entries` with `valid_until` set.  Schema-v7's `vec_memories_insert` trigger references the `vec_memories` virtual table (`vec0` module from sqlite-vec).  A raw `rusqlite::Connection::open()` opened from inside `tests/audit_stats.rs` does NOT have the sqlite-vec extension registered (`db.rs::ensure_sqlite_vec_registered` is `pub(crate)`, not visible to integration tests), so the trigger fails to parse with `no such module: vec0`.

Three roads forward, none cheap, none in scope for v0.0.1 Step 4:

1. **Use the public `Db::open` API + a seed helper**: would link the integration test against `mengdie::core::*` and pull in fastembed (~90MB model download on first run) — defeats the test's no-fastembed-dependency design point.
2. **Expose a `pub fn ensure_sqlite_vec_registered_for_tests()` on the lib crate**: clean but adds production-shaped surface for test convenience only.
3. **Add a test-only `mengdie seed-audit-events --count N` subcommand**: heaviest, but lets all integration tests share a seed path.

None of these is appropriate during F-005's narrow operator-debug-subcommand scope.

## Trigger condition

This BL is filed `defer-until-trigger`.  Move it to a sprint when EITHER:

- An operator script wired to `mengdie audit-stats --format json` reports an unexpected `supersession_count_30d` value, AND we need higher confidence that the CLI layer is faithfully serializing what the SQL accessor returned, OR
- A future SQL refactor (e.g., extracting the shared WHERE clause into a SQL CTE or a Rust prelude const) is proposed — at that point the cross-layer assertion becomes the cheapest way to detect SQL-layer regressions before they reach a JSON consumer.

Until either trigger fires, the unit-level `test_audit_stats_supersession_count` plus the integration-level type-check coverage is judged sufficient.

## Hint at fix shape

```rust
// In tests/audit_stats.rs (when triggered):
//
// Pick option (2) above — add a single `pub fn` to the mengdie lib that
// lets integration tests seed a supersession event without going through
// fastembed.  Or add a small `mengdie seed-supersession --count 5`
// subcommand if option (3) is preferred for shared use across other
// future integration tests.
//
// Then assert:
//   let v = run_json(&db_path);
//   assert_eq!(v["supersession_count_30d"], 5);
```
