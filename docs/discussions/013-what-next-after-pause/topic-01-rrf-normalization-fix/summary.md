---
id: "01"
title: "RRF Score Normalization Fix"
status: converged
current_round: 1
created: 2026-04-16
decision: "Option A: lower DEFAULT_MIN_RELEVANCE from 0.65 to 0.45, keep RRF_MAX=2/61, no avg_relevance reset"
rationale: "Option B (RRF_MAX=1/61) destroys dual-signal semantics (math proof). Option A preserves 0.5 vs 1.0 differentiation. 11 entries immediately qualify — all verified as genuinely useful (UAG passed)."
reversibility: "high"
reversibility_basis: "One constant change in dreaming.rs, can be tuned further with empirical data"
---

# Topic: RRF Score Normalization Fix

## Current Status
Pending — analysis confirmed Dreaming is permanently inert due to RRF normalization ceiling.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
RRF normalization divides by `RRF_MAX = 2/61` (dual-ranker theoretical max). Single-ranker rank-1 normalizes to ~0.5. Dreaming threshold is 0.65 — permanently unreachable. All 46 memories score 0.47-0.50. Zero promotions. This breaks the "spiral upward" core loop.

Three fix options identified in analysis:
- Option A: Lower threshold to 0.45 (quick hack, loses dual-signal semantics)
- Option B: Change RRF_MAX to single-ranker max (1/61), rescale to 0-1 (preserves semantics, ~5 lines)
- Option C: Decouple Dreaming threshold from RRF normalization entirely (architectural)

## Constraints
- Dreaming's `is_longterm` boost (1.2x) is already wired into search.rs — any normalization change must not break this
- Score is exposed to MCP callers via `min_score` parameter — callers interpret scores as 0-1 relevance
- Dreaming batch runs daily (launchd) — no hot-reload requirement
- 46 existing memories have `avg_relevance` values recorded at current scale — changing normalization affects historical data interpretation

## Key Questions
- Does the normalization change need to be backward-compatible with existing avg_relevance values in the DB?
- Should dual-ranker agreement still produce higher scores than single-ranker, or should normalization just map to 0-1 regardless?
- Is this coupled to the FTS5 fix (Topic 02)? If FTS5 starts working, dual-ranker hits become possible — does that change the fix approach?
