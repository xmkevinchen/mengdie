---
id: BL-decay-ops-doc-polish
status: open
origin: BL-008 /ae:review (gemini P2 + challenger C4)
created: 2026-04-20
scope: mengdie (operator procedure completeness)
---

# Polish `dreaming-decay.md` — actionable approval gate + arrow fallback

## What

Two operator-facing gaps in the ops doc shipped in plan 013 Step 4:

1. **Approval gate not actionable** (Gemini P2): the first-run procedure
   step 3 says "if `decay_floor_breaches > max(10, 10% of long-term count)`,
   HALT" but provides no command to retrieve the long-term count. An
   operator reads the doc, sees the breach count in the script output,
   and then has to go find a separate query to compute the threshold.
2. **Arrow regex ASCII fallback promised, not delivered** (Challenger
   C4 LOW-MEDIUM): plan 013 AC5 documents the arrow as `(?:→|->)` —
   tolerate both Unicode and ASCII. Current format string only emits
   `→`. Not a bug today but a promised axis of robustness that didn't
   land. If a terminal / pipeline eats the Unicode, the human line
   breaks.
3. **Missing rollback procedure** (Gemini P2): no guidance on what to
   do if demotion turns out to be too aggressive post-fact — e.g., "run
   `UPDATE memory_entries SET is_longterm = 1 WHERE id IN (<breached list>)`"
   with the IDs from the previous pass's JSON event.

## Why

Operator experience. These gaps don't block the ship — the current doc
carries enough information for Kai (the implementer) to run the system.
But the whole point of shipping the ops doc is that future operators
(or Kai-in-6-months) can run it without re-reading the code.

## How to apply

1. Add a ready-to-copy SQL snippet to the first-run procedure:
   ```sql
   SELECT COUNT(*) FROM memory_entries
     WHERE is_longterm = 1 AND valid_until IS NULL;
   ```
   Plus a helper bash line that pipes it into the threshold comparison.
2. Emit both `→` and `->` in the CLI format string (or commit to one
   and update the AC + plan). If keeping `→`, remove the ASCII fallback
   from the AC regex so the contract matches reality.
3. Add a "Rollback" section to `dreaming-decay.md` explaining how to
   re-promote memories from the previous pass's `breaches[]` list if
   an over-aggressive demotion is detected. Use a SQL snippet parameterized
   on the JSON list.

## Trigger

First of:
- First operator who isn't Kai runs `verify-decay.sh` and asks for the
  threshold command.
- First aggressive-demotion incident requires rollback.
- Any corpus-size-related surprise where the 10% heuristic misfires.
