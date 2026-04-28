---
round: 02
date: 2026-04-28
score: converged
---

# Round 02 — Topic 4

## Discussion

**Position evolution** — significant convergence on shape, divergent specifics:

| Agent | Round 1 | Round 2 |
|---|---|---|
| architecture-reviewer | top-3 score 30d-rolling avg < 0.35 + corpus > 500 + avg cluster > 5 | UPDATED: dropped score-threshold; adopted minimal-change's stale-delivery count + corpus ≥500 + avg cluster > 5 |
| minimal-change-engineer | corpus ≥1k + ≥5 stale-retrieval + 1 paper replication | UPDATED: dropped paper replication; corpus ≥1k + ≥5 supersession-within-7-days/30d window |
| challenger | precision-based if MCP-ACK; corpus-only fallback | UPDATED: stale-retrieval count (≥5 superseded-within-14-days/30d) + corpus ≥500; corpus number flagged for operator calibration |
| codex-proxy | 4-AND with eval-dependent | UPDATED: corpus ≥1k + (5+ stale-fact OR 15% zero-fact) + failure pattern = updates/contradictions |
| gemini-proxy | corpus > 5k + retrieval quality degrading (unspecified) | UPDATED: corpus ≥1k + ≥5 supersession-within-7-days/30d window |

**Meta-decision integrated**: NO ACK feedback in v0.0.1 MCP
`memory_search` contract (challenger's argument: "used" signal
ambiguous, contractual burden too high). Forces all triggers to be
server-side observable.

**Convergent shape**: corpus floor + audit-log supersession signal.
**Divergent specifics**:
- Corpus floor: 500 (arch-reviewer, challenger) vs 1k
  (minimal-change, codex, gemini)
- Supersession window: 7 days (minimal-change, gemini) vs 14 days
  (challenger)
- Additional precondition: avg cluster > 5 (arch-reviewer); 15%
  zero-fact searches alternative (codex)

## Outcome

- **Score**: converged (shape); specifics flagged for operator
  calibration when filing the A-MEM BL
- **Decision**:
  1. **A-MEM bidirectional update is deferred from v0.0.1.**
  2. **Trigger condition** (concrete, measurable from the v0.0.1
     persisted domain audit table; no MCP ACK required):
     - Corpus floor: **≥1,000 facts** (3-of-5 majority; 500
       acceptable as alternative — operator calibrates)
     - Supersession signal: **≥5 superseded-within-7-days events
       per rolling 30-day window**, where supersession is detected
       via existing schema's `valid_until` + `superseded_by` fields
       set on returned facts after a `memory_search` call (the
       returned fact is later superseded)
     - One of these must hold simultaneously
  3. **Integration constraint** (v0.0.1 instrumentation
     requirement): the persisted domain audit table MUST log
     `returned_fact_ids` per `memory_search` call so the
     supersession signal can be computed via join (`audit.returned_fact_ids`
     → `memory_entries.valid_until`/`superseded_by`).
- **Rationale**:
  1. 5-of-5 agreed A-MEM defers from v0.0.1 (carried from analyze
     phase).
  2. 5-of-5 converged on "corpus floor + audit-log supersession
     signal" trigger shape after Round 2 conflict resolution.
  3. challenger's MCP ACK protocol decision (NO) was integrated;
     all triggers are server-side observable.
  4. Specific numeric thresholds (corpus 500 vs 1k, window 7d vs
     14d) are within operator's calibration discretion when filing
     the BL — these are scale-tuning parameters, not architectural
     decisions.
  5. The instrumentation constraint (`returned_fact_ids` logged per
     search) is a v0.0.1 P0 instrumentation requirement that falls
     out naturally from this trigger; arch-reviewer + others flagged
     it.
- **Reversibility**: high
- **Reversibility basis**: A-MEM is deferred work, not committed
  v0.0.1 work. The trigger condition is observed via existing
  v0.0.1 instrumentation; the trigger fires automatically when
  conditions hold; the BL is then opened for design + implementation.
  No reversal cost in the deferral itself.

## Open question for v0.0.1 instrumentation BL

The integration constraint above (`returned_fact_ids` per search
call) becomes a **v0.0.1 instrumentation BL** subordinate to the AE
Round-0 wiring + audit table design. This is not a new BL outside
the existing v0.0.1 minimum sprint — it's a design requirement on
the audit table schema.
