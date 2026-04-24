---
id: "01"
title: "Which fix option(s) ship in the v0.8.0 synthesis plan"
status: converged
current_round: 2
created: 2026-04-23
decision: "Ship Option 1 (audit subcommand) + Option 4 reinterpreted as 'surface source_type field in CLI search/list output'. Defer Option 2 (LLM verification) and Option 3 (downrank). Reject Option 5 (new KnowledgeType::Synthesized enum variant) on axis-discipline grounds."
rationale: "Codex's axis-discipline argument decided the Option 5 debate: `knowledge_type` is epistemic (factual/experiential/decisional), `source_type` is provenance (conclusion/review/plan/synthesis). Syntheses are factually-shaped content with synthesized provenance; mixing 'synthesized' into knowledge_type conflates axes. Option 4 reinterpreted as 'surface source_type' (not hardcoded [SYN] prefix) respects both axes and requires no schema/enum change — synthesis rows already have source_type='synthesis' stored (dreaming.rs:564, db.rs:1064). Option 1 gives Kai a read-only audit path for fidelity spot-checks (BL-clustering-validation.md confirms 10/27 manually reviewed, no hallucinations observed — systematic audit needs a CLI surface). Options 2 and 3 defer per codex's data-gating rule: ship verification/downrank only when audited failure rate > 1/20 OR syntheses dominate top-5 regularly. At 40% corpus prevalence (27/68) with zero confirmed bad syntheses, both options would change behavior blindly against a clean corpus. Architect initially recommended 1+3+4 (multiplier 0.7) then recanted after codex's prevalence data; architect's Option 5 dissent preserved for the future case where epistemic-level search discriminator becomes needed."
reversibility: "high"
reversibility_basis: "No schema change, no enum change, no migration. Decision lives in src/bin/cli.rs formatting (2-10 LOC for Option 4; new Option 1 subcommand is a new file, fully revertable). Option 5 can be added later if axis-discipline call proves wrong under real search-behavior data; Option 3 can ship once bad-rate data justifies a multiplier."
---

# Topic: Which fix option(s) ship in the v0.8.0 synthesis plan

## Current Status

**Converged**: ship Option 1 + Option 4 (surface source_type), defer 2+3, reject 5. Architect dissent on Option 5 preserved as forward trigger.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 3-way split: architect 1+3+4, challenger +Option 5 root fix, codex 1+4 |
| 2 | converged | Option 3 unanimous defer after 40% prevalence data; Option 4 reinterpreted as "surface source_type" not title prefix; Option 5 rejected on axis discipline (codex), architect dissent preserved |

## Decision Details

### What ships in the v0.8.0 synthesis plan

1. **Option 1 — audit subcommand**: `mengdie synthesis audit <syn-id>` (or similar subcommand name TBD at plan-time) that prints synthesis content alongside the linked source memories. Operator reads both and judges fidelity.
   - Requires: `get_synthesis_sources(syn_id)` DB helper if not present, or reuse of existing FK join.
   - Blast radius: read-only, new CLI surface only.

2. **Option 4 reinterpreted — surface `source_type` in CLI output**: `mengdie search` and `mengdie list` output formatters gain a visible provenance line showing `source_type`. Syntheses will show `source_type: synthesis` (stored at `src/core/dreaming.rs:564`, `src/core/db.rs:1064`). Primary sources show `conclusion`/`review`/`plan`/`analysis`/`retrospect`.
   - Not a hardcoded `[SYN]` title prefix — rejected by all 3 agents implicitly (no title mutation).
   - Blast radius: ~2-10 LOC in `cli.rs` formatters.

### What defers

- **Option 2 (LLM verification)**: per-dream-pass LLM cost + same-family bias concerns + no observed hallucinations at sample size 10/27. Ship trigger per codex: audited failure rate > 1/20 OR syntheses dominate top-5 in real queries.
- **Option 3 (downrank)**: 40% prevalence makes any multiplier a systemic search-behavior change, not a nudge. Ship trigger: same as Option 2 OR operator reports synthesis rows ranking higher than expected primary sources.

### What is rejected

- **Option 5 (new `KnowledgeType::Synthesized` enum variant)**: axis-discipline violation. `knowledge_type` is epistemic; `source_type` is provenance. Syntheses should be classified by epistemic class (usually factual) with their provenance (synthesis) recorded on the separate axis. Architect's dissent preserved: if a future plan needs epistemic-level search discriminator and proves the axis-dilution cost is less than the co-landing pragmatism benefit, revisit.

## Key Questions — resolved

- **Are the 4 options pick-one, pick-combination, or minimum subset?** → partially orthogonal; picked Options 1 + 4 as minimum subset.
- **Is Option 2 in v0.8.0 scope?** → NO, defer per codex data-gating rule.
- **Does any option require a schema change co-landed with BL-synthesis-dedup-key's v5?** → NO, none of the shipping options touch the schema. No coupling.
- **If Option 3 ships, what's the multiplier?** → not applicable; Option 3 defers.
