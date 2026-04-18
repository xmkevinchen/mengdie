---
id: BL-valid-until-boundary
status: open
origin: BL-006 Step 2 Doodlestein review
created: 2026-04-18
---

# Unify `valid_until` boundary semantics across readers

## Finding

`src/core/vector.rs::search_vector` and `src/core/clustering.rs::load_embeddings`
both filter `(valid_until IS NULL OR valid_until > ?1)` where `?1` is a
freshly-captured `chrono::Utc::now()`. `db::invalidate_memory` also captures
its own `Utc::now()` and writes it to `valid_until`.

The strict `>` is exclusive. Under concurrent access, a row whose
`valid_until` is captured a microsecond AFTER the reader's `now` would pass
the filter even though it was just invalidated. In practice the
`Arc<Mutex<Connection>>` serializes calls so this race cannot happen today.
If the DB connection strategy ever changes (connection pool, WAL-mode
multi-reader), the race opens up.

## What to do

- Option A: change both SELECT filters to `valid_until >= ?1` so equal
  timestamps also exclude.
- Option B: keep `>` but guarantee writer's `valid_until` is captured AFTER
  the lock is acquired (matches reader pattern). `invalidate_memory`
  currently captures `now` before `conn.lock()`.

Either is one-line. Do both for safety.

## Trigger to revisit

When SQLite access moves off `Arc<Mutex<Connection>>` (connection pool,
tokio-rusqlite, WAL multi-reader) — that's when the race becomes exploitable.
Also revisit if tests ever flake on invalidation-adjacent assertions.

## Not doing now

- BL-006 scope is clustering primitives, not concurrency hardening.
- The behavior is identical to pre-existing `search_vector`. Fixing it in
  isolation would create inconsistency.
- Single-threaded serialization makes it a latent rather than active bug.
