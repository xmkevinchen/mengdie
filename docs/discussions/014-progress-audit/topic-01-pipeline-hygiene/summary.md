---
id: "01"
title: "Pipeline field hygiene — close stale discussion/plan frontmatter"
status: converged
current_round: 1
created: 2026-04-18
decision: "Single mechanical cleanup commit (may split CLAUDE.md for traceability). No new status enum. Plan 008 → `done` ONLY AFTER filing the 3 carried Step 3 items (ci.yml clippy+test) to `docs/backlog/BL-ci-full-clippy-test.md` with a concrete trigger (e.g., 'next time a CI-relevant PR lands or the 2026-Q2 milestone ships' — pick a concrete signal). Then flip 008 to `done` with a header note pointing at the backlog entry. Fix topic tables in 012 + 016 (stale `pending` rows). Update 016 index `plan:` field to list all 3 spawned plans (007/009/010). Flip 016 status to `done`. Add future rule to CLAUDE.md: ae:work completion commit must also update the parent discussion's status + pipeline fields. File BL-ae-work-closes-parent-discussion.md capturing the skill-level enforcement gap (the CLAUDE.md rule is advisory; real enforcement lives in the external ae:work skill definition which this repo doesn't own — backlog entry tracks the upstream improvement)."
rationale: "Archaeologist ground-truth scan confirmed most original audit items already self-corrected. Remaining drift is mechanical metadata. Challenger: this is archaeology, not cleanup — do not run a discuss cycle for decisions the audit already made. Architect: single commit is correct; new status enum for 'partial delivery with carry' is over-engineering an exception. Doodlestein-adversarial flagged the blunder: 'done with header note' silently drops the carried CI work with no tracking — must file to backlog with trigger before closing. Doodlestein-strategic flagged that the ae:work-closes-parent-discussion rule in CLAUDE.md alone isn't enforced at execution time; file as an upstream backlog item for the AE plugin. Doodlestein-regret: none on this topic."
reversibility: "high — metadata-only changes, all revertable via git"
reversibility_basis: "No behavioral code changes; frontmatter + doc edits only; backout = single revert"
---

# Topic: Pipeline field hygiene — close stale discussion/plan frontmatter

## Current Status

Round 1 team converged on mechanical cleanup. Decision rows captured in frontmatter.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | converged | Single mechanical cleanup commit; plan 008 → done with scope-down note; 012/016 topic tables fixed; new "ae:work closes parent discussion" rule added to CLAUDE.md |
| 1 (amended post-Doodlestein) | converged | File BL-ci-full-clippy-test.md before closing plan 008 (Doodlestein-adversarial); file BL-ae-work-closes-parent-discussion.md for the upstream skill enforcement gap (Doodlestein-strategic) |

## Context

Stale pipeline frontmatter confuses `/ae:dashboard` and `/ae:next` — they treat "active" + "work pending" as real work-in-progress when in fact some items are closed by sibling plans or explicitly carried. This was the original motivation for the audit. The audit's proposed fix list is now partially stale because 3 more plans shipped since.

## Constraints

- Plan 008 has 3 intentionally carried pending steps (scope-down in commit `df7ba2d`). Marking plan 008 `done` would misrepresent reality.
- Plan 001 is a special case — audit notes "stuck at the gate state despite all work being finished." All reviews PASS.
- Discussion 016 (dreaming evolution) spawned two plans (007 done + 010 done). Both ship under its umbrella, so `status` should flip to `done`/`concluded` and `pipeline.work: done` is already correct.
- ae:work skill's Completion Invariant (in docs/skills) says: do NOT set plan status to done — leave as `reviewed` for /ae:review to promote to `done`. So plans that completed ae:review should be `done`; plans that completed ae:work but skipped ae:review stay `reviewed`.

## Key Questions

1. What's the current complete list of stale fields (audit list + drift since 2026-04-16)?
2. For plan 008 — is the right state "reviewed with 3 pending (intentional carry)" OR should the 3 pending be removed from the plan entirely and filed to backlog, allowing status=done?
3. For plan 001 — why didn't ae:review promote it to `done`? Is the `/ae:review` → `status: done` promotion step missing from older reviews, or was plan 001 pre-protocol?
4. Should this cleanup be a single mechanical commit or split by semantic theme?
