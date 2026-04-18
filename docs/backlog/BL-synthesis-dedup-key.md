---
id: BL-synthesis-dedup-key
status: open
origin: BL-007 /ae:review (cross-family-fallback #4)
created: 2026-04-18
---

# Synthesis row dedup key is `content_hash` — unstable under prompt evolution

## Finding

`Db::insert_synthesis_with_links` uses
`ON CONFLICT(project_id, content_hash) DO UPDATE` as the dedup mechanism
for synthesis rows. `content_hash` is computed over the LLM's generated
synthesis text.

**Failure mode**: any deliberate improvement to `SYSTEM_PROMPT`
(`src/core/synthesis.rs:5`) changes the LLM's output. The new output
produces a new `content_hash`. The old synthesis row does NOT get
superseded — it simply coexists with the new one.

After N prompt iterations, the same input cluster has N synthesis rows.
All of them appear in search results (same source_type, same FTS/vector
surface).

The regression guard in `src/core/synthesis.rs:147`
(`EXPECTED_SYSTEM_PROMPT`) prevents accidental drift from breaking tests
but does not prevent intentional prompt improvement from leaving
zombie synthesis rows in the DB.

## Trigger

Fires when either:
- `SYSTEM_PROMPT` is intentionally edited (first `cargo test` run after
  the edit, the regression test will fail and `EXPECTED_SYSTEM_PROMPT`
  must be updated — that's the signal this backlog item should be
  addressed in the same plan), OR
- An operator observes duplicate syntheses in `mengdie search` output
  after re-running `mengdie dream --synthesize`, OR
- Cluster count × prompt iterations > ~N synthesis rows (operator
  judgment).

## Fix options

**Option A (preferred, minimal)**: Change the dedup key to
`(project_id, sorted-hash-of-source-ids)`. Add a
`synthesis_cluster_hash TEXT` column in a v5 migration. Compute it as
`sha256(source_ids.sort().join(","))` at insert time. Enforce uniqueness
via an index on `(project_id, synthesis_cluster_hash) WHERE
source_type = 'synthesis'`. Any re-synthesis of the same cluster
updates in place regardless of content text.

**Option B (heavy)**: On prompt-change detection, emit a one-off
`mengdie synthesis rebuild` command that invalidates all existing
synthesis rows (sets `valid_until = now`) and re-runs the pass. Keeps
content_hash as the dedup key; trades one-liner dedup for explicit
operator action on prompt upgrades.

**Option C (lazy)**: Do nothing. Accept zombie rows. Operators manually
call `memory_invalidate` on stale syntheses they no longer want. Lowest
effort; highest long-term cruft.

Prefer A. B is defensible if prompts stabilize and edits are rare. C is
not defensible once the corpus exceeds ~500 memories.

## Why not fixed in BL-007

Requires a schema migration (v5) and changes to `insert_synthesis_with_links`
semantics. Out of scope for the MVP, and the failure mode only manifests
after a prompt edit — which has not happened yet.
