---
reviewer: doodlestein-adversarial
type: post-conclusion-adversarial
scope: sprint-bl-implementability
---

# Adversarial Review — Discussion 028 Sprint BLs

## Question

Where does this conclusion first fail in real use — a constraint not surfaced,
a decision punted but actually load-bearing, an integration that doesn't work
as written?

## Finding: BL #1 (AE Round-0 Wiring) Is Already Partially Wired — Wrong Phase

The conclusion says BL #1 adds: "calls `memory_search` before research agent
spawn; Round-0 context block from results."

The implementer who reads this and then opens
`plugins/ae/skills/analyze/SKILL.md` will find that mengdie is **already
integrated** — but at the wrong phase. Lines 165–174 of SKILL.md call
`memory_search` *after all teammates have SendMessage'd their findings to TL,
before synthesis*. That is not Round-0. That is post-research injection.

The distinction matters: Round-0 means the context is presented to the TL
**before spawning the research agents**, so the agents can read it and avoid
re-discovering already-known facts. Post-research injection means agents have
already done their work without the prior context.

The conclusion does not surface this gap. It says "wire Round-0" as if the
call site does not exist, when in fact a call site exists at the wrong phase.

**Wall the implementer hits**: they read the BL, look at the skill, see
"mengdie integration already there" and declare done — without noticing the
injection happens after, not before, the agent research phase. The bug
survives the sprint with no one catching it because the mechanism
(memory_search called in ae:analyze) is structurally present.

Alternatively, an implementer who does notice the phase mismatch has no spec
for what "before spawning agents" means operationally: does the context go
into the TL's initial prompt? Into a Round-0 prefix block visible to spawned
agents? Into the `prompt:` parameter of each Agent call? The conclusion is
silent on the wire format.

## Finding: BL #5 (Audit Table) Has No Schema

The conclusion says: "log query, scope, returned fact IDs, took_ms" per
`memory_search` call. It calls this P0 instrumentation. It does not specify
the table schema.

This sounds like a small omission until the implementer opens `schema.rs` and
sees migration v5 adding `synthesis_cluster_hash` with a full pre-check
sequence, backfill loop, idempotence guard, and safety integrity check. The
pattern in this codebase is: every schema change is wrapped in a versioned
migration, and every migration has a pre-check + backfill + version bump.

The audit table needs a name, column types, index strategy, and a migration
number (v6). None of this is in the conclusion or the BL description. The
implementer must answer: is `returned_fact_ids` a JSON array column, a joined
text column (comma-separated IDs matching `entities` pattern), or a separate
link table (like `memory_synthesis_links`)? The schema choice is
non-trivial:

- Comma-separated text: works for point queries, breaks for join-based
  supersession rate computation (the one use case the audit table exists for).
- JSON column: parseable in Rust, not directly joinable in SQL without
  json_each (available in SQLite ≥ 3.38.0 — the bundled rusqlite version is
  unspecified in Cargo.toml's feature flags; compatibility unknown).
- Separate link table (like `memory_synthesis_links`): correct for the join
  query, doubles the write path, adds FK pressure.

The conclusion's Topic 4 trigger computation ("≥5 superseded-within-7-days
events per rolling 30-day window from audit table") requires a join between
audit rows and `memory_entries.valid_until`. That join is only correct if
`returned_fact_ids` is joinable, which means the comma-text encoding is wrong
and the link-table encoding is right. But this was never stated.

**Wall the implementer hits**: arrives at the schema decision with no
guidance, picks comma-text (simplest, matches `entities` pattern already in
the code), and builds a trigger computation that cannot actually join. The
A-MEM trigger is broken before it starts because the audit table's schema
makes the join infeasible.

## Finding: BL #3 (search.rs Free-Functions Refactor) — Call Site Count Is Hidden

The conclusion says move `impl Db { fn memory_search() }` to module-level
`search::memory_search(&db, ...)`. This looks like a straightforward refactor.

The implementer opens `search.rs` and sees: `memory_search` is on `impl Db`
(line 152), `search_fts` is also on `impl Db` (line 83). Both are called
from `mcp_tools.rs` (lines 211 and 223 respectively). But `search_fts` is
also called from within `memory_search` itself (line 163 — `self.search_fts`).

After the refactor, `search::memory_search(&db, ...)` internally calls
`search::search_fts(&db, ...)`. That's fine. But `mcp_tools.rs` line 223
calls `self.db.search_fts(...)` in the FTS-only fallback path. After the
refactor this becomes `search::search_fts(&self.db, ...)`. That's also fine.

The hidden problem: `search_vector` is called at line 164 (`self.search_vector`)
and is defined in `vector.rs` via `impl Db`. After the refactor, is
`search_vector` also moved to a free function? The conclusion only mentions
`memory_search`. If `search_vector` stays on `impl Db`, the refactored
`search::memory_search` takes `&Db` and calls `db.search_vector(...)` — which
still works. But the refactor's stated goal ("Establish Retrieval as a real
layer at the type level") is only half-achieved: two of three search entry
points are free functions, one is still a method.

**Wall the implementer hits**: not a hard blocker, but the scope boundary is
ambiguous. A careful implementer asks "does `search_vector` move too?" and has
no answer in the conclusion. They either under-scope (leave `search_vector`
behind, producing the half-finished layer the conclusion claims to avoid) or
over-scope (move `search_vector` too, touching `vector.rs` which is not
mentioned in any BL).

## Summary

The first concrete pain point in implementation order is **BL #1's phase
ambiguity**: the AE plugin already has a mengdie call site but at the wrong
phase, and the correct wire format for a true Round-0 injection (before agent
spawn) is unspecified. An implementer who misses this ships a no-op BL.

The second is **BL #5's missing schema**: the audit table's column encoding
must be decided before the table is created, and the one query that justifies
the table's existence (the A-MEM trigger join) can only be served by a link
table, not a scalar column — a design choice the conclusion never surfaces.

These are not reopen signals. They are BL-filing inputs: each BL needs a
narrower spec before an implementer can start without hitting a wall.
