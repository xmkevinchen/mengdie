---
id: "03"
title: "Reflection module collapse timing + Reflector trait introduction"
status: converged
current_round: 2
uag_passed: true
created: 2026-04-27
decision: "Defer Reflection module consolidation (clustering.rs + synthesis.rs + dreaming.rs) until sqlite-vec compatibility spike outcome is known — clustering.rs may be deleted entirely if ANN replaces it. Do NOT introduce Reflector trait in v0.0.1 regardless of sqlite-vec spike outcome — ANN is similarity-primitive swap, not 2nd reflection strategy."
rationale: "5-of-5 unanimous + falsification attempts unrefuted (UAG passed). 'Name v0.0.1 call site selecting between ≥2 reflection strategies at runtime' — none named. ANN swap doesn't change algorithm identity; trait abstracts strategies, not primitives."
reversibility: high
reversibility_basis: "Both sub-decisions reversible. Module consolidation can happen post-spike if clustering.rs survives. Reflector trait can be introduced if 2nd reflection strategy materializes."
---

# Topic: Reflection module collapse timing + Reflector trait introduction

## Current Status
**Converged via UAG** (Round 2). Defer consolidation pending sqlite-vec spike. NO Reflector trait in v0.0.1.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | exploratory | 5-of-5 defer collapse; 4-of-5 explicit Reflector NO; gemini implicit |
| 2 | converged (UAG) | 5-of-5 affirm both sub-decisions; falsification attempts unrefuted |

## Context

archaeologist's empirical finding: `src/core/clustering.rs` (626
LoC) and `src/core/synthesis.rs` (450 LoC) are imported exclusively
by `src/core/dreaming.rs` (1327 LoC). No other module imports them.
The proposed Reflection layer is therefore three modules that
collapse into one operation (the dream pass: cluster → synthesize →
store).

challenger argues the boundary between clustering and synthesis is
not an API boundary — it is a function call within a single
algorithm. There is no caller that uses one without the other.
Recommendation: collapse to a single module (keep `dreaming.rs`
name).

Pragmatic complication: blueprint §10 schedules a sqlite-vec
compatibility spike. If sqlite-vec adoption succeeds and ANN-based
similarity replaces hand-rolled clustering (per 025
CONDITIONAL-DELETE verdict for clustering.rs), `clustering.rs` may
be deleted entirely rather than merged. The collapse question would
be moot.

The decision: defer the collapse decision until the sqlite-vec
spike resolves, or commit to one direction now and revise if needed.

## Constraints

- sqlite-vec compatibility spike is part of v0.0.1 minimum (already
  filed in blueprint §10).
- 025 verdict for `clustering.rs` was CONDITIONAL-DELETE — only if
  ANN replaces it.
- Blueprint §11: BLs that don't trace to a blueprint section are
  scope creep.

## Related question — Reflector trait introduction

The analyze-phase verdict on `Reflector` trait was "defer with
trigger" (premature at v0.0.1; needs ≥2 reflection strategies). But
the trigger may fire **in-sprint**: if the sqlite-vec compatibility
spike succeeds, ANN-based similarity becomes a candidate 2nd
reflection strategy alongside the existing seed-neighborhood cosine
clustering, satisfying the "≥2 impls" condition. Round 1 may
legitimately re-open the Reflector trait introduction under this
topic.

## Key Questions

- Should `clustering.rs` / `synthesis.rs` / `dreaming.rs`
  consolidation be part of v0.0.1, or wait until the sqlite-vec
  spike outcome is known?
- If the sqlite-vec spike succeeds, is the operator committed to
  replacing hand-rolled clustering with sqlite-vec ANN, or might
  `clustering.rs` survive even after sqlite-vec adoption?
- If sqlite-vec adoption produces a 2nd reflection strategy, does
  the Reflector trait introduction get pulled into v0.0.1 scope?
  Or is the trait still premature even with 2 impls because the
  operator's actual call sites are stable enough to live without
  abstraction?
- What is the cost of deferring the collapse decision (vs.
  committing to one direction now)?
