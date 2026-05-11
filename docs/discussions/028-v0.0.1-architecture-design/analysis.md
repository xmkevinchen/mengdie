---
id: "028"
title: "Analysis: v0.0.1 architecture design"
type: analysis
created: 2026-04-27
tags: [v0.0.1, architecture, module-layering, trait-boundaries, schema, instrumentation]
---

# Analysis: v0.0.1 architecture design

## Question

Validate or correct the proposed v0.0.1 architecture: 6 layers
(Storage / Ingestion / Retrieval / Reflection / LLM Provider / External
Interface) + cross-cutting; 6 trait abstractions (`Storage`,
`LlmProvider`, `EmbeddingProvider`, `Reflector`, `Transport`,
`EventEmitter`); 4 new modules / extensions (Instrumentation,
bi-temporal schema, bidirectional update, AE Round-0 caller);
4 specific open decisions. Architecture sits between blueprint
(`docs/blueprint.md`) and per-feature plans.

## Findings

### Prior Art from Project Knowledge Base

Prior context: unavailable (memory_search MCP tool not registered in
this session).

### Empirical dependency graph

archaeologist traced `use crate::core::*` and `use super::*` across
all of `src/core/` + `src/bin/`. Key results:

**Confirmed corrections to TL's proposed layering:**

- `decay.rs` has zero DB imports. It is consumed by `search.rs`
  (Retrieval) and `dreaming.rs` (Reflection). It is pure math, not
  Storage. **Cross-cutting placement is correct, not Storage.**
- `embeddings.rs` is imported by 5 distinct layers (Ingestion,
  Retrieval, Reflection, External Interface). **Cross-cutting infra,
  not Retrieval-specific.**
- `contradiction.rs` is imported by `ingest.rs:5`, not by any
  reflection-layer module. Contradiction detection runs synchronously
  inside `ingest_document` (`ingest.rs:39-49`). **Belongs in
  Ingestion, not Reflection.**

**Single layer violation in v0.x:**

- `ingest.rs:5: use super::contradiction::Conflict;` — Ingestion
  imports Reflection (upward dependency). Fixed by relocating
  contradiction to Ingestion layer (the relocation is a relabeling,
  not a code move — the file stays where it is).

**Two new structural findings:**

1. **`search.rs` functions are `impl Db` extension methods**
   (`search.rs:80`). `mcp_tools.rs` calls `self.db.memory_search(...)`,
   not `search::memory_search(&db, ...)`. The Retrieval layer is
   invisible at the type level — search is grafted onto Storage's
   method surface. To establish Retrieval as a real boundary, search
   functions need to move to module-level API.
2. **`mcp_tools.rs:306-331` reimplements the ingest pipeline inline**
   rather than calling `ingest::ingest_document`. CLI / watcher use
   one ingest path (with content-hash dedup); MCP `memory_ingest`
   uses a different inline path. **Two ingest code paths with
   different behavior** — a defect to fix in v0.0.1 regardless of
   layer choices.

No cycles found. The directed graph is a DAG.

### Architecture review

architecture-reviewer evaluated layering, traits, dependency
direction, and the 4 open decisions:

**Layer model: ACCEPT with two corrections** (decay → cross-cutting,
contradiction → Ingestion). Everything else holds: parser /
ingest / watcher in Ingestion; embeddings / vector / search in
Retrieval (note: embeddings reclassified to cross-cutting per
archaeologist); clustering / synthesis / dreaming in Reflection;
mcp_tools / mcp_server / cli in External Interface.

**Trait abstractions:**

| Trait | Verdict | Reasoning |
|---|---|---|
| `Storage` | ACCEPT — conditional | Must split search out (FTS5 specifics leak otherwise); Tier 2 Kuzu impl needs clean CRUD-only surface |
| `LlmProvider` | ACCEPT | Earns trait through near-future second impl (Codex / oMLX) |
| `EmbeddingProvider` | ACCEPT | Formalizes existing `Embed` trait; MockEmbedder is real second impl (`ingest.rs:96-121`) |
| `Reflector` | REDESIGN | Trait conflates "what reflection does" (action) with "when it runs" (trigger). Separate concerns. |
| `Transport` | REJECT (premature) | One impl (rmcp); no second committed |
| `EventEmitter` | REJECT (premature) | Rebuilds `tracing` crate; use module + concrete functions |

**Open decisions:** delete persisted metrics; demote watcher to
opt-in; bidirectional update SYNCHRONOUS for v0.0.1 (cluster sizes
small, ~10–50ms cost); Reflector trigger = count threshold (N=20)
+ 30-day time escape (SCM composite is P2).

**Sharpest opinion:** "The architecture is solving the right problem
in the wrong order." Wire AE plugin Round-0 first; library /
schema work follows from confirmed loop closure.

### Industry pattern check (cross-family)

codex-proxy via Codex MCP confirmed:

- 6 layers is unremarkable for ~10k LoC Rust projects. Public memory
  systems (Mem0, LangMem, Graphiti) converge on 5–7 conceptual
  layers. **Not over-engineered.**
- 6 traits is the sensitive question. **`LlmProvider` +
  `EmbeddingProvider` "earn themselves fast"** (vendor churn is
  real). `Storage` defensible if narrow + repository-shaped.
  `Transport` should not exist as trait at v0.0.1.
  `Reflector` and `EventEmitter` premature unless a 2nd impl is
  forced quickly.
- Bidirectional update refinement: **split sync (durable parts:
  fact + provenance + embedding) + async (cluster re-evaluation).**
  Memory writes blocking user workflows is wrong.
- Domain audit persistence is necessary: search calls + facts
  returned + AE acknowledgement = loop-closure evidence. **Generic
  counters via tracing; domain audit persisted.**

**Risk identified:** confusing optionality with the core loop —
spending design energy on swappable reflection policies and
bidirectional update variants before the single non-negotiable piece
(Round-0 AE plugin integration + persisted audit trail) is
undeniable.

### Challenger filter (Phase 1 + Phase 2)

challenger applied YAGNI rule: "introduce a trait only when ≥2
concrete impls exist or are committed in the same sprint."

| Trait | YAGNI verdict | Final position |
|---|---|---|
| `LlmProvider` | ACCEPT (2nd impl committed: Codex / oMLX) | Convergent ACCEPT |
| `EmbeddingProvider` | ACCEPT (MockEmbedder qualifies as 2nd impl) | Convergent ACCEPT |
| `Storage` | PREMATURE unless Retrieval refactor is in-sprint | Conditional — see Disagreements |
| `Reflector` | PREMATURE | Convergent REJECT |
| `Transport` | PREMATURE | Convergent REJECT |
| `EventEmitter` | PREMATURE | Convergent REJECT |

**Phase 1 prediction updates:**

- "Reflection layer is artificial" — partially validated. After
  contradiction relocates to Ingestion, Reflection = {clustering,
  synthesis, dreaming}, where clustering and synthesis are exclusively
  called by dreaming. **Recommendation: collapse into single module
  (keep `dreaming.rs` name), DEFER until sqlite-vec spike resolves**
  (clustering may disappear entirely if ANN replaces it).

- **Bi-temporal split challenge stands.** For AE artifacts, event
  time ≈ ingest time (artifact is generated and ingested within
  seconds of pipeline completion). Graphiti's bi-temporal model is
  borrowed from a chat-derived use case where event_time and
  ingest_time genuinely differ. **Concrete falsifiable demand: show
  one AE artifact in production where event_time ≠ ingested_at by
  > 60 seconds. If none exists, the schema column is borrowed
  pattern with no actual payoff.**

- **A-MEM bidirectional update unchanged.** One NeurIPS 2025 paper,
  no independent replication. Backlog with trigger, not v0.0.1.

**Phase 2 position update on "do nothing on src/":** conceded.
Two-ingest-paths defect (archaeologist new finding) is a real bug
regardless of architecture. Updated minimum v0.0.1:

1. Wire AE plugin Round-0 injection (AE plugin BL — no mengdie src/ change)
2. Fix mcp_tools two-ingest-paths defect (consolidate to single path
   through `ingest::ingest_document`)
3. sqlite-vec compatibility spike (already in blueprint §10)
4. Ship

Everything else (Storage trait, bi-temporal column, A-MEM
bidirectional, Reflector trait, EventEmitter, Reflection collapse)
filed as backlog with explicit triggers.

### Convergence (3-of-4 or 4-of-4 agreement)

- **Wire AE Round-0 first; library / schema work follows.** All four
  agents.
- **Two-ingest-paths defect must be fixed in v0.0.1.** Surfaced by
  archaeologist; agreed by challenger; consistent with reviewers'
  position.
- **`Transport` trait — defer until 2nd transport.** All four.
- **`EventEmitter` trait — replace with `tracing` + AtomicU64 module.**
  All four.
- **`Reflector` trait — premature or redesign.** All four (reviewers
  said redesign; challenger said premature; same direction).
- **`LlmProvider` trait — ACCEPT.** All four.
- **`EmbeddingProvider` trait — ACCEPT.** All four (challenger
  conceded).
- **Layer corrections:** decay → cross-cutting; contradiction → Ingestion;
  embeddings → cross-cutting. All four.
- **Domain audit persistence.** codex-proxy + reviewers; challenger
  agrees this is the "loop-closure evidence" he was demanding.
- **Reflector trigger default:** count threshold + time escape (not
  full SCM composite). All four.
- **Push ingest as v0.0.1 default; watcher demoted to opt-in.** All four.
- **Persisted generic metrics counters: DELETE.** All four (`stats`
  CLI subcommand is the only consumer, scaffolding-grade).

### Genuine disagreements

Four points where the team did NOT converge:

**1. `Storage` trait introduction in v0.0.1**

- architecture-reviewer + codex-proxy: ACCEPT, conditional on search
  being split out of `impl Db` to module-level
- challenger: PREMATURE unless the search-split refactor is committed
  to the same v0.0.1 sprint

The disagreement is not on the trait's eventual desirability — all
agree it's the right shape for §7 ladder evolution. The disagreement
is on timing: define the trait now (reviewers) vs. wait until the
abstraction barrier is real (challenger). Resolution path: **if the
search-split refactor IS scoped into v0.0.1, define `Storage` trait
in the same change. If NOT scoped, defer the trait.** The `Storage`
trait introduction is conditional on the refactor.

**2. Reflection layer collapse**

- architecture-reviewer: keep three modules (implicit conservative
  default)
- challenger: collapse to single module (`dreaming.rs`); concrete
  reasoning — clustering and synthesis are exclusively called by
  dreaming, so the module boundary is fictional
- challenger pragmatic position: defer the collapse decision until
  sqlite-vec spike resolves, since clustering may disappear if ANN
  index replaces it

Resolution path: **defer Reflection collapse until sqlite-vec spike
completes.** If sqlite-vec adoption succeeds and ANN-based
similarity replaces hand-rolled clustering, the question is moot
(clustering.rs is deleted, not merged). If sqlite-vec is deferred,
revisit collapse decision in v0.0.1 sprint scoping.

**3. Bi-temporal `event_time` column**

- architecture-reviewer: ACCEPT (correctly placed in Storage,
  borrows Graphiti design pattern)
- challenger: DEFER until concrete evidence shows event_time differs
  from ingested_at in actual AE workflow

The challenger's demand is falsifiable: produce one AE artifact in
production where the gap is > 60 seconds. If none exists in v0.x's
214-fact production corpus, the column is dead schema borrowed from
a use case (chat-derived facts) that doesn't apply.

Resolution path: **operator answers the falsifiable demand.** If the
AE workflow has post-hoc documentation of past decisions, the column
is justified. If not, defer the column to a future trigger
(e.g., when post-hoc workflows are introduced).

**4. Bidirectional update timing (A-MEM pattern)**

- architecture-reviewer: synchronous in ingest path
- codex-proxy: split — sync durable parts + async cluster reeval
- challenger: defer entirely; one paper, no independent replication

Resolution path: **defer the entire feature to backlog with a
specific trigger.** Once the loop is closed (Round-0 wired) and
real usage data accumulates, decide whether the cluster-evolution
cost is worth paying. Codex's split design is the right shape if /
when it's adopted, but introducing it in v0.0.1 is premature.

## Summary

The v0.0.1 architecture proposal converged after team review, with
significant scope reduction relative to the TL's draft.

**v0.0.1 minimum sprint (4 items):**

1. Wire AE plugin Round-0 injection (AE plugin BL, not mengdie src/)
2. Fix `mcp_tools.rs` two-ingest-paths defect — consolidate to single
   path through `ingest::ingest_document` (mengdie src/ BL)
3. sqlite-vec compatibility spike (already filed in blueprint §10)
4. Ship and verify the loop closes (instrumentation: log every
   `memory_search` call, what was returned, what was used; persisted
   domain audit)

**Architectural changes that the minimum sprint INDUCES:**

- search.rs functions move from `impl Db` to module-level API
  (`search::memory_search(&db, ...)`). Required to fix two-ingest-
  paths defect cleanly; enables Retrieval as a real layer.
- decay.rs / embeddings.rs / contradiction.rs are relabeled per their
  actual usage (cross-cutting / cross-cutting / Ingestion). File
  locations may stay; layer documentation updated.
- `LlmProvider` trait introduced, conforming to `rig::CompletionModel`.
  ClaudeCliProvider remains as a mengdie-side impl. Codex / oMLX
  impls follow.
- `EmbeddingProvider` trait formalized from existing `Embed` trait.
- Persisted SQLite metrics counter table DELETED. Generic counters
  via `tracing` + AtomicU64. Domain audit (loop-closure events)
  persisted in a small dedicated table.

**Architectural changes DEFERRED to backlog with explicit triggers:**

- `Storage` trait — defer unless v0.0.1 includes search-split
  refactor (conditional)
- `Reflector` trait — defer until 2nd reflection strategy ships
- `Transport` trait — defer until 2nd transport materializes
- `EventEmitter` trait — defer or skip entirely; concrete `tracing`
  calls suffice
- bi-temporal `event_time` column — defer until concrete AE artifact
  with > 60s event-vs-ingest gap appears
- A-MEM bidirectional update — defer; trigger = corpus > 1k facts
  AND retrieval quality measurably degrading
- Reflection module collapse — defer until sqlite-vec spike resolves

**Single most important takeaway:** the architecture's value is
cumulative — wiring the loop (P0) is non-negotiable. Trait
abstractions, schema extensions, and module boundaries are P1+
that gain meaning only after the loop is closing and data is
flowing. v0.0.1 should ship the loop and the defect fix; everything
else evolves under triggered conditions.

## Possible Next Steps

→ `/ae:discuss docs/discussions/028-v0.0.1-architecture-design/`
  for the four genuine disagreements:
  1. Is search-split refactor in v0.0.1 scope? (Conditional on
     this, Storage trait ACCEPT or DEFER)
  2. Bi-temporal column — operator answers challenger's falsifiable
     demand
  3. Reflection collapse — defer or schedule
  4. A-MEM bidirectional — confirm defer with trigger

→ Or, write `docs/architecture.md` directly capturing the converged
  findings (layer model with corrections, 3 traits accepted, deferred
  items with triggers); operator decides on the 4 disagreements
  inline. Single document.

→ Then file three v0.0.1 BLs (AE Round-0 wiring in
  `agentic-engineering/`, two-ingest-paths defect fix in mengdie/,
  sqlite-vec compatibility spike) and one v0.0.1 BL for the
  search-split refactor if accepted as in-scope.
