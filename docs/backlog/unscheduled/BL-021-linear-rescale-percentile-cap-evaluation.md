---
id: BL-021
title: "Re-evaluate FTS-fallback normalization function: linear-rescale's bottom-result-always-0.0 property"
status: open
created: 2026-04-30
origin: F-003 /ae:review (Challenger C2)
trigger: "Operator reports `min_score > 0` silently dropping bottom-of-page results in degraded mode (FTS-fallback). Earliest signal: a search call returns N-1 results when the operator expected N, and the missing result has score = 0.0 due to the linear-rescale-by-per-call-max definition mapping the minimum input to exactly 0.0."
---

# BL-021 — Re-evaluate FTS-fallback normalization with percentile-cap or log alternatives

## What

F-003 Step 2 selected `linear_rescale_normalize` (linear-rescale-by-per-call-max)
as the FTS-fallback normalization function via the HARD GATE benchmark
that compared sigmoid, tanh-half-positive, and linear-rescale on the
fixture `{0.1, 1.0, 5.0, 10.0, 50.0}`. Sigmoid + tanh failed upper-range
discriminability (compress 5+ to ≈0.99); linear-rescale passed.

**Untested alternatives** (Challenger C2 in F-003 /ae:review):
- **Logarithmic compression**: `log(x + 1) / log(max + 1)` — softer
  upper compression than sigmoid, monotonic, range-bounded.
- **Percentile-capped linear-rescale**: cap at the 95th percentile of
  the result set rather than the absolute max. The minimum no longer
  always maps to 0.0; bottom results retain a score floor.

## Why it matters

Linear-rescale-by-per-call-max has a structural property the F-003
plan did not evaluate:

> **The lowest-scoring result in any FTS-only result set always scores
> exactly 0.0** (by the definition `(s - min) / range`).

If a caller passes `min_score = 0.01`, the last result in every
FTS-only page is dropped, regardless of its actual relevance. This is a
per-call-set artifact, not a quality signal — the result was the worst
match in this set, but might still be a relevant match in absolute
terms.

The default `min_score = 0.0` masks this: zero passes the filter
(`r.score >= 0.0` is always true). But any caller using `min_score > 0`
on the FTS-fallback path silently loses the bottom result.

## Trigger

File the implementing plan when ANY of:

1. An operator reports unexpected N-1 results from `mengdie search` /
   MCP `memory_search` calls under embed-fail (FTS-fallback) when
   `min_score > 0` is set. Earliest signal: a support thread or a
   `mengdie audit-stats` (BL-014) report showing systematic
   `min_score`-suppressed results in FTS-fallback mode.
2. A future v0.x feature requires `min_score`-based result curation in
   degraded mode (e.g., A-MEM trigger plan adds a quality-floor filter
   that callers should respect even when the embedder is broken).
3. The HARD GATE benchmark fixture is revisited for any other reason
   (e.g., adding a new candidate function), at which point the
   percentile-cap and log alternatives should be evaluated alongside.

## Candidate functions to benchmark

| Function | Pros | Cons |
|---|---|---|
| Linear-rescale-by-per-call-max (current) | Best upper-range discriminability; simple implementation | Bottom result always 0.0; min_score > 0 drops it |
| Linear-rescale capped at p95 | Bottom result has a floor (proportional to p95); preserves upper-range discriminability | Slightly more complex; quantile cost |
| Log compression `log(x+1)/log(max+1)` | Monotonic, range-bounded, softer than sigmoid | Still compresses upper range somewhat |
| Sigmoid + larger fixture | Already rejected via HARD GATE — keep as negative reference | Same upper compression issue |

## Why not now (F-003 scope)

The HARD GATE benchmark in F-003 Step 4 correctly rejects sigmoid +
tanh on the agreed fixture. Linear-rescale's selection over the
unevaluated alternatives is empirically defensible at v0.0.1 — the
default `min_score = 0.0` makes the bottom-0.0 property invisible.
Re-running the benchmark with two new candidates is not a v0.0.1
correctness gate; it's a quality refinement bounded by observable
trigger conditions.

## Implementation sketch (when triggered)

1. Add `linear_rescale_capped_at_p95` and `log_compression` as private
   helpers next to `linear_rescale_normalize` in
   `src/core/search.rs`.
2. Extend `test_fts_score_normalization_discriminability` with two
   new candidates + the same fixture + new assertions:
   - Bottom-result floor: `output(min_input) > 0.0` (linear-rescale
     fails this; capped or log passes).
   - Upper-range discriminability: same as F-003 (`output(max) -
     output(2nd-max) >= some_threshold`).
3. Pick the function that passes both. Update `linear_rescale_normalize`
   to call the chosen helper, OR rename it to reflect the new function
   shape.
4. Update plan AC2 to reflect the new function selection.
5. Update `mcp_tools.rs::search`'s `degraded` reason if the new
   normalization changes the user-visible score range.

Note: changing the normalization is a v0.0.1 → v0.x version-bump
behavior change for any operator with calibrated `min_score`. Per
F-003 plan Topic 6 reversibility note: "Medium pre-production-audit-data;
HIGH-cost post-production-data" — if production audit rows have been
written under linear-rescale, the new function changes A-MEM analytics'
historical comparability. Consider data-migration strategy (separate
score column? rescore on read?) at plan time.

## Reviewer note

Challenger track in F-003 /ae:review (Confidence 6). The full review is
captured in `.ae/features/active/F-003-retrieval-and-ingest-layer-consolidation/review.md`
under "Challenger C2 (Conf 6)".
