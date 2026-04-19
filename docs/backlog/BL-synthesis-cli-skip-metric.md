---
id: BL-synthesis-cli-skip-metric
status: open
origin: BL-residuals-reduction AC5 post-ship audit (2026-04-19)
created: 2026-04-19
scope: mengdie (observability / operator-facing metric label)
---

# `mengdie dream --synthesize` CLI mislabels pair-cluster skip rate

## Finding

The CLI output line for synthesis currently reads:

```
Synthesis: N syntheses created from M clusters (R residuals skipped,
S LLM-skipped (S/P pair-clusters = X%), ...)
```

where `S = syntheses_llm_skipped` (total LLM skips across ALL cluster sizes)
and `P = pair_clusters_processed` (count of clusters whose trimmed size == 2).

The numerator and denominator count different things:

- `S` includes skips from size=3, size=4, ..., size=20 clusters (non-pair
  clusters the LLM rejected as incoherent)
- `P` is pair-only

On the 2026-04-19 production run: 11 LLM-skips total (3 from pair-clusters,
8 from non-pair clusters), 11 total pair-clusters processed. The CLI
displayed `(11/11 pair-clusters = 100%)` — suggesting every pair-cluster was
skipped. The **true** pair-cluster skip rate is 3/11 = 27% (8 pair-clusters
were synthesized successfully).

The plan 011 review (architecture-reviewer, code-reviewer,
cross-family-fallback) caught a related gap — the all-skip fixture couldn't
distinguish pair-denominator from total-denominator — and fixed it with
`ClusterSizeAwareProvider`. That test fix verifies the **denominator** is
pair-count; it does NOT verify the **numerator** is pair-skip-count, because
the test's stub skips only pair-clusters (by design — to discriminate
denominator). Production has a different mix.

## Why this matters

The label reads as "hatch efficacy on pair-clusters" — the key operator
signal from plan 011 AC5 ("target < 25% hatch working, 25-40% monitor, > 40%
revisit min_size"). A misleading 100% reading here would wrongly trigger a
`min_size` revert decision when the hatch is actually working correctly at
27%.

The fix is in the audit write-up manually (`docs/backlog/BL-clustering-validation.md`
records the true 3/11 = 27%) but operators running `dream --synthesize`
directly see the wrong number.

## Two repair options

### Option A: fix the math, keep the label

Track a `pair_clusters_skipped` counter alongside `pair_clusters_processed`.
Display `pair_clusters_skipped / pair_clusters_processed`. Matches the label
intent precisely. Requires one more field in the tuple/struct return (likely
pairs with `BL-synthesis-result-struct-promotion` trigger).

### Option B: keep the math, fix the label

Rename the metric to `"S LLM-skipped ({S}/{C} clusters = {X}%, {P} were
pair-clusters)"` — honest about what's being divided. Zero code-flow change.

Option A is the better operator signal (the pair-cluster skip rate was the
whole point of the plan 011 AC3 metric). Option B is a 1-minute fix if the
struct promotion hasn't landed yet.

## Trigger

Fires when:
- Next `dream --synthesize` run with a non-trivial corpus (i.e., non-demo).
- OR: `BL-synthesis-result-struct-promotion` lands — at that point, add
  `pair_clusters_skipped` as another struct field (Option A) in the same
  commit.

## Fix direction (Option A, recommended)

Add to `SynthesisResult` or the return tuple:

```rust
pub pair_clusters_skipped: usize,   // subset of syntheses_llm_skipped where
                                     // cluster_size == 2
```

Increment in `run_synthesis_pass` alongside `syntheses_llm_skipped` when
the Skipped branch fires AND `trimmed_ids.len() == 2`. Update the CLI line
to divide `pair_clusters_skipped / pair_clusters_processed`.

Add a test using the existing `ClusterSizeAwareProvider` that asserts
`pair_clusters_skipped == 2` (not 4) on the mixed-size fixture.

## Why not fixed on 2026-04-19

Outside the scope of the AC5 audit. The audit's job is to record the
post-ship signal; the metric bug was discovered while recording. Filing
separately keeps the audit artifact clean and gives the fix its own review
trail. Not urgent — the audit write-up carries the correct 27% figure for
the operator record.
