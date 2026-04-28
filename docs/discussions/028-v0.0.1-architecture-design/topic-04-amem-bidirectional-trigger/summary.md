---
id: "04"
title: "A-MEM bidirectional update deferral trigger"
status: converged
current_round: 2
created: 2026-04-27
decision: "A-MEM bidirectional update deferred from v0.0.1. Trigger fires when ALL hold: (1) corpus ≥1,000 facts (operator may calibrate to 500 — within operator discretion), (2) ≥5 superseded-within-7-days events per rolling 30-day window from the persisted domain audit table. v0.0.1 instrumentation MUST log returned_fact_ids per memory_search call to enable the supersession-rate computation."
rationale: "5-of-5 agreed A-MEM defers from v0.0.1 (carried from analyze). 5-of-5 converged on 'corpus floor + audit-log supersession signal' shape in Round 2. NO MCP ACK in v0.0.1 contract (challenger's meta-decision); forces server-side observable trigger. arch-reviewer + others flagged returned_fact_ids logging requirement — falls into v0.0.1 instrumentation BL scope."
reversibility: high
reversibility_basis: "A-MEM is deferred work, not committed v0.0.1 work. Trigger fires automatically; BL opens for design+implementation when conditions hold. No reversal cost in deferral."
mcp_ack_meta_decision: "NO ACK feedback in v0.0.1 memory_search MCP contract. challenger's argument: 'used' signal ambiguous (AI exclusion-discard counts as used); contractual burden > value of noisy precision estimate."
instrumentation_dependency: "Persisted domain audit table must log returned_fact_ids per memory_search call. This is a v0.0.1 P0 instrumentation requirement, derived from Topic 4 trigger needs."
---

# Topic: A-MEM bidirectional update deferral trigger

## Current Status
**Converged** (Round 2). Deferred from v0.0.1 with audit-table-derived trigger. Specific numbers within operator calibration discretion.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | exploratory | 5 different trigger proposals; ACK protocol question raised |
| 2 | converged | Shape converged on corpus + audit-log signal; specifics for operator calibration; NO MCP ACK; returned_fact_ids logging required |

## Context

A-MEM (Xu et al., NeurIPS 2025, arxiv:2502.12110) proposes
bidirectional update on memory ingest: when a new fact F arrives,
re-evaluate related facts in F's entity cluster (not just append F).

In analyze phase, three positions emerged:
- architecture-reviewer: synchronous bidirectional update in ingest
  path for v0.0.1 (cluster sizes small at current scale, 10–50ms
  cost is acceptable)
- codex-proxy: split — sync (durable parts: fact + provenance +
  embedding) + async (cluster re-evaluation) — memory writes
  blocking user workflows is wrong shape
- challenger: defer entirely; one paper, no independent replication
  cited

4-of-4 converged on: defer A-MEM bidirectional update from v0.0.1.
But the deferral trigger needs to be concrete enough to fire
automatically without further design discussion.

Possible trigger conditions raised in analyze phase:
- Corpus size threshold (e.g., > 1k facts)
- Retrieval quality measurement showing degradation
- Independent replication of A-MEM published in 2026+

## Constraints

- v0.0.1 minimum (per challenger update + reviewer agreement) does
  NOT include A-MEM. This is settled.
- The deferral trigger must be measurable using instrumentation
  that v0.0.1 will have (per blueprint §5 P0: persisted domain audit
  + tracing events).
- Blueprint §11: BLs without traceable trigger conditions are scope
  creep.

## Key Questions

- What is the concrete, measurable trigger condition that fires
  "implement A-MEM bidirectional update"?
- Which signal indicates retrieval quality has degraded enough to
  need bidirectional update?
- Should the trigger be size-based (corpus growth), quality-based
  (measured retrieval degradation), external (paper replication), or
  composite?
