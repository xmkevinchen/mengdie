---
id: BL-decay-dreaming-pass-optim
status: open
origin: BL-008 /ae:review (performance-reviewer P2 + challenger C6 MEDIUM)
created: 2026-04-20
scope: mengdie (daily Dreaming pass runtime when corpus scales)
---

# Eliminate second long-term SELECT in `run_dreaming_with_config`

## What

Two performance-reviewer findings that collapse to one fix:

1. **Redundant post-demotion SELECT**: after the UPDATE chunk loop, the
   function runs a second SELECT over the surviving `is_longterm = 1`
   set solely to compute `avg_effective_score_after`. At corpus scale
   this is acceptable (<5 ms at dozens-to-low-hundreds of long-term
   memories), but it's wasted work — the post-state mean can be derived
   in-memory from the already-loaded `longterm_rows` set minus the
   `breached_ids` set.
2. **No zero-breach short-circuit** (Challenger C6): the initial SELECT
   runs every pass regardless of whether there's anything to demote.
   When BL-010 daemon runs the pass daily, this linear scan is a real
   cost on large corpora.

## Why (defer now)

Both findings are P2, triggered only at scale. Current corpus is 323
memories (~41 long-term), and the daily Dreaming pass is operator-invoked
(sub-second total time). The short-circuit and in-memory _after would save
~5 ms today — not worth the code churn.

## How to apply

When the trigger fires (below):

1. Replace the post-UPDATE SELECT with an in-memory filter:
   ```rust
   let breached_set: HashSet<&String> = breached_ids.iter().collect();
   let sum_after: f64 = longterm_rows
       .iter()
       .filter(|(id, _, _)| !breached_set.contains(id))
       .map(|(_, avg, last)| {
           parse_last_recalled(last)
               .map(|dt| decay::effective_relevance(*avg, dt, now))
               .unwrap_or(0.0)
       })
       .sum();
   let count_after = longterm_rows.len() - breached_ids.len();
   ```
2. Already have zero-breach short-circuit at line 252 — extend it
   to skip the initial SELECT entirely when `is_longterm = 1` count is 0.

## Trigger

First of:
- Corpus exceeds **50k long-term memories** (performance-reviewer's
  specific threshold; at that scale the second SELECT crosses 50 ms
  per pass, and a daily daemon amplifies).
- `mengdie dream` p95 runtime exceeds **1 second** in operator reports.
- BL-010 daemon lands and profiles the pass (the daemon context makes
  even small overheads worth trimming when scaled to continuous runs).

## Non-goals

NOT pursuing this now. The current implementation's clarity is worth
more than the premature optimization at current scale. This backlog
item exists so the trigger is recorded for the day it matters.
