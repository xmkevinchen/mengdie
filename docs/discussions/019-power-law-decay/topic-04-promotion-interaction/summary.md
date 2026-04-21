---
id: "04"
title: "Interaction with existing promotion thresholds"
status: converged
current_round: 2
created: 2026-04-20
decision: "Promotion predicate UNCHANGED. Reads raw avg_relevance (>= 0.45), keeps recall_count >= 3 gate, keeps last_recalled within 14 days gate. Decay affects demotion only. BL-008 scope claim narrowed: addresses staleness; does NOT claim to fix recall_count burst inflation (prior-art §1)."
rationale: "Codex computation: switching promotion to effective_relevance would demote a 15-day-old memory with avg=0.50 (effective = 0.50 × 2^(-15/60) = 0.420, below 0.45). That causes immediate mass-demotion of the 41 current long-term memories. Keeping promotion on raw avg_relevance makes BL-008 a single-surface addition with no disruption to the existing 323-memory corpus. Architect R1 + codex R2 independent convergence."
reversibility: "high"
reversibility_basis: "Zero code change in promotion path; only additive change in demotion path. Fully reversible by skipping the demotion code path."
---

# Topic: Interaction with existing promotion

## Current Status
Converged at Round 2. Promotion logic untouched; decay scope narrowed to staleness only.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | explore | Architect: keep promotion unchanged. Codex: implicit asymmetric. |
| 2 | converged | Codex explicit agreement; burst-inflation scope-narrowed per challenger option (a). |

## Context
Today's promotion (src/core/dreaming.rs):
`recall_count ≥ 3 AND avg_relevance ≥ 0.45 AND last_recalled ≥ now - 14d`.
Adding decay raises the question: does the promotion comparator switch
from `avg_relevance ≥ 0.45` to `effective_relevance ≥ 0.45`? If so, the
`last_recalled >= 14d` predicate becomes partially redundant (decay
already penalizes stale recalls). If not, promotion ignores decay and
only demotion uses it — a design that's asymmetric in a specific way.

## Constraints
- `last_recalled >= now - 14d` is a hard cutoff (boolean), decay is
  continuous. They're not semantically identical — removing one to add
  the other is a behavior change.
- Plan 004's tuning fixed `min_relevance = 0.45` based on observed
  clamping at 0.47–0.50. Introducing a multiplier < 1.0 on the left side
  can reduce the number of eligible candidates dramatically on the
  first pass.
- BL-008 scope (~50–100 LOC) is tight — compound changes (promotion
  predicate change + demotion + decay formula) may not fit.

## Key Questions
1. Does promotion read `effective_relevance` or keep reading
   `avg_relevance`?
2. If promotion reads `effective`, does the 14-day recency window stay
   or is it now redundant?
3. What migration concern exists — existing ~238 memories were
   promoted under old rules; will the next Dreaming pass cause mass
   demotion if the predicate flips?
4. Is there a version of BL-008 that only affects demotion (promotion
   logic unchanged) — as a safer first ship?
