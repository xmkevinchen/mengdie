---
id: "021"
stage: framing
created: 2026-04-23
round_0: overridden
round_0_reviewers: [codex-proxy, gemini-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer]
round_0_notes: |
  v1: 4 REVISEs (codex bias-language, gemini bias-language, doodle-strategic
  higher-leverage sprint-commitment reframe, doodle-adversarial BL-010
  in/out distinction) + 1 unavailable (minimal-change-engineer timed out).
  Edits applied inline to framing + Topic 4 added per strategic reframe.
  Override rationale (auto mode + user signaled process-proportionality
  concern in plan 014 review): all 4 REVISEs were surgical non-conflicting
  edits; v2 re-run would be ceremony. If any edit has unintended
  consequences, Round 1 will surface them and the team can revise the
  framing then. Per-agent verdicts in round-00/.
---

# Framing — v0.8.0 remaining-BL execution shape

## Problem Statement

After plan 014 closed 2 of 9 items in sprint v0.8.0 (`006-ci-runner-env-cleanup`
and `BL-ci-full-clippy-test`), seven items remain open. The dependency
analysis (`docs/discussions/021-v0.8.0-bl-dependencies/analysis.md`,
2026-04-23) showed:

- Decay and synthesis subsystems don't interact — two independent clusters
- 3 dependency edges, all within the decay cluster (schema contract
  precedes its consumers)
- 2 items are explicitly "defer until trigger" per their own bodies —
  `BL-decay-dreaming-pass-optim` (scale trigger) and
  `BL-synthesis-preload-db-miss-edge` (delete-path trigger)
- 5 items appear actively workable; 3 were identified as forming a
  coherent potential grouping ("decay operator surface hardening"), but
  the discussion should decide whether they ship together or separately

Two intertwined questions surface: **execution shape** (how the 5
workable items partition into plans) and **sprint-commitment semantics**
(what v0.8.0 does with the 2 items whose triggers haven't fired). Per
doodlestein-strategic's Round 0 feedback, the commitment-semantics
question may be higher-leverage: the 2 deferred items reveal that v0.8.0
committed to items whose own bodies said "not now" — that's a policy
signal, not just a housekeeping task.

This discussion decides both: the execution shape for the next
`/ae:plan` invocation AND the policy for how v0.8.0 treats committed
items with unresolved pre-conditions. It does not re-litigate
individual BLs' content, trigger conditions, or fix options — those
are already recorded in each BL's body.

## Scope

In:
- How to account for the 2 defer-until-trigger items currently assigned
  to v0.8.0 when their triggers have not fired (keep, remove via
  `/ae:roadmap remove`, close as superseded-by-trigger, other)
- Policy for closing v0.8.0 when committed items have unresolved
  pre-conditions — does sprint-close require all committed items or
  all eligible items?
- Whether the 3 active decay BLs ship as one plan, multiple plans, or
  some other grouping
- Which of the 5 internal hardening actions inside
  `BL-verify-decay-script-hardening` ship in the v0.8.0 plan vs. defer
  to their own triggers. The `--threshold daemon mode` sub-action has
  an explicit relationship to future BL-010 scope; evaluating its
  stub-vs-defer call on fit with the current decay operator surface
  is **in-scope**, but BL-010 internal design is not.

Out:
- Changing the decay mechanism design recorded in discussion 019
- Changing the synthesis mechanism design (separate BL cluster for
  later plan)
- BL-010 daemon internal design (future sprint)
- Changes to any individual BL's fix option (each BL's body is the
  source of truth)

## Reference Material

- `docs/discussions/021-v0.8.0-bl-dependencies/analysis.md` — the
  dependency-map analysis that surfaced the 3 questions
- `.ae/backlog/v0.8.0/BL-decay-json-schema-contract.md`
- `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md`
- `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md`
- `.ae/backlog/v0.8.0/BL-decay-dreaming-pass-optim.md`
- `.ae/backlog/v0.8.0/BL-synthesis-preload-db-miss-edge.md`
- `.ae/roadmaps/v0.8.0.md` — sprint roadmap with initial commitment
