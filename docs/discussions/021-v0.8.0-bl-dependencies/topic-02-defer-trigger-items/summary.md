---
id: "02"
title: "Fate of the 2 defer-until-trigger items in v0.8.0"
status: converged
current_round: 2
created: 2026-04-23
decision: "/ae:roadmap remove both BL-decay-dreaming-pass-optim and BL-synthesis-preload-db-miss-edge from v0.8.0. Both commands + manual gate-text update in ONE commit to avoid atomicity risk. --reason fields cite trigger-not-fired."
rationale: "Unanimous Round 2 (minimal-change-engineer conceded from 'do nothing'). Archaeologist verified: (1) /ae:roadmap remove is non-destructive — moves BL intact to unscheduled/, appends `descope` Notes entry, file body fully preserved; (2) /ae:roadmap close uses warn-by-default (not refuse) on open items, so 'do nothing' is technically viable, but items would stay in done/v0.8.0/ on archive which is wrong tier for trigger-gated items; (3) current v0.8.0 gate language 'All 4 BL-decay-* / All 3 BL-synthesis-*' becomes factually wrong after remove (challenger's missing-topic blind spot confirmed). Remove + gate-text update in one commit is the atomic fix."
reversibility: "high"
reversibility_basis: "Remove is non-destructive per archaeologist SKILL.md read. Can /ae:roadmap add-back the BL to v0.8.0 if trigger fires during sprint. BL file body fully preserved with trigger conditions intact."
---

# Topic: Fate of the 2 defer-until-trigger items in v0.8.0

## Current Status

**Converged**: remove both, update gate text, all in one commit.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 4 lean remove (architect, codex, challenger-leaning, archaeologist evidence-only), 1 do-nothing (minimal-change), 1 needs-fact-check (gemini) |
| 2 | converged | Archaeologist verified roadmap-remove is non-destructive + gate counts break after remove. minimal-change conceded. Unanimous remove both. |

## Decision Details

**Action sequence (all in one commit)**:

1. `/ae:roadmap remove BL-decay-dreaming-pass-optim --reason "body says not-now; triggers (corpus>50k, p95>1s, BL-010 daemon) are not v0.8.0 scope"`
2. `/ae:roadmap remove BL-synthesis-preload-db-miss-edge --reason "body says filed-for-trigger; trigger (mengdie delete/memory_invalidate CLI subcommand) does not exist — verified absent from Commands enum"`
3. Manual edit of `.ae/roadmaps/v0.8.0.md` gate text: update "All 4 BL-decay-* closed" → "All 3 BL-decay-* closed" and "All 3 BL-synthesis-* closed" → "All 2 BL-synthesis-* closed" (new counts reflect post-remove state). Cite discussion 021 in the Notes.

**Atomicity**: both removes + gate-text update must land in the same commit. If any step fails, revert the commit — don't leave the sprint in a half-removed state (challenger's Round 2 blind spot).

**Rejected alternatives**:
- Close as "superseded-by-trigger": wrong semantics — the items' concerns remain valid, just not yet triggered.
- Do nothing (leave open): minimal-change's original position, conceded in Round 2 because "All 7 closed" gate text can't be satisfied without wontfix-closing the items, which loses trigger context.

## Sub-question resolution: sprint-gate text

Gate text uses prefix counts and must be updated at remove time. Archaeologist confirmed SKILL.md does not auto-update gate text. This is a manual edit in the same commit as the removes.
