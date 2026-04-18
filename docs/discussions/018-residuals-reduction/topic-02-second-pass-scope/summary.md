---
id: "02"
title: "Second-pass strategy — scope and value"
status: decided
current_round: 2
created: 2026-04-18
decision: "SPLIT: (a) Second-pass clustering strategy → DEFER as originally concluded; unchanged. Not a plan item until topic 1 data proves it necessary. If built: extend run_synthesis_pass with Option<SecondPassConfig>, no new file or subcommand. (b) Null-escape-hatch (LLM returns {\"skip\": true, ...} on weak pair-clusters) → PROMOTED from defer to IN SCOPE, bundled into topic-01's plan as step 2. Reversed after ai-engineer's spot-check data (60% near-dup + 30% topic-adj) showed the escape is load-bearing for topic-01's min_size=2 flip to ship cleanly."
rationale: "Second-pass: still premature. No empirical case built for running a second clustering pass on residuals; topic-01's Option 2 + null-escape combo covers the near-duplicate surface. Revisit only if 3-5 dream runs post-flip show pair-syntheses routinely skipping via null-escape AND the residuals at 0.75 still contain recoverable groups at looser parameters. Null-escape-hatch (promoted): architect's Round 2 argument 'do not bundle, contaminates the measurement signal' was defensible pre-spot-check — a bundled prompt change would conflate parameter effect with prompt effect. Post-spot-check, the 30% topic-adj share is evidence that Option 2 WITHOUT escape ships known-noise outputs. The measurement signal is already contaminated by default; bundling the escape is the ship-quality-neutral choice. The bundle is still reviewable as 2 separate steps in the same plan, each with its own AC. If quality analysis post-ship wants to separate parameter effect from prompt effect, that is a diff analysis on synthesis output, not a code sequencing choice."
reversibility: "Second-pass: N/A — not built. Null-escape-hatch: moderate. If the escape turns out to over-reject (skip rate > 25% becomes the re-review trigger), the prompt edit reverts cleanly but the SynthesisOutcome enum + counter add a small API surface. Acceptable cost given the quality-gate payoff. Reversible by: (a) reverting the prompt string, (b) leaving the enum in place as dead-code until a future plan needs it (zero runtime cost), OR (c) full revert of the enum via one commit."
---

# Topic: Second-pass strategy — do we need one, or is parameter tuning enough?

## Current Status

Decided: defer. See decision/rationale fields above.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | — | Defer topic 2 until topic 1 data is in hand. If built: Option<SecondPassConfig> on run_synthesis_pass, not a new file or subcommand. Two distinct prompt builders (or SynthesisMode enum). No schema change needed. |
| 2 | — | Null-escape hatch (LLM returns null for weak pairs) also deferred — option (c). Trigger: majority of 2-member syntheses prove low-value after 3–5 runs. If built: SynthesisMode enum on existing builder, LlmRejected error variant, not bundled with parameter flip. |
| 2 (final) | converged | Null-escape-hatch PROMOTED to in-scope, BUNDLED into topic-01's plan. Reversal driver: ai-engineer pair-cluster spot-check found 30% topic-adj share — exactly what the escape filters. Shipping Option 2 without escape = shipping known-noise. Second-pass decision unchanged (still deferred). |

## Context

The second-pass strategy preserves high-quality dense clusters from
the strict first pass while giving residuals a second chance at
looser parameters. But it adds engineering cost:

- A new `run_synthesis_pass` variant (or an additional function) for
  the second pass
- Distinct prompt for pair/loose synthesis (maybe: "Note these two
  related memories and their common thread, but don't over-synthesize")
- Extra test surface (unit tests for second-pass flow, integration
  test for end-to-end)
- CLI flag additions (`--second-pass`, `--second-pass-threshold`, etc.)
- Plan-level scope: ~3 steps similar to BL-007 Steps 1/2/3

## Constraints

- Topic 1's decision affects this: if parameter tuning alone drops
  residuals to ~35%, the incremental value of a second pass is small.
  If parameter tuning stays at ~50%, second pass may be the cleaner
  answer.
- The "describe layers, not identifiers" rule from progress-audit (014
  topic-03) applies: any new surface we introduce becomes docs drift
  if we name it in CLAUDE.md — prefer functional descriptions.

## Key Questions

1. **Should this topic even be discussed now?** Or does it depend on
   topic 1 outcome? If topic 1 picks option 1 (0.70 + 2), run that for
   a pass or two and re-measure before entertaining a second pass. If
   topic 1 picks option 2 or 4 (conservative), second pass might be
   needed sooner.

2. **Value proposition**: a second pass at 0.65 + 2 on residuals
   would theoretically drop 83 → ~50 (guess). Is that 33-memory
   recovery worth 3 steps of engineering?

3. **Alternative designs**: instead of a true "second pass", consider:
   - Per-cluster prompt tier (pair-cluster gets a different prompt; no
     structural change, just prompt branching in `synthesis.rs`)
   - "Miscellaneous" cluster: group N residuals into one synthesis
     with explicit "these are unrelated memories grouped only because
     none clustered elsewhere" framing
   - Skip this entirely; accept that 30-35% residuals is a natural
     property of a dense engineering corpus
