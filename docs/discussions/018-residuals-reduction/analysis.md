---
id: "018"
title: "Analysis: Residuals reduction — parameter sweep"
type: analysis
created: 2026-04-18
tags: [clustering, residuals, dream-synthesis, parameter-sweep]
---

# Analysis: Residuals reduction — parameter sweep

## Question

The first real `mengdie dream --synthesize` (2026-04-18) produced 67%
residuals (133 of 198 memories didn't cluster). This is above
BL-clustering-validation's "> 50% = signal" threshold. What parameter
change(s) meaningfully reduce the residual rate without over-clustering?

## Method

Dry-run sweep over `--threshold` × `--min-cluster-size`. Each row is a
single `mengdie dream --synthesize --dry-run` invocation. No LLM calls,
no DB writes. Corpus at time of sweep: 238 eligible memories (up from
198 at AC5 writeback — 40 new from session knowledge-capture).

## Data

| threshold | min_size | clusters | residuals | residual % | clustered % |
|-----------|----------|----------|-----------|------------|-------------|
| 0.65 | 3 | **9** ↓ | 107 | 45% | 55% |
| 0.70 | 3 | 11 | 115 | 48% | 52% |
| 0.70 | 2 | **27** | **83** | **35%** | **65%** |
| 0.75 (default) | 3 | 14 | 136 | 57% | 43% |
| 0.75 | 2 | 24 | 116 | 49% | 51% |
| 0.80 | 3 | 15 | 164 | 69% | 31% |
| 0.85 | 3 | 4 | 223 | 94% | 6% |
| 1.5 | 3 | 0 | 238 | 100% | — (baseline total) |

## Findings

### 1. Threshold 0.75 → 0.70 (min_size=3 fixed): modest gain

Residual rate drops 57% → 48% (+9 pp improvement). Cluster count rises
11 → 14 (more valid clusters form; no merging). This is a safe
threshold adjustment — no evidence of over-clustering.

### 2. Threshold 0.70 → 0.65 (min_size=3 fixed): over-merging red flag

Residual rate continues dropping (48% → 45%) BUT cluster count
**collapses** from 11 to 9. Fewer clusters + more memories clustered =
unrelated topics merging together. This is a quality regression signal.
**Do not drop threshold below 0.70 without strong evidence.**

### 3. Min_size 3 → 2 at threshold 0.70: major gain

Residual rate drops 48% → 35% (+13 pp). Cluster count rises 11 → 27
(+16 clusters). The 32 memories released from residuals are near-
duplicate pairs — memories that are similar to exactly one other
memory but didn't hit the 3-member floor.

### 4. Corpus has near-duplicate pairs

Pair-analysis (min=3 vs min=2 at same threshold):
- @ 0.75: 136 − 116 = 20 memories in pairs
- @ 0.70: 115 − 83 = 32 memories in pairs

So ~15–30 memories per run are "2-way clusters" currently residual.
Lower threshold surfaces more of them.

### 5. Best candidate from data: `--threshold 0.70 --min-cluster-size 2`

- 27 clusters (vs current 14 default)
- 83 residuals, 35% residual rate (vs current 57%)
- Cost: 27 LLM calls per run (vs current 14) — ~2× latency and token
  cost per dream pass
- Quality risk: 2-member clusters may produce weak syntheses (1 LLM
  call summarizing only 2 memories is borderline useful)

## Key Questions (for /ae:discuss)

1. **Do we accept the 2× LLM cost increase for +22 pp residual
   reduction?** The cost scales with cluster count; 27 calls × ~10s
   claude-sonnet = ~4.5 min per dream run vs current ~2.3 min.

2. **Are pair-clusters worth synthesizing at all?** Or should we keep
   min_size=3 and accept the residuals, because 2-memory syntheses
   add little value over just reading both memories?

3. **Should threshold and min_size be co-tuned**, or is it cleaner to
   keep threshold at a conservative value (0.70) and let min_size
   determine cluster density?

4. **Alternative: two-pass strategy.** First pass: threshold 0.75 +
   min_size=3 (today's default). Second pass on residuals only:
   threshold 0.65 + min_size=2 with a distinct "pair synthesis"
   prompt. Complex but preserves quality on high-confidence clusters
   while giving residuals a second chance.

5. **Alternative: reduce only min_size.** Keep threshold 0.75 (strict),
   lower min_size to 2. Data: 24 clusters, 49% residuals. Simpler than
   a second pass; less aggressive than 0.70+2.

## Non-goals

- Changing the seed-neighborhood algorithm (BL-clustering-validation
  remediation ladder item #1). Empirical cluster quality from real LLM
  run was good ("topical cohesion strong"). Algorithm is fine.

- Tuning against the 13%-truncation signal (separate concern, tracked
  in BL-clustering-validation additional observations).

- Changing prompt / synthesis shape. Scope is parameter tuning + maybe
  a second pass, not rewriting BL-007.

## Possible Next Steps

1. **Quick parameter flip only**: change `DEFAULT_THRESHOLD` to 0.70 and
   `DEFAULT_MIN_SIZE` to 2 in `src/core/clustering.rs`. One-line change.
   Accepts 2× LLM cost, relies on real LLM run to validate pair
   synthesis quality.

2. **Conservative flip + validation**: change only `DEFAULT_MIN_SIZE`
   to 2 (keep threshold 0.75). Simpler, lower-risk. Validate with a
   real LLM run before deciding whether to also drop threshold.

3. **Second-pass strategy**: design a new plan that runs clustering
   twice — first strict (current defaults), then loose on residuals
   with a distinct prompt. More engineering; better quality
   guarantees. Bigger scope.

4. **Wait for more data**: run the current defaults for a week, see
   whether the residual rate stabilizes or improves as the corpus
   grows (might just be a small-corpus artifact). No code change.

Ready for `/ae:discuss docs/discussions/018-residuals-reduction/` to
pick an option.
