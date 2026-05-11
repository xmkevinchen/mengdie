---
id: "028"
title: "v0.0.1 architecture design — Conclusion"
concluded: 2026-04-28
plan: ""
entities: [v0.0.1, architecture, storage, search-split, free-functions, storage-trait, bi-temporal-column, event-time, reflection-collapse, reflector-trait, a-mem-trigger, bidirectional-update, mcp-ack, returned-fact-ids]
---

# v0.0.1 architecture design — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Storage trait + search-split refactor scope | **Search-split refactor IN v0.0.1** (alongside `mcp_tools.rs` two-ingest-paths defect fix). search.rs functions move from `impl Db { fn memory_search() }` to module-level `search::memory_search(&db, ...)`. **Storage trait NOT introduced** in v0.0.1; mechanism = free functions over `&Db`. Trait deferred to Tier 2 trigger (Kuzu adoption when 2nd Storage impl exists). | 4-of-5 majority. arch-reviewer's "decisions are independent" argument decisive. minimal-change-engineer's Rust nominal-typing cost asymmetry refutes gemini's Go-style "design interfaces early" reasoning. challenger's YAGNI rule satisfied (1 impl, no committed 2nd in-sprint). | high — free functions can be wrapped in trait later when 2nd impl materializes; no data migration cost |
| 2 | Bi-temporal `event_time` column | **NOT in v0.0.1 schema.** Single `ingested_at` is the only timestamp. Alternative for bulk historical import: extend `memory_ingest` to accept optional `valid_from` parameter (default = current time). **Re-open path**: a new discussion when a batch-import workflow ships that ingests artifacts whose decision-time differs from ingest-time by minutes-to-weeks (e.g., AE plugin imports old plan / meeting-note files). Not "permanently rejected" — narrowly rejected for v0.0.1's currently-modeled real-time AE pipeline workflow. | 4-of-5 majority. minimal-change-engineer's chicken-and-egg argument decisive: codex's DEFER-with-trigger formulation (>60s gap auto-fire) requires the column to exist before it can fire — operationally identical to "not in v0.0.1" but worse governance. Per blueprint §6: do not borrow patterns whose payoff is not demonstrable. | medium — adding column later is SQLite ALTER TABLE; back-fill `event_time = ingested_at` for existing rows; non-zero migration cost |
| 3 | Reflection module consolidation + Reflector trait introduction | **Defer Reflection module consolidation** (`clustering.rs` + `synthesis.rs` + `dreaming.rs`) until sqlite-vec compatibility spike outcome is known. **Do NOT introduce `Reflector` trait** in v0.0.1, regardless of sqlite-vec spike outcome. ANN-based clustering is similarity-primitive swap inside one strategy, not a 2nd reflection strategy. | 5-of-5 unanimous + UAG passed. Falsification attempt ("name a v0.0.1 call site selecting between ≥2 reflection strategies at runtime") was unanswered by any agent. ANN swap doesn't change algorithm identity; trait abstracts strategies, not primitives. | high — both sub-decisions reversible; module consolidation can happen post-spike if `clustering.rs` survives; trait can be introduced when 2nd reflection strategy materializes |
| 4 | A-MEM bidirectional update deferral trigger | **A-MEM deferred from v0.0.1.** Trigger fires when ALL hold: (a) corpus ≥1,000 facts (operator may calibrate down to 500), (b) ≥5 superseded-within-7-days events per rolling 30-day window from the persisted domain audit table. **Integration requirement** (P0 v0.0.1 instrumentation): the persisted domain audit table MUST log `returned_fact_ids` per `memory_search` call so the supersession signal is computable via join. | 5-of-5 agreed deferral; 5-of-5 converged on "corpus floor + audit-log supersession signal" shape after Round 2. Numeric specifics (corpus 500 vs 1k, supersession window 7d vs 14d) are operator calibration parameters, not architectural decisions. | high — A-MEM is deferred work, not committed v0.0.1 work; trigger fires automatically when conditions hold |

## Meta-decision recorded

**MCP `memory_search` ACK feedback channel — NO in v0.0.1 contract.**
challenger's argument (Round 2): the "used" signal is ambiguous — an
AI that reads and discards facts by exclusion has still "used"
them. Contractual burden on every integrator is not worth a noisy
precision estimate. All Topic 4 triggers must be server-side
observable from the persisted domain audit table.

This decision propagates: Topic 4 trigger uses supersession events
(observable from existing schema's `valid_until` + `superseded_by`
fields on returned facts) rather than caller acknowledgment.

## Doodlestein Review

Three post-conclusion reviewers (`strategic`, `adversarial`, `regret`)
audited the written conclusion. All three findings are valid; all
three are TL-absorbable operational/specification refinements rather
than challenges to architectural decisions. No new council round
fired. Verdicts and dispositions:

| Reviewer | Finding | Disposition |
|---|---|---|
| `regret-post` | Topic 2 "REJECT permanently" framing is too strong. The chicken-and-egg argument refutes codex's specific self-referential trigger but does not refute the column's utility under a different entry path (operator-initiated batch import where artifact-time vs ingest-time gap is externally observable). Highest-regret-probability decision. | **Absorbed** — Topic 2 row in Decision Summary softened to "NOT in v0.0.1 schema; re-open path = new discussion when batch-import workflow ships." Decision content unchanged (already preserved re-open via new discussion); language inflation removed. |
| `strategic-post` | The 5-BL Next Steps list is flat but contains 3 hidden coupling constraints: (1) BL #2 + #3 are co-committed (same mcp_tools↔search boundary); (2) BL #4 sqlite-vec spike needs an explicit PASS/FAIL outcome record as exit criterion or the dependent Reflection consolidation BL files against unresolved gate (phantom-active failure mode); (3) BL #5 audit table is P0 prerequisite for A-MEM trigger and should be Wave 1, not last. | **Absorbed** — Next Steps section restructured into 2 waves with co-commit annotation, spike outcome record as explicit acceptance criterion, audit table promoted to Wave 1. |
| `adversarial-post` | BL #1 (AE Round-0) has phase ambiguity — existing `ae:analyze` SKILL.md `memory_search` call is post-research; blueprint's Round-0 injection requires pre-spawn. BL #5 (audit table) needs explicit schema (link table vs comma-separated text); only link table supports the Topic 4 supersession join. BL #3 (search.rs free-fn) doesn't specify whether `search_vector` is in scope — leaving it as `impl Db` half-achieves "Retrieval as real layer." | **Absorbed** — Next Steps adds acceptance criteria annotations on BL #1 (pre-spawn phase + wire format), BL #5 (link-table schema with foreign key to `memory_entries.id`), and BL #3 (`search_vector` in scope alongside `memory_search`). |

None of the findings challenged a converged decision. Each was a
specification or framing refinement absorbable by TL without
re-running the council. Dissents in the original Decision Summary
(gemini on Topic 1, codex on Topic 2) are unaffected.

## Spawned Discussions

None. All four topics resolved within this discussion.

## Deferred Resolutions

None. Sweep was empty (no `revisit` or `deferred` items at Round 2
close).

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude (this session) | Start |
| architecture-reviewer | architectural lens | Claude (`ae:review:architecture-reviewer`) | Round 1 |
| minimal-change-engineer | YAGNI / anti-over-engineering | Claude (`engineering-minimal-change-engineer`) | Round 1 |
| challenger | pure opposition | Claude (`ae:workflow:challenger`) | Round 1 |
| codex-proxy | OpenAI family cross-lens | Codex MCP | Round 1 |
| gemini-proxy | Google family cross-lens | Gemini MCP | Round 1 |
| (Doodlestein × 3) | post-conclusion review | Claude | Step 9 |

## Process Metadata

- Discussion rounds: 2
- Round 0 framing review: 1 rerun (initial 3 REVISE → rerun-1 5 APPROVED)
- Topics: 4 total (4 converged, 0 spawned, 0 deferred)
- Autonomous decisions: 4
- User escalations: 0
- UAG passes: 1 (Topic 3)
- Doodlestein challenges: *(populated after Step 9)*
- Deferred resolved in Sweep: 0

## Recorded dissents

- **gemini** on Topic 1: held CONDITIONAL ACCEPT trait if search-split
  is in v0.0.1; majority position prevailed. If a 2nd Storage impl
  materializes unexpectedly within v0.0.1 (e.g., Kuzu adoption pulled
  forward), trait can be introduced incrementally per the high
  reversibility basis.
- **codex** on Topic 2: held DEFER with trigger (governance argument:
  lower-friction for solo operator). REJECT permanently prevailed
  because the proposed trigger was ruled chicken-and-egg
  (unobservable without the column it gates).

## Next Steps

**Update blueprint** (`docs/blueprint.md`): reflect Topic 1 decision
in §6 (Storage = adopt only at Tier 2 trigger; for Tier 1 = free
functions); reflect Topic 4's instrumentation requirement (`returned_fact_ids` logging)
in §5 P0; reflect Topic 2 rejection in §9 (out of scope) or §10
verification spikes section.

**File v0.0.1 sprint BLs** (against blueprint §5 P0/P1), arranged
in two waves per `strategic-post` finding:

### Wave 1 — independent or prerequisite (file + start in parallel)

- **BL A — Persisted domain audit table + `returned_fact_ids` logging**
  (in mengdie/, P0 instrumentation). Per `memory_search` call, log:
  query, scope, took_ms, **and a separate `audit_returned_facts`
  link table** (audit_id, fact_id) with foreign key to
  `memory_entries.id`. Comma-separated text encoding is rejected
  because the Topic 4 supersession join (`returned_fact_ids` ↔
  `memory_entries.valid_until`) cannot be expressed against a
  scalar string column. Acceptance: schema migration ships; one
  end-to-end search call writes both audit row + link rows; SQL
  query that computes "≥5 superseded-within-7-days events per 30-day
  window" runs without join-on-string contortion.

- **BL B — sqlite-vec compatibility spike** (in mengdie/). Per
  blueprint §10 verification spike. **Acceptance criteria require an
  explicit PASS/FAIL outcome record** at
  `docs/spikes/sqlite-vec-compat.md` (or equivalent) so that the
  dependent Reflection consolidation backlog item can fire on its
  trigger without ambiguity. Without an outcome record, the
  consolidation BL files against an unresolved gate (phantom-active
  failure mode).

- **BL C — AE plugin Round-0 wiring** (in
  `agentic-engineering/`, NOT mengdie/). **Phase**: pre-spawn, not
  post-research. The existing `ae:analyze` SKILL.md §3.5
  `memory_search` call runs *after* all agents have reported; that is
  Round 0 within the analyze workflow but POST-research relative to
  the original concept. Blueprint's "Round-0 injection" means the
  `memory_search` results land in agent prompts BEFORE research
  agents are spawned. **Wire format** must be specified: TL initial
  context block? Per-agent `prompt:` parameter prefix? Both? File
  this as part of the BL; don't punt to implementer guesswork.

### Wave 2 — co-committed; depends on Wave 1 BL B outcome being recorded

- **BL D + E (single co-commit PR) — `mcp_tools` two-ingest-paths
  defect fix + `search.rs` free-functions refactor**. Both touch
  the `mcp_tools.rs` ↔ `search.rs` boundary; queuing them as
  independent commits produces an intermediate API state with no
  stable contract. Single PR. **Scope must include `search_vector`
  in vector.rs** — leaving `search_vector` on `impl Db` half-
  achieves "Retrieval as a real layer" because the refactored free
  `search::memory_search` ends up delegating to a method on Db.
  Move `search_vector` and any other search-internal `impl Db`
  methods to module-level alongside `memory_search`. The two-ingest-
  paths fix consolidates `mcp_tools.rs:306-331` to call
  `ingest::ingest_document`.

**File backlog with triggers** (deferred, not in v0.0.1 sprint):

| Backlog item | Trigger condition |
|---|---|
| `Storage` trait introduction | Tier 2 Kuzu adoption commitment / 2nd Storage impl committed in same sprint |
| Reflection module consolidation | After sqlite-vec spike outcome — collapse if `clustering.rs` survives; delete entirely if ANN replaces |
| `Reflector` trait introduction | Concrete 2nd reflection strategy + runtime call site that selects between strategies |
| Bi-temporal `event_time` column | Operator-initiated new discussion (no automatic trigger); pre-condition: concrete AE artifact with > 60s gap between event time and ingest time |
| A-MEM bidirectional update implementation | Corpus ≥1k facts AND ≥5 superseded-within-7-days events per rolling 30-day window from audit table (numbers within operator calibration discretion at BL filing time) |

→ `/ae:plan` for converged decisions (the v0.0.1 sprint BL list above).
