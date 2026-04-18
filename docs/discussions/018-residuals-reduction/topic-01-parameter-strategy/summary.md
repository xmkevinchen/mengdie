---
id: "01"
title: "Parameter strategy — which knob(s) to turn and by how much"
status: decided
current_round: 2
created: 2026-04-18
decision: "Option 2 (min_size flip) BUNDLED with null-escape-hatch from topic 2 — ship together as one plan. Flip DEFAULT_MIN_SIZE 3→2 in src/core/clustering.rs:23; keep DEFAULT_THRESHOLD at 0.75. Bundle ai-engineer's null-escape prompt change + SynthesisOutcome enum so the ~30% topically-adjacent pairs the flip would surface get LLM-rejected instead of synthesized. Doc comment on the constant cites discussion 018 + trigger: '3-5 dream runs; if skip rate > 25% of pairs OR manual spot-check shows majority-weak syntheses, revisit'. Accept challenger's synthesis_hit_rate protocol as BL-clustering-validation's replacement trigger metric (>50% residuals AND synthesis_hit_rate < 10% = signal). No code change for that — backlog wording update only."
rationale: "ai-engineer empirical spot-check (2026-04-18) on 10 pair-clusters from the production 214-memory corpus: 60% NEAR-DUP (plan↔backlog, discuss↔conclusion, analyze↔analyze pairs — exactly the dedup-with-provenance case mengdie synthesis is designed for), 30% TOPIC-ADJ (project-name + AE-boilerplate vocabulary overlap without shared intent — pure noise), 10% unclear. 60% near-dup is at the top of minimal-change's 'mixed' band (40-60%) — insufficient to ship Option 2 alone. But the 30% topic-adj share is EXACTLY what ai-engineer's null-escape-hatch filters: the LLM returns `{\"skip\": true, \"reason\": ...}` when the pair lacks a meaningful common thread. With escape bundled, ~90% of pair-clusters either synthesize usefully (60%) or get cleanly rejected (30%). Topic 2's 'defer null-escape' position was defensible pre-spot-check; post-data, it's wrong — the escape is load-bearing. Shipping Option 2 without the escape would degrade average synthesis quality even as total count rises. Challenger's synthesis_hit_rate is accepted as the next-iteration measurement that gates future threshold decisions; adopted in BL-clustering-validation wording. Option 2 + escape is 1 plan, 2 steps (clustering constant + synthesis parser/enum/counter changes)."
reversibility: "Zero-cost. One-line revert in clustering.rs. No schema change, no data migration. CLI --min-cluster-size flag lets any run opt back to 3 without a code change."
---

# Topic: Parameter strategy

## Current Status

Decided. See decision/rationale/reversibility fields above.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | — | Conservative flip preferred: min_size=2 only, keep threshold 0.75. No config file, no dynamic defaults. Reversibility clean. |
| 2 | — | Direction confirmed as Option 2. Doc comment on constant required: cite discussion 018, name trigger ("3–5 runs, spot-check pair quality"). Commit message must reference BL-clustering-validation.md. |
| 2 (revision) | — | Status downgraded from decided → contingent. Two open gates: (1) ai-engineer pair-cluster spot-check ≥70% near-dup required before shipping; (2) challenger's synthesis_hit_rate protocol accepted as replacement trigger metric in BL-clustering-validation, complementing residual-% signal. |
| 2 (final) | converged | Spot-check returned 60% near-dup, 30% topic-adj, 10% unclear. Below the strict 70% ship bar but 30% topic-adj is exactly what the null-escape-hatch filters. Decision: bundle null-escape with Option 2 (topic 2 flipped from defer → in-scope). Combined, ~90% of pair-clusters are usefully handled (60% synthesize, 30% cleanly rejected). One plan, two steps. |

## Context

Current defaults (shipped in BL-007, commit `6d52bda`): threshold 0.75,
min_cluster_size 3, max_cluster_size 20. They produce 57-67% residual
rates (see analysis.md for the % variation from corpus growth).

The data argues for reducing residuals, but:
- Lower threshold + lower min_size = more LLM calls per run (~2×)
- Lower threshold alone risks over-merging (cluster count collapses at 0.65)
- Min_size=2 produces "pair synthesis" — 1 LLM call to summarize 2 memories,
  borderline value vs just reading both

## Constraints

- No code scope change except `DEFAULT_THRESHOLD` and/or `DEFAULT_MIN_SIZE`
  constants in `src/core/clustering.rs:22-23`.
- The CLI already exposes `--threshold` and `--min-cluster-size` flags
  (BL-007), so defaults are just defaults — users can override per-run.
- Real LLM quality has only been validated at current defaults (13 of
  14 clusters produced good syntheses). No real-LLM quality data yet
  for looser parameters.

## Key Questions

1. **Which option from analysis.md § "Possible Next Steps" do we pick?**
   - Option 1: full flip to 0.70 + min_size=2 (aggressive, 2× LLM cost, 35% residuals)
   - Option 2: min_size=2 only, keep threshold 0.75 (conservative, 49% residuals)
   - Option 4: wait and re-measure after corpus grows (zero code, defer decision)

2. **Pair-cluster value**: is a 2-memory LLM synthesis worth 1 LLM call,
   or should we deliberately keep min_size=3 to avoid low-value calls?
   Claude Sonnet 4.6 synthesis of 2 paragraphs costs ~$0.005 per call —
   27 calls is trivial. The real cost is latency and run duration, not
   dollars.

3. **Over-clustering risk** at threshold 0.70: cluster count was 11
   (min=3) or 27 (min=2), no sign of merging (dropped to 9 only at
   0.65). Is 0.70 safe without empirical LLM quality validation?

4. **Commit vs iterate**: should we commit to one parameter set and run
   it, or design a CLI toggle story (e.g., `--preset conservative` /
   `--preset aggressive`)?
