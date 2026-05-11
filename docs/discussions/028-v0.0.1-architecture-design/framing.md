---
id: "028"
stage: framing
created: 2026-04-27
round_0: approved
round_0_reviewers: [codex-proxy, gemini-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer]
round_0_notes: |
  Round 0 approved on rerun #1 (2026-04-28). Unanimous APPROVED across
  all 5 reviewers including both cross-family lenses (codex + gemini).
  Per skill 1.5.3 rule 4: full coverage achieved.

  Audit trail:
  - Run 1 (2026-04-27): 3 REVISE + 2 APPROVED, verdicts in `round-00/`
  - Rerun 1 (2026-04-28): 5 APPROVED, verdicts in `round-00-rerun-1/`

  Revisions between runs (all 6 REVISE points addressed):
  - A: softened "broad agreement on shape" → "analyze phase proposed
    a layer model"; Round 1 may re-open shape if evidence warrants
  - B: trait verdicts moved from Out-of-Scope → "Analyze-phase inputs
    (not closed)"; Reflector flagged as conditional with in-sprint
    trigger via sqlite-vec spike (re-openable under topic 3)
  - C: resolution types extended with "reject permanently"
  - D: topic 1 mechanism opened (trait / struct / free fns / none)
  - E + F: item 4 reworded — concretely about A-MEM's settled
    deferral, not abstract presumption; procedural rule moved to Scope.

  doodlestein-strategic raised one cosmetic non-blocking suggestion in
  rerun (note topic 3's two sub-decisions in framing). TL disposition:
  not applied — topic-03 summary.md already has the "Related question
  — Reflector trait introduction" subsection covering this.
---

# Framing — v0.0.1 architecture design

## Problem Statement

The v0.0.1 redesign needs to commit to an architecture before BLs can
be filed against blueprint §5 priorities. Step 028's `/ae:analyze`
review (architecture-reviewer + archaeologist + challenger +
codex-proxy) converged on most architectural directions but surfaced
genuine decisions requiring resolution.

The analyze phase proposed a layer model (see analysis.md). Round 1
may re-open the layer model itself if evidence warrants, but the
discussion's primary focus is **what should be committed to in v0.0.1
vs deferred with triggers vs rejected permanently**, for three
commit-or-not decisions, plus defining the concrete trigger for one
item already converged on deferral.

The four open decisions:

1. **Storage abstraction — timing + mechanism.** Should an
   abstraction at the storage boundary be introduced in v0.0.1, and
   if so in what form? Mechanism is open: Rust trait, struct, free
   functions over a connection handle, or no abstraction at all.
2. **Bi-temporal `event_time` column.** Does the borrowed schema
   pattern (Graphiti) apply to the operator's actual AE workflow?
   Possible outcomes: commit, reject permanently, or defer with a
   concrete trigger.
3. **Reflection module consolidation + Reflector trait.** Consolidate
   `clustering` / `synthesis` / `dreaming` now or wait for the
   sqlite-vec spike outcome? Closely related: the Reflector trait
   (analyze filed as "defer with trigger") may re-open under this
   topic if sqlite-vec adoption introduces a 2nd reflection strategy
   in-sprint.
4. **A-MEM bidirectional update — concrete deferral trigger.** All
   four analyze-phase agents converged on "defer A-MEM from v0.0.1."
   This topic defines the precise, measurable trigger condition for
   re-opening A-MEM (per CLAUDE.md Review Rules: deferred items must
   have triggers).

## Scope

In scope:
- Resolving each decision as `converged` (commit / reject permanently /
  defer with trigger), `revisit` (need more information from a
  sub-spike), or `deferred` (cannot decide now; resolved in Sweep)
- Defining concrete, measurable trigger conditions for any items
  resolved as "defer with trigger"
- Updating `docs/blueprint.md` and architecture documentation to
  reflect resolutions

Procedural rule (applies across all topics): any deferred item must
exit Sweep with a concrete, measurable trigger condition. Per
CLAUDE.md Review Rules.

Out of scope:
- Re-litigating mengdie's identity (settled in `docs/blueprint.md`)
- Re-deriving the call graph / dependency violations (empirically
  established by archaeologist in 028 analyze phase)
- Designing the four-item v0.0.1 minimum sprint (AE Round-0 wiring +
  two-ingest-paths defect fix + sqlite-vec compatibility spike +
  ship). Settled by 028 analyze phase.

## Analyze-phase inputs (not closed; Round 1 may re-examine if evidence warrants)

- `LlmProvider`, `EmbeddingProvider` trait verdicts (analyze: ACCEPT
  4-of-4) — strongest convergence; lowest priority for re-litigation.
- `Transport`, `EventEmitter` trait verdicts (analyze: PREMATURE
  4-of-4) — also strong convergence; Round 1 may revise if a 2nd
  impl becomes concrete during research.
- `Storage` trait (analyze: CONDITIONAL ACCEPT — conditional on
  search-split refactor). This is the substantive content of topic 1.
- `Reflector` trait (analyze: DEFER WITH TRIGGER) — trigger may fire
  in-sprint via sqlite-vec adoption introducing a 2nd reflection
  strategy. May be re-opened under topic 3.

## Reference Material

- `docs/blueprint.md` v0.2 — what mengdie is
- `docs/v0.0.1-rebuild-plan.md` — migration outline
- `docs/discussions/025-functional-inventory/analysis.md` — v0.8.0 module inventory
- `docs/discussions/026-rust-oss-survey/analysis.md` — Rust OSS library landscape
- `docs/discussions/027-industry-state-2026/analysis.md` — 2026 industry state
- `docs/discussions/028-v0.0.1-architecture-design/analysis.md` — analyze-phase output (proposes shape, identifies the open decisions, includes trait verdict table)
- `src/` — current v0.8.0 code, especially `src/core/search.rs`, `src/core/ingest.rs`, `src/core/mcp_tools.rs`, `src/core/contradiction.rs`, `src/core/dreaming.rs`, `src/core/clustering.rs`, `src/core/synthesis.rs`
