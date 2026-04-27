---
agent: minimal-change-engineer
round: 0
verdict: REVISE
timestamp: 2026-04-27T17:42:00Z
---

# minimal-change-engineer — framing Round 0 verdict: REVISE (mild)

Framing is close to minimal but over-structures the decision and over-cites references.

## Over-engineering found

**1. Three axes collapse into a sequential binary.**
- Existence (ship v0.8.5?) is the real decision.
- Shape (what category?) and Specific items (which BLs?) are the same question — you can't pick a category without picking items, and vice versa.
- Structurally this is: (a) yes/no, then (b) if yes, which BLs. Two questions, not three.

**2. Reference Material (7 bullets) anchors too broadly.**
For a binary schedule-or-skip decision, the load-bearing inputs are exactly three:
- Analysis 023 (inventory + paths A/B/C/D)
- The 9 unscheduled BL files
- Discussion 021's trigger-discipline rule

The other four (022, CHANGELOG, CLAUDE.md staleness, Phase 2 chain doc, prod orphan row) are inputs to *what v0.8.5 contains if it ships*, not inputs to *whether it ships*.

**3. CLAUDE.md staleness + orphan synthesis row don't belong in framing.**
The framing itself flags both as "independent of this discussion." If independent, they're noise here.

## Suggested minimal framing

> Problem: v0.8.0 closed; v0.9.0 anchor is BL-009. Should we ship a v0.8.5 sprint between them, or skip directly to v0.9.0? If yes, which of the 9 unscheduled BLs (per Analysis 023) belong in it, respecting discussion 021's trigger-discipline rule?
>
> Scope: in — schedule decision + BL selection if yes. Out — BL implementation design (that's /ae:plan), v0.9.0 contents beyond BL-009.
>
> Reference: Analysis 023, the 9 BL files, discussion 021's trigger rule.

~10 lines vs current ~70. Same problem coverage, less anchoring, no false trichotomy.

## What to keep as-is
- Problem statement opener (v0.8.0 just closed, v0.9.0 anchor named) — good context.
- Out-of-scope items — correctly bounded.
- The observation that "the disagreement itself is signal" from Analysis 023 — useful framing.
