---
id: "023"
stage: framing
created: 2026-04-27
round_0: overridden
round_0_override_reason: "User override after attempt 3. Codex + minimal-change-engineer both APPROVED on attempt 3 (the two reviewers who pushed back hardest in attempts 1-2 — strongest discipline signal). Remaining 3 REVISEs were each different polish concerns that didn't converge on a single problem, indicating reviewer-aesthetic divergence rather than a structural framing defect. Strategic's factual bug was fixed inline before override. Per spec rerun-limit (3 attempts) and override-with-reason path."
round_0_reviewers: [codex-proxy, gemini-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer]
round_0_notes: "Attempt 1: 4 REVISE + 1 unavailable. Attempt 2: 4 APPROVED + 1 REVISE (Q1 binary). Attempt 3: 2 APPROVED + 3 REVISE (each different polish — strategic bug, adversarial list-anchoring, gemini title+constraint placement). Strategic's factual bug fix applied autonomously (cadence model still requires Q2/Q3, doesn't dissolve them). Adversarial + gemini REVISEs left for user decision: their concerns are real but reviewers disagree about whether they justify another revise pass."
---

# Framing — v0.8.5: should we ship it, and if yes what's in it?

## Problem Statement

mengdie v0.8.0 closed 2026-04-24. The next named anchor on the roadmap is
v0.9.0 / BL-009 (MCP Dream Tool — bring Claude into the synthesis loop
in-session).

Three real questions, in this order:

1. **What's the right delivery-unit shape here?** The space includes at
   least three points reviewers are free to argue for — and a hybrid is
   plausibly the right answer:
   - **Continuous, no version tags** — work flows to main; releases are
     just "current main"; no version markers between v0.8.0 and v0.9.0.
   - **Curated sprint with version tag** — pick a BL set, ship it
     together as v0.8.5, follow the discuss/plan/work/review cycle.
   - **Cadence- or threshold-triggered tag** — work flows continuously,
     but a version tag fires when N PRs accumulate or T days pass
     (CHANGELOG-accumulation model).
   These shapes are not exhaustive; reviewers may argue for a fourth.
   If the continuous (no-tag) model wins, the downstream questions
   dissolve. The cadence/threshold-triggered model still lands a
   version tag and still requires Q2/Q3 reasoning. The volume of
   fired-trigger work surfaced in Analysis 023 is one input to this
   choice, but reviewers should hold delivery-unit reasoning
   independent of any specific BL until Q2/Q3.

2. **If sprint-as-unit is right: should we cut v0.8.5 between v0.8.0 and
   v0.9.0, or skip straight to v0.9.0?** "Skip v0.8.5" is genuinely on
   the table — discussion 022's conclusion already named v0.9.0 as next
   destination. The argument for v0.8.5 has to come from real fired-trigger
   work, not from "we should have a sprint between releases."

3. **If we cut v0.8.5: which BLs belong in it?** Subject to the
   trigger-discipline rule from discussion 021 (don't schedule items
   whose trigger conditions haven't fired). Analysis 023 produced an
   inventory of 9 unscheduled BLs and judged each one's trigger status —
   reviewers should reach their own conclusions, not anchor on the
   analysis's recommendation.

## Scope

In:
- Delivery-unit choice (continuous vs sprint)
- Schedule decision (v0.8.5 yes/no, or skip-to-v0.9.0)
- BL selection if v0.8.5 ships

Out:
- BL implementation design (that's `/ae:plan`)
- v0.9.0 contents beyond the named BL-009 anchor
- CLAUDE.md cleanup + production v5 migration (those are independent ops
  tasks owed regardless of this decision; not part of the framing)

## Reference Material

Three load-bearing inputs:
- **Analysis 023** (`analysis.md` in this directory) — feature inventory
  + per-BL trigger-status judgment + paths surfaced. Reviewers should
  read selectively for evidence, not anchor on its recommendations.
- **The 9 unscheduled BL files** (`.ae/backlog/unscheduled/BL-*.md`) —
  source of truth for trigger conditions per BL.
- **Discussion 021 trigger-discipline rule**
  (`docs/discussions/021-v0.8.0-bl-dependencies/conclusion.md`) — the
  rule that governs which BLs can be scheduled.

Optional context if relevant to a specific argument:
- Discussion 022 conclusion (named v0.9.0 as next).
- Industry convention for 0.x.5 patch positions in Rust ecosystem
  (tantivy/PyO3/tokio).
