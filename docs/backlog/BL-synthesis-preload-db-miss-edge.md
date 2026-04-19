---
id: BL-synthesis-preload-db-miss-edge
status: open
origin: plan 012 /ae:review (challenger claim C)
created: 2026-04-19
scope: mengdie (theoretical edge — denominator inflation under DB-load miss)
---

# Pre-DB-load `pair_clusters_processed` inflates when DB load returns fewer rows than expected

## Finding

`run_synthesis_pass` in `src/core/dreaming.rs` increments
`result.pair_clusters_processed` at the pre-DB-load site
(`trimmed_ids.len() == 2`) for consistency with `clusters_processed`.
Subsequently, `db.get_memories_by_ids(&trimmed_ids)` is called. If the DB
returns fewer memories than expected (e.g., a memory was deleted between
clustering and synthesis, or an ID corruption), the post-load guard
`if memories.len() < min_size { continue; }` bails out.

On that bail-out path, the cluster was already counted in the denominator
(`pair_clusters_processed += 1`) but will NEVER increment the numerator
(`pair_clusters_skipped += 1` only fires on the Skipped LLM branch, reached
only after `memories.len() >= min_size`). The displayed pair-cluster skip
percentage becomes understated by one bin per DB-load miss.

## Why not fixed in plan 012

Astronomically unlikely in current production:
1. `cluster_memories` loads embeddings from the SAME DB that
   `get_memories_by_ids` reads; no concurrent writers.
2. IDs round-trip from the clustering phase; no external input.
3. Failure mode is effectively "memory vanished between adjacent DB reads" —
   requires data corruption or a concurrent `delete` path.

Fixing requires choosing between:
- **Option A**: Post-load attribution — increment `pair_clusters_processed`
  after `memories.len() >= min_size` confirmed. Matches numerator
  attribution. Breaks the "consistent with clusters_processed" invariant
  from plan 011 architect review — but that invariant was named to address
  a different concern (plan 011 was about trimming, not DB misses).
- **Option B**: Decrement `pair_clusters_processed` on the bail-out path.
  Adds a specific compensation; keeps the pre-load attribution elsewhere.
- **Option C**: Track DB-miss separately (`pair_cluster_db_misses: usize`)
  and subtract from the denominator at CLI-display time.

## Trigger

Fires when ANY of:
1. A `mengdie delete` / `memory_invalidate` subcommand lands that could run
   concurrently with `dream --synthesize`.
2. A real-world run observes a `pair_clusters_processed` count that does
   NOT equal `pair_clusters_skipped + syntheses_created` for its
   pair-cluster subset — that arithmetic mismatch is the observable signal.
3. The DB schema gains tombstone semantics where clustering can surface
   IDs for rows a downstream load silently skips.

Until any of these trigger, the edge is documented in
`src/core/dreaming.rs` (attribution-invariant docstring on
`run_synthesis_pass`) but not fixed.

## Fix direction (Option B — minimum-surprise)

```rust
let memories = db.get_memories_by_ids(&trimmed_ids)?;
if memories.len() < min_size {
    if is_pair_cluster {
        // Compensate for the pre-load bump on the bail-out path.
        result.pair_clusters_processed =
            result.pair_clusters_processed.saturating_sub(1);
    }
    tracing::warn!(...);
    continue;
}
```

Adds 4 lines, preserves pre-load attribution where it matters (the common
path), and closes the denominator-inflation edge without introducing a
third counter field.

## Why filed as backlog rather than fixed now

Plan 012 was scoped to the CLI-display bug (100% → 27%) observed in
production. This edge is a theoretical extension that has never been
observed. Filing keeps the decision visible for the first real occurrence.
