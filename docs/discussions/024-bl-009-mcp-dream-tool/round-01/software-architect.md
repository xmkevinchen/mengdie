---
agent: software-architect
round: 1
topic: 01
created: 2026-04-27
---

# Round 1 — software-architect

Lens: system shape, dependency direction, abstraction-level
boundaries, and what BL-009's choice forecloses for BL-010+.

## Findings (with file:line evidence)

### F1. The current synthesis pipeline already separates the four
responsibilities BL-009 needs to redistribute.

`run_synthesis_pass` (`src/core/dreaming.rs:399`) is a thin orchestrator
that calls four pure-ish layers:

1. **Cluster selection** — `cluster_memories(...)` at
   `dreaming.rs:408`. Pure function over DB state; no LLM dependency.
2. **Prompt assembly** — `build_synthesis_prompt(&input)` at
   `dreaming.rs:477`, defined in `synthesis.rs:50`. Takes
   `SynthesisInput { cluster_memories, cluster_centroid, project_id }`,
   returns `(system, user)` Strings. Truncation at
   `CONTENT_CHAR_LIMIT = 4000` (`synthesis.rs:7`) is enforced HERE.
3. **LLM dispatch** — `provider.complete(&system, &user).await` at
   `dreaming.rs:493`. Abstracted behind `LlmProvider` trait
   (`llm.rs:180`).
4. **Response parsing + persistence** — `parse_synthesis_response`
   (`synthesis.rs:92`) returns `SynthesisOutcome::Synthesized | Skipped`,
   then `db.insert_synthesis_with_links(new_mem, &draft.source_memory_ids)`
   at `dreaming.rs:574` writes the row + link table atomically.

This is already the right shape. BL-009's structural question reduces
to: **which subset of (1)+(2)+(3)+(4) does mengdie own, and which does
the host Claude get?** Not "redesign synthesis" — "re-route step 3".

### F2. The cluster-hash invariant has THREE potential enforcement
layers; only one of them is structural under future writers.

- **App-level** (current): the only caller of
  `insert_synthesis_with_links` is `run_synthesis_pass`
  (`dreaming.rs:574`). The call site computes `source_memory_ids` from
  the cluster the LLM was asked about, so the hash is correct by
  construction. **Fragile**: any new caller can write a
  `source_type = 'synthesis'` row through `insert_memory` and bypass the
  hash entirely.
- **DB-level partial unique index** (already shipped, plan 017,
  `db.rs:372`): `ON CONFLICT(project_id, synthesis_cluster_hash) WHERE
  source_type = 'synthesis' AND synthesis_cluster_hash IS NOT NULL DO
  UPDATE`. This enforces *uniqueness* per (project, cluster_hash) but
  the partial-index `WHERE` clause means a synthesis row with
  `synthesis_cluster_hash IS NULL` is silently allowed. v0.8.5
  BL-synthesis-cluster-hash-not-null-enforcement closes this gap
  (`docs/discussions/023-v0.8.5-scope-decision/conclusion.md:107`).
- **MCP-tool-level**: a new `memory_dream` commit handler that
  recomputes the hash from cluster member IDs the host Claude got from
  step (1) is *equivalent* to current app-level enforcement — it just
  moves the trust boundary outward. Not stronger than DB-level NOT NULL.

**Conclusion**: the strongest guarantee comes from the v0.8.5 DB-layer
NOT NULL — because it covers ALL writers including future
`LlmProvider` impls, direct SQL in tests, and BL-010's daemon. BL-009
should NOT try to re-implement enforcement at MCP-tool layer; it
should ride on v0.8.5's NOT NULL.

### F3. `memory_ingest` currently CANNOT write a synthesis row that
satisfies the cluster-hash invariant, even after v0.8.5.

`MengdieServer::ingest` (`mcp_tools.rs:282`) calls `db.insert_memory`
or `db.insert_memory_resolving` (`mcp_tools.rs:381-385`). Neither path
passes `source_memory_ids` and neither computes a
`synthesis_cluster_hash`. The `SourceType::Synthesis` enum variant is
accepted at the type level (`mcp_tools.rs:46`) but a synthesis ingest
through this path produces a row with
`synthesis_cluster_hash IS NULL`. Today the partial-unique-index's
`IS NOT NULL` clause silently lets it through. **After v0.8.5 NOT
NULL lands, today's `memory_ingest` writes with
`source_type=synthesis` will start failing** unless the call site is
extended with cluster-link semantics.

This is the structural fact that makes the
"extend `memory_ingest` with synthesis-aware shape" option non-trivial:
it's not pure ergonomics — it's the only way the existing tool
coexists with the v0.8.5 invariant.

### F4. rmcp v1.3 with `["server", "macros", "transport-io"]` does
NOT expose MCP sampling — the host-Claude reverse call mechanism.

`Cargo.toml:8` enables only `server`, `macros`, `transport-io`. MCP's
`sampling/createMessage` request (server → client, asks the host LLM
to complete a prompt) requires the client side of the protocol. A
"new `LlmProvider` impl that reverse-calls Claude in-session" requires
either a sampling feature in rmcp (not enabled, possibly not present
in 1.3 — needs verification) or a different mechanism. **Therefore**
the option "make BL-009 a McpSessionProvider behind the existing
LlmProvider trait" is not free — it depends on either an rmcp feature
flag flip or a protocol-level workaround. This rules out the
cleanest architectural option (LlmProvider impl, zero MCP-surface
change) unless verified.

### F5. The current path is single-process synchronous; BL-010 will
need it asynchronous, but the **interface** between cluster-selection
and synthesis-commit is already trivially queueable.

`insert_synthesis_with_links` takes
`(NewMemory, &[String /* source_ids */])` (`db.rs:349`). That tuple is
verbatim what a daemon would dequeue from a `pending_jobs` row. The
*caller* of the LLM doesn't need to be in the same process as the
caller of the DB write — only the cluster_hash must be computed from
the same source_ids that go into the link rows, and that's enforced
by passing them as one tuple. **Implication**: any BL-009 shape that
produces "(synthesis_text, source_memory_ids)" as a logical artifact —
regardless of whether the host Claude or a CLI subprocess produced
the text — is BL-010-compatible. The shape that breaks BL-010 is
"Claude does it all in one tool call, and the tool returns a
done-flag" — because that pattern owns the LLM dispatch inside the
tool handler and is not pause-resumable.

### F6. Plan 010 first-caller pattern: BL-009 pressure-tests "cluster
selection as a pure read-only primitive" and "cluster-hash as the
single dedup key under multi-writer".

Plan 010 (`docs/plans/010-dream-synthesis.md`) was the first caller of
the LlmProvider trait + clustering primitive. It validated that the
trait could carry a real workload. BL-009 is the first caller of:
- **Cluster selection without persistence** — until now, every caller
  of `cluster_memories` has been internal to `run_synthesis_pass`,
  which immediately consumes the result. BL-009 (in any shape that
  returns clusters to the host) externalizes them. The bet:
  cluster IDs are stable enough between a tool-list call and a
  tool-commit call that the host Claude can round-trip through them.
  **Empirical risk**: between read-clusters and commit-synthesis,
  another writer could `memory_invalidate` one of the source rows.
  The cluster-hash will still match (it's computed from IDs), but the
  link table will point at an invalidated row. Acceptable
  (graceful-degradation on `get_synthesis_with_sources` per
  `db.rs:425`), but worth surfacing.
- **Multi-writer cluster-hash dedup** — until now the only writer is
  `run_synthesis_pass`. BL-009 adds a second writer
  (host-Claude-driven). v0.8.5 NOT NULL is the structural
  precondition; without it, the second writer can produce zombie
  syntheses that the partial unique index silently accepts.

## Mechanism options + trade-offs

Five shapes, mapped to the responsibility split:

| Shape | (1) cluster | (2) prompt | (3) LLM | (4) parse + persist | BL-010 compat | v0.8.5 dep |
|---|---|---|---|---|---|---|
| **A. Single round-trip tool** (`memory_dream` returns clusters; Claude calls `memory_ingest` with synthesis) | mengdie | host Claude | host Claude | mengdie via extended `memory_ingest` | ✓ if commit goes through queueable path | requires `memory_ingest` synthesis-aware extension OR new commit path |
| **B. Two-tool (read clusters / commit synthesis)** | mengdie | host Claude | host Claude | mengdie via dedicated commit tool | ✓ — split mirrors daemon enqueue/dequeue | clean — commit tool owns hash computation |
| **C. Stateful session** (tool returns cursor, host streams syntheses) | mengdie | host | host | mengdie | ✗ — stateful tool handler precludes daemon | independent |
| **D. Extend `memory_ingest`** (add `cluster_source_ids` field; tool computes hash on synthesis writes) | host has to know cluster ahead of time | host | host | mengdie | ~ — works but doesn't expose clusters | hard requirement |
| **E. McpSessionProvider** (new LlmProvider impl, reuse `run_synthesis_pass`) | mengdie | mengdie | mengdie via reverse-call | mengdie | ✗ — owning the LLM dispatch inside the tool handler is the BL-010-incompatible shape | requires rmcp sampling feature (not enabled per F4) |

**Architectural read**:
- E is the "cleanest" by the LlmProvider abstraction it would slot
  into — but it (a) requires rmcp sampling support and (b) repeats the
  BL-010-incompatible shape (LLM dispatch inside the tool handler).
- C (stateful session) is BL-010-incompatible *and* hostile to
  rmcp v1.3 stdio transport (long-lived state across tool calls
  is not what stdio MCP optimizes for).
- B (two-tool) maps directly onto BL-010's queue model: read-clusters
  = enqueue payload; commit-synthesis = consume completed job.
  Discussion 008's no-new-MCP-tool precedent argues against, but B is
  precisely the case where "one tool that does both reads and writes"
  is wrong — the read and the write are separated by an
  unbounded-latency LLM step.
- A is B with the commit folded into `memory_ingest`. Cleaner tool
  count; muddier `memory_ingest` semantics
  (`memory_ingest` becomes "ingest a primary memory OR commit a
  synthesis").
- D is A minus the cluster-read tool — assumes the host already knows
  which cluster to ingest, which doesn't compose with mengdie owning
  cluster-selection.

## Phase 2 chain compatibility

The chain is BL-009 → BL-010 (daemon) → BL-011 / BL-013
(`framing.md:57`, `analysis.md:78` in disc 023). BL-010 introduces a
`pending_jobs` table that the daemon polls. The structural test:
**when BL-010 lands, can the BL-009 path be re-routed through a job
row without changing the tool surface seen by the host Claude?**

- Shape A passes the test if `memory_ingest` (when called with
  synthesis intent) writes either a row or a job, configurable.
- Shape B passes the test trivially: read-clusters can enqueue an
  "extract" job that mirrors the same payload; commit-synthesis can
  be replaced by a daemon that consumes the job.
- Shape C and E fail the test — both put the LLM dispatch inside the
  tool handler's request lifetime.

## Cluster-hash invariant: where it BEST lives

**DB layer.** Per F2, this is the only layer that covers all
writers. BL-009 should not re-enforce; it should produce
`source_memory_ids` and let the existing
`insert_synthesis_with_links` (`db.rs:349`) compute the hash. Any
shape that bypasses `insert_synthesis_with_links` (e.g., extending
`memory_ingest` to write synthesis rows via `insert_memory` + a
side-table call) re-introduces the bypass risk that v0.8.5 is closing.

The architectural rule: **`insert_synthesis_with_links` is the only
synthesis writer.** Whatever BL-009 ships must call it (directly or
through a thin handler) and must NOT add a parallel synthesis-write
path.

## Agreements

(none yet — Round 1 independent research)

## Disagreements

(none yet — Round 1 independent research)

## Open Questions

1. **Does rmcp v1.3 expose MCP sampling at all** (with or without a
   feature flag we haven't enabled)? If yes, option E becomes
   live and the trade-off shifts. If no, the architectural answer is
   forced toward A or B.
2. **Is the "whether" answer "yes" given rmcp's sampling status?**
   If sampling is not available, the user-visible win of BL-009 is
   limited to "Claude in-session can drive synthesis without forking
   a subprocess" — which is real (latency, env reuse, no
   `--no-session-persistence` token spend) but smaller than the
   "the host IS the LLM, eliminate the indirection" framing
   suggests.
3. **A vs. B: should the read-clusters output be addressable
   independently of the commit?** Two-tool (B) lets the host Claude
   make multiple commits per read (synthesize multiple clusters in
   one read pass) and lets BL-010 drop in cleanly. Single-tool (A)
   forces a 1:1 read↔commit pairing inside one tool sequence.
   Empirical: how many clusters does a typical dream pass surface?
   First real run produced 13 syntheses
   (`CLAUDE.md` "Project Status"). That's >>1, suggesting B's
   independence is worth the second tool.
4. **Should `memory_ingest`'s `Synthesis` source_type variant be
   removed** if BL-009 ships a separate commit tool? Today it
   already produces zombie rows
   (`synthesis_cluster_hash IS NULL`); after v0.8.5 NOT NULL it will
   start failing at the DB layer. Either way it's a footgun. Removing
   the variant is a separate (small) plan; flagging here so the
   discussion doesn't leave it dangling.
5. **Does the cluster IDs returned to host need a TTL or signing
   token** to prevent the host Claude from committing a synthesis
   against an unrelated cluster? Probably no for MVP (host is
   trusted, single-tenant), but flagging — this is the trust-boundary
   that LlmProvider abstraction hides today.
