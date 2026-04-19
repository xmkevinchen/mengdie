---
id: "004"
title: "Comparison: Retrospect 002 (plans 001-003) vs Retrospect 003 (plans 001-011)"
type: retrospect-comparison
created: 2026-04-19
compared: ["002", "003"]
---

# Pipeline Comparison: Retrospect 002 vs Retrospect 003

10-day window between snapshots (2026-04-09 → 2026-04-19). 6 new plans
landed (004-009 in retrospect-003 numbering, corresponding to plans
004/005/007/009/010/011 by file ID). Report 002 covered the MVP and
close-the-loop phase; report 003 extended through the Phase-2 intelligence
layer (LLM provider, clustering, dream synthesis, residuals reduction).

## Delta Summary

| Metric | Retrospect 002 (n=3) | Retrospect 003 (n=9) | Change |
|--------|----------------------|----------------------|--------|
| Steps completed | 49/49 (100%) | 67/67 (100%) | — 0 |
| Mean rework rate | 33% | 19% | ↓ -14pp |
| Mean P1 escape (per plan) | 1.67 | 0.89 | ↓ -0.78 |
| Drift events (total) | 0 | 3 | ↑ +3 |
| Auto-pass rate (plans at 100%) | 33% (1/3) | 78% (7/9) | ↑ +45pp |

## Analysis

### Steps completed — stable at 100%
Pipeline reliably finishes planned work regardless of scale or complexity.
9-plan data validates the 3-plan baseline. No degradation signal; this is
the strongest invariant the pipeline has.

### Mean rework rate — improving (33% → 19%)
Plan 001's 100% rework pulled the 3-plan mean sharply up. Adding 6 more
plans including plan 005 (50%) and plan 006 (67%) dilutes plan 001's
outlier effect while introducing two new mid-range spikes. Net improvement
comes from the majority of plans (004, 007, 008, 009) scoring 0% rework.
**Caveat** (surfaced in retrospect 003 Insight #2): rework spikes
correlate with "first-caller-of-new-primitive" plans (005 = first TOML
parser; 006 = first LlmProvider). Not a monotonic quality trend — a
structural correlation.

### Mean P1 escape — improving (1.67 → 0.89 per plan)
Plan 001's 5 P1s remain the dominant contributor. Plans 005 (2 P1s) and
009 (1 P1) add 3 total across 6 new plans. The 6 new plans added 3 P1s
against 6 plans — lower per-plan rate (0.5) than the 3-plan baseline
(1.67). Genuine improvement, but still non-zero — plan-review is catching
most issues but /ae:review still finds occasional P1s on primitive-first
plans. Retrospect 003 Insight #1 (plan-review is a distinct P1-preventing
layer) supported by this delta.

### Drift events — degrading per metric direction (0 → 3)
Surface reading: drift got worse. Actual cause: drift detection protocol
landed in plan 008 (mid-sample for retrospect 003). Retrospect 002's "0
drift" was partly "0 drift detected because detection didn't exist yet."
Retrospect 003's 3 drifts are all known-safe (2 approved at /ae:work, 1
non-production .DS_Store from a `git add -A` slip). This metric is NOT a
quality regression — it's detection catching up with reality. Future
retrospects should treat pre-plan-008 drift counts as "unknown," not "0."

### Auto-pass rate — improving (33% → 78%)
Largest relative improvement. Plans 002-003 were executed manually (no
ae:work auto-pass); their `N/A` pulled the 3-plan rate to 33%. 8 of 9
plans in retrospect 003 used ae:work; 7 reached 100% auto-pass. Gate
conditions are well-tuned for the post-002/003 plan shape. The lone
sub-100% plan (008, at 67%) had a Step 3 TL-executed manually, not a gate
failure.

## Cross-family coverage delta (supplementary observation)

Not in the 5-metric comparison table but tracked in both retrospects:

| | Retrospect 002 | Retrospect 003 | Change |
|---|---|---|---|
| Plans with full cross-family | 1/3 (33%) | 2/9 (22%) | ↓ -11pp |
| Plans in degraded/fallback mode | 2/3 (67%) | 6/9 (67%) | — 0 |
| P1s caught ONLY by cross-family | N/A (not tracked) | 0 | — |

Retrospect 003 Insight #4: cross-family's specific value-prop (different
model family → fresh eyes) remains empirically unvalidated. Plan 012's
review (post-retrospect-003 data point, not in this comparison) added a
4th data point reinforcing the null hypothesis — Codex's first successful
proxy call since plan 007 produced zero unique findings vs Claude
reviewers. Continued degraded operation recommended.

## Recommendations

Based on delta patterns:

1. **Keep gate conditions as-is** — 78% plans at 100% auto-pass, up from
   33%. The improvement came from ae:work adoption, not gate tightening;
   don't fix what works.
2. **Plan-review earns mandatory status** — P1 escape rate declined
   0.78/plan. Retrospect 003 attributed this to plan-review catching
   4-5 Must Fix items per plan before /ae:work. Delta supports that story.
3. **Treat pre-plan-008 drift counts as "unknown"** — the 0→3 drift trend
   is a detection-gain artifact, not a quality regression. Future
   comparisons should note this sample-boundary effect.
4. **Don't chase rework trend reversals** — 33%→19% is real but the
   underlying driver (first-caller primitive plans) is structural. Expect
   the next novel-primitive plan to spike rework again. This is signal,
   not regression.
5. **Accept cross-family degradation as normal** — 9+ degraded plans have
   shipped PASS with no findings gap. Further retrospect data should focus
   on whether the specific cross-family value is ever observed empirically,
   not on quota recovery.

## Next Steps

Trends are healthy. No immediate action needed. Candidate follow-ups:
- `/ae:roadmap` — backlog at 16 items; good time to group.
- Continue accumulating data; next retrospect should cover plans 010-012
  at minimum to test the retrospect-003 conclusions on fresh data.
