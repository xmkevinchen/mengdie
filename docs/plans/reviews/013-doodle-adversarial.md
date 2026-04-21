---
plan: "013"
reviewer: doodlestein-adversarial
date: 2026-04-20
verdict: conditional-pass
---

# Adversarial Review — Plan 013 (BL-008 Exponential Decay)

## Where the plan first fails in real use

**Step 5 / AC7: `scripts/verify-decay.sh` per-memory breach list is undeliverable from `DreamingResult`.**

AC7 requires the script to print one line per breached memory:

```
BL-X title — last_recalled_age=Nd effective=0.XX
```

But `DreamingResult` only carries aggregate counters (`decay_floor_breaches: usize`, `demoted: usize`). It does not carry a `breached_entries: Vec<MemoryEntry>` (or even a `Vec<String>` of IDs). The shell script wraps `mengdie dream --decay-dry-run` and parses structured-JSON stderr — that JSON also only has the six numeric fields specified in AC5.

The implementer hits this during Step 5 and faces a forced choice not covered by any AC:

1. **Add `breached_entries: Vec<MemoryEntry>` to `DreamingResult`** — unplanned struct change, ripples to every caller listed in Step 2 (the `run_dreaming` wrapper, `cli.rs:215`, `tests/e2e.rs:92`), and requires the structured-JSON output to serialize the full entry list.
2. **Extend the structured-JSON line with a `breaches` array** — contradicts AC5's six-field contract; any downstream parser breaks.
3. **Have `verify-decay.sh` do a second DB query** — the script has no DB client; it is defined as a thin wrapper around the CLI.

None of these paths is pre-decided. The implementer must invent a solution mid-Step-5 that affects Steps 2, 4, and 5 simultaneously.

## Severity

**Blocker for Step 5 implementation, not just a test gap.** The approval-gate mechanic (AC7's `--i-reviewed-each`) is correct in intent but the data pipeline supporting it is missing. The fix is small: add `breached_ids: Vec<String>` (not full entries — IDs suffice if CLI does a follow-up lookup, or titles if the struct carries them) to `DreamingResult`, and extend the structured-JSON line with a `"breaches":[{...}]` array. But this must be decided before Step 2 locks the struct.

## Recommended fix before `/ae:work`

In Step 2's `DreamingResult` extension, add:

```rust
breached_ids: Vec<String>,   // memory IDs where should_demote() == true
```

In Step 4's structured-JSON output, extend the schema:

```json
{"event":"dreaming_pass", ..., "breaches":[{"id":"...","title":"...","last_recalled_age_days":N,"effective":0.XX}]}
```

AC5's six-field contract becomes a seven-field contract. AC7's script reads the `breaches` array from the JSON line — no second DB query needed.

This is a one-struct, one-JSON change that resolves the gap cleanly without reopening any concluded decisions.
