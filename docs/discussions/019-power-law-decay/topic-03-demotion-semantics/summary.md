---
id: "03"
title: "Demotion semantics & threshold"
status: converged
current_round: 2
created: 2026-04-20
decision: "Demotion is asymmetric with promotion. Demote iff (is_longterm = 1 AND last_recalled IS NOT NULL AND effective < 0.20). Clears is_longterm (no new was_longterm state). Memories with last_recalled IS NULL are skipped entirely (no decay, no demotion)."
rationale: "Asymmetric rule provides natural hysteresis — promotion requires last_recalled within 14 days AND recall_count >=3 AND avg_relevance >=0.45; demotion requires ~75 days of silence at floor=0.20. Gap is wide enough to prevent flapping by construction. Clearing is_longterm is the minimum change that produces user-visible effect (removes 1.2× LONGTERM_BOOST at search.rs:142). New `was_longterm` state would require schema migration, exceeding scope. Archaeologist V3: zero demotions on first pass under any floor — design is empirically safe to ship."
reversibility: "medium"
reversibility_basis: "Threshold is a constant. Full reversal requires a migration to add was_longterm — deferred to BL if needed. Clearing-is_longterm path is reversible in one PR."
---

# Topic: Demotion semantics & threshold

## Current Status
Converged at Round 2. Asymmetric rule, floor=0.20, natural hysteresis, skip-if-null-recall.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | explore | Architect: asymmetric, no new state. Codex: asymmetric with separate floor. Challenger: decay-only first ship? |
| 2 | converged | Challenger conceded on empirical first-pass safety. Floor=0.20 via evidence preponderance. |

## Context
BL-008 proposes: demotion when `effective < 0.01` → clear `is_longterm`.
That's one option among several: (a) demote on decay floor as proposed,
(b) demote only if memory fails to meet the ORIGINAL promotion
thresholds under the decayed value, (c) never demote, only avoid
re-promotion (one-way promotion + decay on the weight), (d) add a new
state `was_longterm` rather than silently clearing the flag.

## Constraints
- Hard: stored `avg_relevance` is never mutated — demotion cannot be a
  "reset relevance to 0" trick.
- Any demotion signal must be idempotent (running Dreaming twice in a
  day doesn't cause flapping).
- Existing schema has only `is_longterm` as a 0/1 flag — any richer
  state requires a migration, which pushes past the ~50-100 LOC scope.

## Key Questions
1. Is demotion symmetric with promotion (same signal, inverted
   threshold) or a distinct concept?
2. Does demotion clear `is_longterm`, or does it introduce a new
   observable state? If new state, is that in scope for BL-008?
3. What prevents flapping — memory promoted Monday, demoted Tuesday,
   promoted Wednesday? Is a hysteresis band needed, or do the
   thresholds naturally avoid it?
4. What's the correct threshold value given the observed distribution
   of `avg_relevance` (compressed near 0.5, not uniform)?
5. Is demotion necessary for BL-008 to deliver value, or could a
   decay-only (no-demotion) first ship work?
