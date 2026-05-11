---
agent: doodlestein-adversarial
review_angle: Round 1 wall / blocked solution classes
verdict_state: REVISE
rerun: 2
timestamp: 2026-05-06T01:36:39Z
---

# doodlestein-adversarial verdict (rerun #2)

**REVISE**: framing.md lists 4 candidates; summary.md lists 5 — "on-demand" is missing from framing.

## Wall

framing.md Topic 2 (lines 86–89) enumerates exactly four candidates: cron-only / salience-threshold / composite / debounced. "on-demand" does not appear.

summary.md Practical constraint section (lines 63–73) lists five candidates: cron / on-demand / salience-threshold / composite / debounced.

This is a real Round 1 wall. framing.md is the gate document agents read first. An agent reading framing first will see 4 options; when it then reads summary.md, "on-demand" appears as a surprise fifth item. The risk: agents anchored on framing's 4-option list may treat on-demand as an editorial omission (i.e., assumed excluded) rather than a legitimate candidate. Especially because summary.md identifies the narrowed fallback as "cron and on-demand" — that fallback scenario is invisible from framing.

## Other two checks: PASS

1. summary.md feasibility is genuinely left open. No pre-answering.
2. Ratify-intent paragraph does not create a wall.

## Suggested edit

In framing.md Topic 2 bullet, add "on-demand" as a fifth candidate:

> **on-demand** (explicit operator-invoked trigger; zero new metrics)

This makes the option set in framing.md identical to summary.md and preserves the tractability constraint sentence that follows unchanged.
