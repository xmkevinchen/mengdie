---
reviewer: doodlestein-strategic
discussion: "019"
date: 2026-04-20
verdict: pass-with-minor-gap
---

# Doodlestein Strategic Review — Discussion 019

**Verdict**: PASS. The conclusion is well-reasoned, internally consistent, and
ready for `/ae:plan`. One minor observability gap worth noting.

## Single Smartest Improvement

**Topic 5 `DreamingResult` is missing `avg_effective_score_after`.**

The conclusion specifies three new fields: `demoted`, `avg_effective_score_before`,
and `decay_floor_breaches`. `avg_effective_score_before` is a point-in-time
gauge — it tells you where the distribution sat before the pass, but nothing
about what changed. Without `avg_effective_score_after`, operators running
`mengdie dream` can't confirm whether a demotion actually shifted the
long-term pool's effective score distribution.

This matters because the first real demotion won't arrive until ~77 days
post-ship. When it does, the only signal that something happened is the
`demoted: N` counter — but if `N > 0` and the score distribution doesn't
visibly improve, there's no data to distinguish "decay working as expected"
from "we demoted the wrong memories". A before/after pair turns a gauge into
a delta and closes that gap with one additional `f64` field.

**Recommendation for plan**: add `avg_effective_score_after: f64` to
`DreamingResult` alongside `avg_effective_score_before`. Cost: ~2 LOC.
This doesn't block the plan or require a new discussion round — it's an
additive field within the already-scoped observability work.

## Everything Else: No Issues

- Formula, constants, and half-life semantics: correct and well-justified.
- Computation sites (dreaming pass + search re-rank): appropriate; same-clock
  requirement is correctly flagged as a correctness constraint, not a
  preference.
- Demotion asymmetry and NULL-skip rule: sound; hysteresis gap (63 days) is
  structurally documented.
- Promotion path unchanged: narrowing scope to staleness-only is honest and
  does not block orthogonal session-dedup work.
- Clock injection via `Option<DateTime<Utc>>`: clean; 2-production-caller
  impact is verified by archaeologist.
- Open tails (burst inflation, NULL row, H tuning, percentile floor) are
  correctly parked with explicit revisit triggers — none require action now.
