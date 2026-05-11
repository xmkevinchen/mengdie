---
id: "027"
stage: framing
created: 2026-05-05
revised: 2026-05-05
round_0: overridden
round_0_reviewers: [codex-proxy, gemini-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer]
round_0_notes: |
  Round 0 outcome: APPROVED-via-override after rerun #2 + surgical fixes.
  Trend: rerun #0=4 REVISE → rerun #1=3 REVISE → rerun #2=2 REVISE, both
  micro-edits to same Topic 2 paragraph.
  3 APPROVED at rerun #2 (codex-proxy, doodlestein-strategic,
  minimal-change-engineer); 2 REVISE on convergent micro-edits applied
  inline before override:
    Theme F-3 (gemini-proxy): "stable, predictable baseline already
      running" → "stable, predictable; already operational"
    Theme F-4 (doodlestein-adversarial): added "on-demand (explicit
      operator-invoked trigger; zero new metrics)" as 5th candidate;
      adjusted "four" → "five" internal references.
  minimal-change-engineer non-blockingly flagged the same on-demand
  consistency issue ("Not a REVISE-level issue — Round 1 will reconcile
  naturally"). With both fixes applied, all reviewer concerns are addressed.
  Override rationale: rerun #3 risks diminishing returns; further
  iterations beyond convergent micro-edits would more likely catch new
  nits than substantive issues. All Round 1 walls and bias risks are
  resolved.
  Verdicts archive: round-00/ (rerun #0), round-00/r2/ (rerun #1),
  round-00/r3/ (rerun #2).
---

# Framing — v0.0.1 Step 0 industry survey, post-survey decisions

## Problem Statement

The 2026 personal AI memory landscape has been surveyed (analysis.md +
blueprint.md v0.2). Eight cross-source convergent patterns emerged
and a real, unfilled niche was identified. Five architectural design
points documented in blueprint §8 need a written decision before P1 /
P2 implementation BLs can be filed under the v0.0.1 rebuild.

Each design point is one of two types:

- **Open** — no prior project commitment exists; the discussion
  proposes a v0.0.1 default and records reversibility.
- **Ratify** — a prior commitment exists in CLAUDE.md or a previously
  concluded discussion; v0.0.1 either confirms it (1-round
  evidence check) or surfaces evidence sufficient to revise.
  The bar for revision is "evidence to overturn", not preference.

**Why ratify topics are kept rather than dropped to constraints:**
v0.0.1 is a deliberate rebuild ahead of P1/P2 BL filing. The intent
is to produce a single auditable conclusion document that locks all
five design points as v0.0.1 commitments — including the two whose
substance was already decided in CLAUDE.md. A ratify topic is not
ceremony when its output is a written, dated, evidence-cited
confirmation that downstream BLs and future operators can cite by
reference. Dropping topics 3+4 to constraints would force every
future BL touching ingest-source-boundary or cross-project-scope to
re-derive the rationale from CLAUDE.md prose; ratify produces a
concrete artifact instead.

Round 1 agents must explicitly classify each design point on entry
and proceed accordingly — open topics get full research, ratify
topics get a focused evidence-check against the prior commitment.
Either result exits the discussion as a written decision.

## Scope — five design points organized by dependency

### Cluster 1 — AE↔mengdie ingest contract (topics 4 → 1, sequentially dependent)

These two design points share a root: they define mengdie's entry
point. The content boundary (topic 4) gates the delivery mechanism
(topic 1).

- **Topic 4 — Ingest source boundary.** *(ratify)*
  CLAUDE.md Project Status (2026-04-27 strategic reframe) commits
  to "mengdie = AE 的大脑 ... mengdie receives AE-distilled
  propositional facts as ingest input." This discussion confirms
  AE-only as the v0.0.1 boundary, or surfaces evidence to revise.
  If ratified, topic 1's design space narrows to a single
  well-defined producer.

- **Topic 1 — Ingest mechanism.** *(open)*
  Push (AE explicitly calls `memory_ingest`) / pull (mengdie
  watches AE output dir) / hybrid (both active, one primary) /
  event-driven alternative (queue, message bus). Pick a v0.0.1
  default with reversibility-aware rationale. Both directions
  have v0.x infrastructure: push exists in `mcp_tools::ingest`;
  notify-based watcher library exists in `core/watcher.rs` but
  was never wired to a daemon.

### Cluster 2 — Synthesis trigger (topic 2, single-item)

- **Topic 2 — Reflection trigger model.** *(open with v0.x baseline)*
  v0.x already shipped cron-based synthesis (`docs/plans/010-dream-synthesis.md`,
  first real run produced 13 syntheses against production DB).
  Open question: which trigger model fits v0.0.1 — **cron-only**
  (stable, predictable; already operational), **on-demand**
  (explicit operator-invoked trigger; zero new metrics),
  **salience-threshold** (responsive but requires runtime metrics
  per Generative Agents), **composite** (entropy + conflict-density
  + elapsed time per SCM), or **debounced submit-dedupe** (per
  LangMem ReflectionExecutor)?
  All five are evaluated as distinct candidates with different
  implementation costs. Constraint: three of the five options
  (salience, composite, debounced) require runtime metrics mengdie
  does not yet compute — whether adding those metrics is tractable
  for v0.0.1 is itself part of the question Round 1 must answer.

### Cluster 3 — Cross-project scope (topic 3, decoupled)

- **Topic 3 — Cross-project default retrieval scope.** *(ratify-or-defer)*
  CLAUDE.md Key Design Decisions §5 commits to "Global storage,
  per-project default search — avoid migration cost when adding
  cross-project later." Independent of the ingest cluster —
  its resolution does not gate P1/P2 work in the ingest path.
  Acceptable outcomes: (a) ratify §5 unchanged, (b) revise with
  evidence, (c) defer with explicit trigger condition (e.g.,
  "revisit when N cross-project queries observed in audit table").

### Cluster 4 — Loop measurement (topic 5, decoupled)

- **Topic 5 — Loop-closure signal.** *(open)*
  Blueprint §5 P0 includes "basic instrumentation." This topic
  narrows P0 to: what's the minimum signal — quantitative or
  qualitative — that confirms mengdie is delivering value, not
  just being called?
  F-002 (recently shipped) provides per-search audit-table data
  as foundational instrumentation; the open question is whether
  audit data is sufficient or a separate measurement layer is
  needed (and what minimum form keeps the operator forced to
  confront whether the loop is working).

## Constraints — locked vs open

**Locked** — do not revisit in this discussion; rebuilding the
blueprint is a separate path:
- Identity (blueprint §1), core promise (§2), conceptual model (§3),
  differentiation (§4), function priority (§5)
- Out-of-scope items (§9): multi-tenancy / SaaS / cloud-only /
  generic doc RAG / code indexing / mobile / enterprise features
- Tech stack (Rust + rmcp + SQLite/FTS5 + fastembed + tokio;
  settled by discussions 001-003)

**Already-decided defaults — ratify or revise with evidence** (the
bar is "evidence to overturn", not "preference"):
- AE-only ingest source for v0.0.1 (CLAUDE.md 2026-04-27 reframe) →
  topic 4
- Per-project default search (CLAUDE.md Key Design Decisions §5) →
  topic 3

**Open** — no prior commitment; discussion produces the decision:
- Topic 1 (ingest mechanism), Topic 2 (reflection trigger),
  Topic 5 (loop signal)

## Reference Material

- `docs/blueprint.md` v0.2 — §1–§7 locked, §8 contains the five
  design points verbatim, §10 lists verification spikes
- `docs/discussions/027-industry-state-2026/analysis.md` — 2026
  industry survey + cross-source convergence
- `docs/v0.0.1-rebuild-plan.md` — Phase 0 → Phase 1 sequencing;
  this discussion gates Phase 1 BL filing
- `CLAUDE.md` Key Design Decisions §5 (per-project default search)
  and Project Status section (2026-04-27 reframe; Phase 0 research items)
- `docs/plans/010-dream-synthesis.md` — v0.x cron-based synthesis
  delivery (baseline for topic 2; first real run produced 13
  syntheses against production DB)
- `src/core/watcher.rs` + `src/core/ingest.rs` — v0.x pull
  infrastructure (baseline for topic 1)
- `src/core/mcp_tools.rs::ingest` — v0.x push infrastructure
  (baseline for topic 1)
- `src/core/audit*.rs` (post-F-002) — v0.x audit instrumentation
  (baseline for topic 5)
