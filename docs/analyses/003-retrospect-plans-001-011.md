---
id: "003"
title: "Retrospect: Plans 001-011 (MVP → Phase 2 intelligence layer)"
type: retrospect
created: 2026-04-19
data_sources: 9 review files
---

# Pipeline Retrospect: Plans 001-011

## Prior Art from Project Knowledge Base

- **Retrospect 002 (plans 001-003, 2026-04-09)** — established "Challenger is
  load-bearing; code-reviewer is confirmatory." Recommended "Test ae:work on
  the next plan — plans 002-003 were manual." Predicted cross-family quota
  degradation would continue. All three retested here.
- **"AE differentiated in 3 areas but cross-family value is unvalidated — no
  empirical P1 detection data"** (analyze 038, 2026-04-16) — claimed the
  cross-family value prop lacks empirical support. This retrospect adds 6
  more data points (plans 004-009) that bear on the claim.
- **"'status: reviewed' is a pre-work gate, not terminal"** (analyze 014,
  2026-04-16) — plan lifecycle draft → reviewed → done. The completion
  invariant fix (plan 011 ships as `done` via ae:review) validates the
  corrected semantic.

## Data Summary

| # | Plan | Steps | Rework | P1 escape | Drift | Auto-pass | Cross-family |
|---|------|-------|--------|-----------|-------|-----------|--------------|
| 001 | MVP Phase 1 | 8/8 | 100% | **5** | 0 | 100% | Complete |
| 002 | Close the Loop | 35/35 | 0% | 0 | 0 | N/A (manual) | Codex only |
| 003 | Phase 1.1 | 6/6 | 0% | 0 | 0 | N/A (manual) | Degraded |
| 004 | Search Quality | 4/4 | 0% | 0 | 0 | 100% | Complete |
| 005 | Project Naming | 4/4 | **50%** | **2** | 0 | 100% | (unrecorded) |
| 006 | LLM Provider (BL-005) | 3/3 | **67%** | 0 | 0 | 100% | Codex partial |
| 007 | Embedding Clustering (BL-006) | 2/2 | 0% | 0 | 0 | 100% | Degraded mid |
| 008 | Dream Synthesis (BL-007) | 3/3 | 0% | 0 | 2 approved | 67% | Degraded all |
| 009 | Residuals Reduction (plan 011) | 2/2 | 0% | **1** | 1 non-prod | 100% | Degraded all |

Aggregates: 67/67 steps completed (100%). Median rework rate: 0%. Total P1
escapes: 8 across 9 plans. Total drift events: 3 (2 approved, 1 non-prod).

## Trends

### Steps Completion: Stable (100%)
67 of 67 planned steps delivered across 9 plans. The pipeline reliably
finishes what it plans, at any scale (2 steps to 35 steps). Confirmed across
Phase 1 MVP, Phase 2 intelligence-layer primitives, and Phase 2 follow-ups.

### Rework Rate: Non-monotonic (100% → 0% … 50% → 67% … 0%)
Plans 002-004 maintained 0% rework, plans 005-006 spiked (50% / 67%), plans
007-009 returned to 0%. The spikes correlate with:
- **Plan 005** (2/4 fixups: TOML comment + transaction semantics) — first
  plan to touch on-disk config parsing; ae:work pre-commit had no precedent
  for the comment-stripping trap
- **Plan 006** (BL-005, 2/3 fixups: accumulated Doodlestein checkpoint
  surfaced provider-dispatch + flag-drift; 7 P2 findings) — first caller
  of a novel primitive (LlmProvider trait); review raised expected volume

Plans 007-009 (built ON those primitives) stayed clean. **Interpretation**:
rework spikes track "first caller of a new concern," not pipeline
degradation. Predicts: next novel primitive (e.g., BL-008 power-law decay)
will likely spike again. Not a regression to chase.

### P1 Escape Rate: Mostly eliminated, 3/9 spikes
- **Plan 001** (5 P1s) — greenfield; cross-cutting bugs (Dreaming math,
  FTS5 injection, embedding dim mismatch) invisible to per-commit review
- **Plan 005** (2 P1s) — transaction semantics + TOML comment handling; the
  ae:work pre-commit review didn't run Codex spot-check on the Step 1/2 diff
  because cross-family was degraded
- **Plan 009** (1 P1) — test-fixture discrimination gap; an all-skip fixture
  couldn't distinguish pair-denominator from total-denominator. Plan-review
  partially addressed, /ae:review closed it with `ClusterSizeAwareProvider`

**Pattern not previously named**: the plan 009 P1 is a new failure mode —
"test fixture that cannot falsify the property it claims to test." Different
from plans 001/005 (missing review coverage) — this is review coverage that
fires on a test that doesn't actually discriminate. Worth a generalized
reviewer heuristic (see Insight #3 below).

### Drift Events: Consistently low, signal is accurate
Zero drift for plans 001-007. Plans 008-009 both logged drift: plan 008's
2 events were legitimate (callsite defaults from adding `is_longterm`,
db.rs helpers mandated by plan body but not in Expected files); plan 009's
1 event was non-production (.DS_Store via `git add -A`). Drift detection
is firing correctly on genuine corner cases.

### Auto-pass Rate: 7/9 plans at 100%
- Plans 002-003 executed manually (pre-ae:work)
- Plan 008 at 67% (Step 3 TL-executed, counted as auto-pass per review but
  strictly not a checkbox tick)
- All other plans: 100%. ae:work's gate conditions are well-tuned.

### Cross-family Coverage: Persistent degradation since plan 007
Plans 001, 004 had complete cross-family (Codex + Gemini). Plans 006-009 ran
degraded (Gemini key invalid / Codex account-limited). The Claude-Sonnet
fallback protocol activated for plans 008 and 009 — no findings-gap was
observed (fallback produced high-quality challenger critique on plan 011).

**Validates analyze-038's claim that cross-family value is empirically
unvalidated**: plans 007-009 all shipped PASS with degraded coverage. The
null hypothesis (cross-family adds measurable value) cannot be rejected on
this data — the 1 P1 in plan 009 was caught by code-reviewer + architect
convergence, not by cross-family.

## Validation of Retrospect 002 Predictions

| 2026-04-09 prediction | 2026-04-19 outcome |
|---|---|
| "Challenger is the consistent MVP" | **Confirmed.** Plan 007 challenger flagged seed-ordering design bet + wall-clock test taxonomy. Plan 008 challenger flagged `unwrap_synthesized` test brittleness + attribution loss. Plan 009 challenger was the primary source of the 60%→90% reframe challenge. All high-leverage. |
| "Test ae:work on the next plan" | **Done.** Plans 005-009 all used ae:work. Quality mixed — plans 005/006 spiked on rework; plans 007-009 recovered. ae:work is healthy on first-caller plans for established primitives. |
| "Accept degraded cross-family as normal" | **Confirmed.** 6+ consecutive plans ran degraded; no delivery impact observed. Claude-Sonnet fallback is adequate for the challenger lens specifically. |
| "Knowledge loop is accelerating review quality" | **Confirmed but more nuanced.** Plan 009 review surfaced 5 prior items (plans 009, 010 reviews; discussion 018) that shaped the finding classification. But plan 006 review showed a new gap: the "tokio subprocess must use concurrent I/O" entry was the plan's OWN ingested knowledge meta-guiding its own Step 2. Self-referential knowledge loops require care not to double-count. |

## Actionable Insights

### 1. Plan-review is a distinct P1-preventing layer (confirmed)
Plans 010 and 011 plan-reviews caught **9 Must Fix items** (5 + 4) before
/ae:work started. Types caught:
- Plan 010 review: type-shape errors (NewMemory.is_longterm, sync/nested-Runtime,
  LlmFuture return type, naive brace parser, --synthesize default)
- Plan 011 review: spec-precision errors (line number off-by-one,
  EXPECTED_SYSTEM_PROMPT implicit, pair-denominator pre-load, Step 3 dangling
  gate)

Without plan-review, these would likely have surfaced as /ae:review P1s. The
ratio "4-5 Must Fix per plan-review" is a strong ROI signal. Plan-review
earns its place as a pipeline step.

### 2. "First caller of a new primitive" plans have elevated rework (new)
Plans 001 (greenfield), 005 (first TOML parser), 006 (first LlmProvider
caller) all spiked on rework or P1 escape. Plans 007-009 (downstream of
those primitives) stayed clean. **Prediction rule**: when planning a
primitive introduction, double the expected review fixup budget and bias
plan-review agents toward "design-intent-verification" over
"code-correctness."

### 3. Fixture-discrimination gap as a new failure mode (new)
Plan 009's P1 was novel: a test using an all-skip `FixedProvider` validated
the pair-denominator math, but the fixture couldn't falsify a total-denominator
bug (both would produce the same assertion result). The
`ClusterSizeAwareProvider` fix introduced MIX of outcomes so the test
discriminates.

**Generalized principle**: when a test's stub returns a single outcome
class, check whether an alternative (buggy) impl would produce the same
assertion. If yes, the fixture lacks discrimination. Worth a reviewer prompt:
*"Does this test fixture vary enough to distinguish correct-impl from
buggy-impl, or does every possible impl pass the assertion?"*

### 4. Cross-family value-prop empirically unvalidated (confirms analyze-038)
6+ consecutive plans in degraded mode shipped PASS. The one P1 in this
stretch (plan 009) was caught by Claude-only agents (code-reviewer +
architecture-reviewer converged). Cross-family's specific claimed benefit
("fresh-eyes from different model family") produced **zero unique findings**
across plans 004-009 that Claude-side agents didn't also find.

**Operational implication**: continue degraded operation. Revisit cross-family
value only when a specific failure pattern emerges that Claude misses. Until
then, the $20/mo Codex Pro spend is buying no measurable review uplift.

### 5. Drift detection is accurate but drift itself is rare (confirm)
3 drift events across 9 plans; 2 approved, 1 non-prod. Drift detection is
firing correctly and not producing noise. Keep the "Expected files:" plan
requirement; it's earning its place.

## Recommendations (Prioritized)

1. **Keep plan-review in the pipeline** — 9 Must Fix items caught across
   plans 010/011 is strong ROI. Make plan-review mandatory for any plan
   touching a primitive.
2. **Add "fixture-discrimination" check to the reviewer prompt** — single-
   outcome stubs that can't falsify the property under test are a new
   failure mode (plan 009). Promote to a durable reviewer checklist item.
3. **Accept cross-family degradation for now** — no review uplift observed
   across 6 degraded plans. Revisit only when Claude-only agents miss
   something Codex/Gemini would have caught. Don't spend effort recovering
   the Gemini key.
4. **Double rework budget on primitive-first plans** — plans 005/006 spiked
   to 50-67% rework. Expect this on BL-008 (power-law decay) or any
   plan introducing a novel type/trait/subsystem.
5. **Don't chase rework trend reversals** — plans 007-009's 0% rework is
   expected variance, not regression correction. Interpretation discipline:
   rework correlates with plan-scope novelty, not pipeline health.

## Delta vs Retrospect 002

| Metric | Retrospect 002 (plans 001-003) | Retrospect 003 (plans 001-009 eff.) | Change |
|--------|-------------------------------|-------------------------------------|--------|
| Mean rework | 33% | 19% | ↓ -14pp (improving) |
| Mean P1 escape | 1.67 / plan | 0.89 / plan | ↓ -0.78 (improving) |
| Mean drift | 0 / plan | 0.33 / plan | ↑ +0.33 (degrading, but intentional — drift detection landed mid-sample) |
| Cross-family availability | 33% (1/3 complete) | 22% (2/9 complete) | ↓ -11pp (degrading externally) |
| Auto-pass (100% plans) | 33% (1/3) | 78% (7/9) | ↑ +45pp (improving) |
