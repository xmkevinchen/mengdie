---
id: "01"
title: "Decay formula & constants"
status: converged
current_round: 2
created: 2026-04-20
decision: "effective = avg_relevance × 2^(-d/H), H=60 days, age input = last_recalled only, floor = 0.20"
rationale: "Architect + codex independently on exponential-half-life-60. Codex flagged exp(-d/H) ≠ half-life semantics; adopt 2^(-d/H) form (Rust: (2.0_f64).powf(-days/60.0)). Floor=0.20 from architect+challenger independent convergence; codex's 0.10 based on math slip. Archaeologist V2 (IQR=0.015) killed percentile-floor. Age input last_recalled only — no created_at fallback (decay only runs on is_longterm=1 memories, which always have last_recalled set by promotion predicate)."
reversibility: "medium"
reversibility_basis: "Formula is a pure function; half-life + floor are constants tunable in one PR. Revisit trigger: avg_effective_relevance <0.25 OR corpus age >90 days OR IQR widens past 0.05."
---

# Topic: Decay formula & constants

## Current Status
Converged at Round 2. Formula, half-life, age-input, and floor all locked.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | explore | Architect + codex independently picked H=60 exponential; floor debated 0.01→0.10→0.30 |
| 2 | converged | Formula `2^(-d/60)`, floor 0.20, last_recalled only |

## Context
BL-008 sketches `effective_relevance = avg_relevance × 0.95^days`. That's
one of many shapes: exponential (what BL-008 names), power-law proper
(`1/days^k`), linear-with-floor, step-function, half-life-parameterized.
The family and the constant together determine how quickly a memory's
effective weight decays and whether decay is smooth or cliff-edged.

## Constraints
- Must be a pure function of stored fields (`avg_relevance`,
  `last_recalled` and/or `created_at`) + current time — no new stored
  state for the formula itself.
- Must be monotonically non-increasing in elapsed time.
- Must be computable in SQL expressions or a post-fetch pass without
  pulling a new crate.
- The distribution of `avg_relevance` in production is compressed near 0.5
  (see prior-art.md §2); the formula must degrade gracefully on that
  distribution, not assume uniform [0,1].

## Key Questions
1. What shape (exponential, power-law proper, other) best matches the
   observed recall pattern? What empirical signal would we use to pick
   one?
2. What "age input" drives decay — time since `last_recalled`, time since
   `created_at`, or something else (e.g., elapsed Dreaming passes)?
3. What's the minimum viable first-ship formula vs. the ideal
   parameterization after 3 months of data?
4. Is there a single knob (half-life in days) that makes the design
   reviewable, vs. multiple free parameters that invite bikeshedding?
