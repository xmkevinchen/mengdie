---
id: "023"
title: "Analysis: v0.8.5 scope — feature inventory + sprint contents"
type: analysis
created: 2026-04-27
tags: [v0.8.5, sprint-planning, feature-inventory, backlog-triage, roadmap]
---

# Analysis: v0.8.5 scope — feature inventory + sprint contents

## Question

(a) What does mengdie do today (post-v0.8.0 close)?
(b) Given that inventory + the unscheduled backlog + the upcoming v0.9.0
BL-009 (MCP Dream Tool) commitment, what should v0.8.5 actually contain?

## Findings

### Prior Art from Project Knowledge Base

- **Trigger-gated BL defer pattern** (discussion 021,
  `docs/discussions/021-v0.8.0-bl-dependencies/analysis.md`,
  knowledge_type: factual, valid_from 2026-04-23) — "before running
  /ae:roadmap plan v<ver>, skim candidate BL bodies for explicit 'not
  now' / 'filed for trigger' language. /ae:roadmap remove such items
  before sprint-commit. Avoids repeating the v0.8.0 pattern where 2
  defer-trigger BLs got committed and had to be retroactively removed."
  Directly governs this analysis.
- **CLAUDE.md drift** (discussion 014,
  `docs/discussions/014-progress-audit/analysis.md`, factual, 2026-04-16)
  — "8 stale pipeline fields, 6 analyze-only discussion stubs, CLAUDE.md
  Project Status completely outdated." Validates challenger's C4
  (CLAUDE.md "Next step: 67% residuals" is stale post-plan 011).
- **Defer-and-re-file methodology obligation** (plan 015 decisional,
  2026-04-23) — when a BL is filed for a future sprint, the deferring
  plan's methodology obligation (test patterns, schema-contract checks)
  carries forward verbatim. Applies to any v0.8.5 BL re-filed from
  plan 017 review.

### Relevant Code

**User-facing CLI surface** (`src/bin/cli.rs` Commands enum):
`dream`, `import`, `list`, `search`, `rename`, `stats`,
`synthesis-audit`. No `delete` / `memory_invalidate` CLI subcommand
(blocks BL-synthesis-preload-db-miss-edge trigger).

**MCP tool surface** (`src/core/mcp_tools.rs`): `memory_search`,
`memory_ingest`, `memory_invalidate`. All shipped, complete.

**Internal subsystems** — most complete; notable shapes:
- `src/core/dreaming.rs` — **1326 lines**, three concerns in one file
  (promotion/demotion + LLM orchestration + test stubs). The split
  was filed as `BL-dreaming-module-split` with trigger "BL-008 plan
  lands"; **BL-008 shipped as plan 013 on 2026-04-20, trigger
  unambiguously fired, split never executed.**
- `src/core/schema.rs` — schema v5 just landed (plan 017). Known gaps:
  `synthesis_cluster_hash` nullable + partial-index silent exclusion;
  FK pragma off; source_type UPDATE-gap. All filed as separate BLs.
- `src/core/db.rs` — `get_synthesis_with_sources` has known N+1; bounded
  today by `max_cluster_size = 20`.
- `src/core/clustering.rs` + `src/core/synthesis.rs` — both complete and
  validated empirically.

**Production-DB state**: known orphan synthesis row
`529d3212-e809-4b81-a1f5-e15143df5128` (zero links) blocks v5 migration
on `~/.mengdie/db.sqlite` until operator resolves it.

### Architecture & Patterns

The codebase is well-factored except for `dreaming.rs` (1326 LOC, three
concerns). Schema migrations follow a transactional pattern with
pre-checks (plan 017 v5 set the precedent). MCP tools wrap library
methods 1:1 (`memory_search` → `db.memory_search`); the abstraction is
thin and adding new tools is mechanical. The LLM provider is
trait-based with one impl (`ClaudeCliProvider`).

Phase 2 dependency chain (per `docs/backlog/005-phase2-roadmap.md`):
**BL-009 (MCP Dream Tool) → BL-010 (daemon) → BL-011 (entity extraction
"Lint") + BL-013 (typed edges "Edges")**. Skipping order would violate
the agreed roadmap.

### Industry Practice Comparison

(From standards-expert)

- **Semver**: `v0.8.5` is idiomatic IF patch-only — fixes, hardening,
  docs, DX improvements. **Awkward** with new features (PyO3, tantivy,
  tokio all reserve patches for bugfixes; new features land at minor).
  Using 0.8.5 for a "light feature" sprint is the awkward case the
  community avoids.
- **Maintenance sprint shape**: invariant hardening + docs + small
  tests. NOT new MCP tools, NOT new CLI subcommands, NOT new algorithms.
- **Sprint discipline**: project's "trigger-fired-only" rule is
  conservative vs Scrum DoR but appropriate for solo+AE workflow (the
  explicit pre-planning sweep substitutes for product-owner separation).
  Validated by v0.8.0 incident (2 trigger-gated items committed and
  retroactively removed).
- **Realistic ceiling**: 8–12 points (4–6 BL items) for a v0.8.5 sized
  to close in ~1 day of agent work and review cleanly in one
  `/ae:review` pass.

### Trigger Status of Unscheduled BLs (archaeologist judgment)

| BL | Size | Trigger status | Notes |
|----|------|---------------|-------|
| **BL-dreaming-module-split** | S est. | **CLEANLY FIRED** | BL-008 shipped in plan 013 on 2026-04-20. Trigger explicitly says "first commit of BL-008 should preferentially split." 1326-line file unchanged. |
| BL-v5-migration-operator-docs | XS | likely fired | v5 migration ran on production DB; docs missing post-migration. |
| BL-enable-pragma-foreign-keys | XS | weak fire | Production orphan found in plan 017 pre-check; "next FK-bearing schema add" hasn't fired. |
| BL-synthesis-cluster-hash-not-null-enforcement | S | arguable | "Next memory_entries schema migration" — v5 just landed; could have been bundled, wasn't. |
| BL-audit-collection-discipline | XS | NOT FIRED (37%/50%) | Latent-never-fires problem. |
| BL-decay-dreaming-pass-optim | S | NOT FIRED | Corpus < 50k, no daemon. |
| BL-decay-threshold-mode | S | NOT FIRED | Gated on BL-010. |
| BL-get-synthesis-with-sources-n-plus-1 | XS | NOT FIRED | max_cluster_size unchanged. |
| BL-release-yml-ci-gate | M est. | NOT FIRED | No release ever cut yet. |
| BL-synthesis-preload-db-miss-edge | XS | NOT FIRED | `delete` CLI doesn't exist. |

**Note** (archaeologist): `docs/backlog/BL-fk-pragma-and-deletion-safety.md`
also exists — possible duplicate / older formulation of
`BL-enable-pragma-foreign-keys`. Worth deduping.

### Industry vs Project Tension

- Standards says: 4–6 items, hardening shape only. Maps cleanly to
  archaeologist's 3 fired-trigger items + at most 1–2 arguables.
- Standards forbids new features. Gemini-proxy's "Transparency Pivot"
  recommendation **adds new features** (Residuals Dashboard + Batch
  Audit + Edge Export) — those would be v0.9.0 work by community
  convention, not v0.8.5.

### Challenges & Disagreements

**Challenger (HIGH confidence on most):**

1. **C1 — v0.8.5 is theater.** Discussion 022 explicitly named v0.9.0
   as next destination. Skipping straight to v0.9.0 with XS items as
   ride-along is the agreed path. Confidence: HIGH.
2. **C2 — TL-recommended items don't actually have fired triggers.**
   "Next doc polish sprint" is self-referential. "Schema v6 migration"
   hasn't landed. The TL was stretching. Confidence: HIGH.
3. **C4 — 67% residuals already addressed.** Plan 011
   `status: done`. CLAUDE.md "Next step (current)" is stale (validated
   against discussion 014 prior). Confidence: HIGH.
4. **C5 — Phase 2 chain is BL-009 → BL-010 → BL-011/BL-013.** Spiking
   on Lint/Edges before BL-009 violates the agreed roadmap.
   Confidence: HIGH.
5. **C6 — Correct answer is probably skip v0.8.5.** Either user has
   unstated work (force them to name it), or 2 XS items is just one PR.
   Confidence: HIGH.

**Codex (technical-debt lens, MCP unavailable — direct fallback):**

- P1: BL-enable-pragma-foreign-keys — shields BL-009's new synthesis
  paths from silent corruption.
- P1: BL-synthesis-cluster-hash-not-null-enforcement — closes
  doc-over-enforcement gap before BL-009 adds new writers.
- P2: BL-v5-migration-operator-docs — operator readiness for production
  v5 migration that has known orphan blocking it.
- Skip: BL-get-synthesis-with-sources-n-plus-1 (premature).
- **Recommends running production v5 migration BEFORE locking v0.8.5
  scope** — migration result informs scope.

**Gemini-proxy (UX/product lens):**

- Reframes the residuals problem: 67% is *not* a math failure (plan
  011 fixed the algorithm); it's a **mental-model + UX** failure (Kai
  feels "it's broken" because audit is one-at-a-time and topology is
  invisible).
- Top 3 UX gaps: (1) residuals anxiety, (2) audit fatigue, (3)
  invisible topology.
- Proposes "Transparency Pivot" theme: Residuals Dashboard + Batch
  Audit + lightweight edge export.
- Strong claim: v0.8.5 is **prerequisite for BL-009 success**. BL-009
  brings Claude into the synthesis loop in-session — if audit is still
  one-at-a-time and noisy, BL-009 amplifies the noise.
- Tension with standards: gemini's recommended items are NEW
  FEATURES, not patch-shape work. They'd violate the "no new
  user-visible API" rule for a 0.x.5 release.

**Disagreement Value Assessment**: The reviewers diverge on whether
v0.8.5 should exist AND what shape it should take. The disagreement is
real and productive — it surfaces 4 distinct paths that the user must
choose between, none clearly dominant.

## Summary

**Inventory finding**: mengdie post-v0.8.0 has a complete user-facing
surface (7 CLI subcommands, 3 MCP tools), all subsystems are functional,
and the production DB has one known operator-action item (orphan
synthesis row blocking v5 migration).

**Scope finding**: the analysis does NOT converge on a single answer.
Four paths, with the dominant tradeoffs:

### Option A: Skip v0.8.5 entirely (challenger)

Open v0.9.0 with BL-009 + 2-3 XS hardening items as scope-dust.

- **Pro**: Honors discussion 022 ("next is v0.9.0"). No version-juggling
  for a single PR. Trigger discipline preserved.
- **Con**: BL-dreaming-module-split's trigger HAS fired and skipping
  v0.8.5 means scheduling it within v0.9.0, where it gets buried under
  BL-009's blast radius.

### Option B: Trigger-fired hardening sprint (archaeologist + standards)

Schedule the 3 cleanly-fired items: **BL-dreaming-module-split** (S,
~100 LOC pure refactor) + **BL-v5-migration-operator-docs** (XS) +
**BL-enable-pragma-foreign-keys** (XS). ~4 points, half-sprint.
Optionally add **BL-synthesis-cluster-hash-not-null-enforcement** (S)
for full BL-009 readiness; bumps to ~6 points.

- **Pro**: Idiomatic v0.x.5 (patch-shape: refactor + docs +
  invariant). All triggers defensible. Closes BL-009 readiness gaps.
- **Con**: Module split has been "should preferentially do" since BL-008
  shipped 7 days ago; if it were urgent, it would have been bundled
  with plan 013. Probably medium-priority at best.

### Option C: Transparency Pivot (gemini-proxy)

Theme: make residuals actionable. Ship: Residuals Dashboard + Batch
Audit + lightweight edge export. ~3 items, but ALL NEW FEATURES.

- **Pro**: Addresses the real UX pain Kai feels ("residuals anxiety").
  Sets up BL-009 trust foundation.
- **Con**: Violates 0.x.5 patch convention (these are minor-version
  features). Items don't exist as BLs — needs discuss→plan cycle BEFORE
  /ae:roadmap plan can run. This is a v0.9.0 theme, not v0.8.5.

### Option D: Hybrid pre-BL-009 readiness

Combine Option B (small fired-trigger work) with deliberate **BL-009
scoping** as a separate /ae:discuss task. v0.8.5 = (a) module split +
(b) operator docs + (c) FK pragma + (d) BL-009 design discussion to
seed v0.9.0. Total: 3 small hardening items + 1 design discussion.

- **Pro**: Preserves Option B's safety; adds explicit BL-009
  preparation that avoids "v0.9.0 starts cold" risk. Archaeologist
  flagged BL-009 has NO discussion doc yet.
- **Con**: Mixes hardening sprint + design work. Two-headed scope.

### Recommendation

**Start with Option B (hardening sprint)** unless the user has unstated
work in mind. If user wants UX features from Option C, those are
v0.9.0+ scope — file as new BLs, don't smuggle into v0.8.5.
Independent of the v0.8.5 decision: **resolve the production orphan
synthesis row + run v5 migration** before any v0.8.5 work begins. Per
codex's direction-validation argument, the migration result may surface
new evidence that informs scope.

CLAUDE.md "Next step (current): residuals reduction" should be updated
**now** regardless of v0.8.5 outcome — it's stale and was misleading
the analysis.

## Possible Next Steps

- **Decide v0.8.5 path** → `/ae:discuss docs/discussions/023-v0.8.5-scope-decision/`
  to walk through Options A/B/C/D with structured argument.
- **Resolve production blocker first** → manually delete or restore
  the orphan synthesis row in `~/.mengdie/db.sqlite`, run v5 migration,
  observe results.
- **Update CLAUDE.md** → unrelated to v0.8.5 but blocking analysis
  clarity. "Next step (current)" line needs to reflect post-plan-017
  reality.
- **If Option B/D selected** → `/ae:roadmap plan v0.8.5 --items
  BL-dreaming-module-split,BL-v5-migration-operator-docs,BL-enable-pragma-foreign-keys
  --theme "..." --gate "..."` once BL files are sized + present in
  unscheduled.
- **If Option C selected** → `/ae:discuss` on a "Residuals UX" topic
  first; v0.8.5 deferred until BLs exist.
- **If Option A selected** → `/ae:roadmap plan v0.9.0 --items
  BL-009,...` directly.
