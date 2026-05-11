---
id: BL-037
title: "vec_memories sync triggers — close downgrade gap when 384-d row updated to non-384 / NULL embedding"
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "any future code path that allows non-384 / NULL UPDATE on memory_entries.embedding (e.g., manual UPDATE, schema migration that nulls embeddings, multi-model support that introduces embedding_dim != 384)"
related: [F-006]
source: F-006 /ae:review (cross-family Codex P2 #2)
---

# BL-037: vec_memories sync triggers — close downgrade gap

## What

The schema-v7 `vec_memories_update` trigger at `src/core/schema.rs:384-391` fires `AFTER UPDATE OF embedding ON memory_entries WHEN NEW.embedding_dim = 384`. If a row that previously had a 384-d embedding (and therefore has a corresponding row in `vec_memories`) is later updated to a non-384 dimension OR to `NULL`, the trigger's `WHEN` clause evaluates to false and the trigger body is skipped. The stale `vec_memories` shadow row remains, with no in-memory entry to back it.

**Concrete scenario** (Codex P2 review on F-006 commit `01910cc`):

1. Row created with 384-d embedding (production path: ingest pipeline → `Db::store_embedding(id, vec, 384)`). `vec_memories_insert` trigger fires; `vec_memories` gets a row.
2. Row's embedding column is updated to NULL or to a 3-d test vector (path: a future `mengdie reindex` command, OR a manual SQL `UPDATE`, OR a schema migration that nulls embeddings, OR multi-model support that introduces `embedding_dim != 384`).
3. `vec_memories_update` WHEN clause = false → trigger skipped → stale `vec_memories` row persists.

The new `test_dim_mismatch_skips_vec_memories` test (commit `01910cc`) covers a different case: insert-no-prior-row. It verifies the WHEN clause skips when there's no prior `vec_memories` row to clean up. It does NOT verify the downgrade case where a prior row exists.

## Why deferred

The downgrade scenario is **not currently reachable** in production code:

- `Db::store_embedding` (the canonical embedding write path) is dim-strict (rejects non-384 since BL-026; verified by `test_store_embedding_dimension_mismatch`).
- No code path performs a manual `UPDATE memory_entries SET embedding = NULL` or similar.
- No multi-model / variable-dim support exists.

Filing as `defer-until-trigger` captures the gap explicitly. The fix is small (~5 lines: either widen the WHEN clause to handle the cleanup case, OR add a separate "downgrade DELETE" trigger). When a real code path that triggers this scenario lands, this BL gets pulled into that sprint.

## Trigger condition

Move this BL to a sprint when ANY of the following appears in code:

- A `mengdie reindex` / `mengdie re-embed` command that updates existing rows' embeddings (paired by Cargo.lock changes if it requires a new dep, OR by new src/ module).
- Multi-model support introducing `embedding_dim != 384` (variable-dim memories require either a separate vec_memories table per dim OR a redesign of the dim-strict trigger).
- A schema migration that nulls embeddings (e.g., for storage pruning).
- A test that explicitly stores 384-d and then updates to non-384 (the act of writing this test would trigger the BL pickup).

## Hint at fix shape

Two equivalent options:

**Option A — widen the existing UPDATE trigger** to handle the downgrade case:

```sql
CREATE TRIGGER vec_memories_update_or_downgrade
    AFTER UPDATE OF embedding ON memory_entries
BEGIN
    DELETE FROM vec_memories WHERE memory_id = OLD.id;
    -- Re-insert only if NEW satisfies the dim invariant
    INSERT INTO vec_memories (memory_id, embedding)
    SELECT NEW.id, NEW.embedding
    WHERE NEW.embedding IS NOT NULL AND NEW.embedding_dim = 384;
END;
```

**Option B — add a separate DELETE-on-downgrade trigger**:

```sql
CREATE TRIGGER vec_memories_downgrade_delete
    AFTER UPDATE OF embedding ON memory_entries
    WHEN OLD.embedding_dim = 384
      AND (NEW.embedding IS NULL OR NEW.embedding_dim != 384)
BEGIN
    DELETE FROM vec_memories WHERE memory_id = OLD.id;
END;
```

Option A is cleaner (one trigger, the existing WHEN-clause scope just widens). Option B preserves the existing trigger and adds a sibling. Pick at sprint pickup based on which is easier to test.

After fix, extend `test_dim_mismatch_skips_vec_memories` to cover the downgrade case: insert 384-d row → assert vec_memories has 1 row → update to NULL/3-d → assert vec_memories has 0 rows.

## Out of scope

- The "stale rows from `invalidate_memory`" case. That's a different design choice (intentional; valid_until is not in the trigger watch list). See `vector.rs::search_vector` doc comment + BL-013 for that scope.
- Performance optimization of the trigger sync at bulk-load (separate concern; performance reviewer Q3).

## F-006 relationship

F-006 close-out's test_dim_mismatch_skips_vec_memories test catches the insert-no-prior-row case but not the downgrade case. F-006 ships with this gap documented; the downgrade scenario is unreachable in current code so the gap is theoretical.
