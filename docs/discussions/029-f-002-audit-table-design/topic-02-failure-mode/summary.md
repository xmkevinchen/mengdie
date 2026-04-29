---
id: "02"
title: "Audit-write failure mode contract"
status: converged
current_round: 2
created: 2026-04-28
decision: "Best-effort + tracing::warn! + METRIC_AUDIT_WRITE_FAILURES counter. Audit failures do NOT propagate to the search caller."
rationale: "UAG-PASS 5/5. False positive structurally impossible (best-effort only subtracts rows). False negatives are bounded delays observable via metric counter. A-MEM trigger is volume metric (>=5/30d) — monotonic-lower under-counting cannot wrong-direction the trigger. Matches record_recall precedent at db.rs:259-272 + caller-side warn pattern at search.rs:188-190."
reversibility: "medium"
reversibility_basis: "Failure-mode contract change (best-effort -> hard-error) is a behavior change to MCP callers. Reversibility cost is medium not high — would require coordinating with MCP callers for the new error semantics."
---

# Topic: Audit-write failure mode contract

## Current Status

Open. Depends on Topic 1's placement decision (failure semantics may differ
per call site if hook covers multiple paths).

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context

When the audit insert fails (disk full, WAL stall, FK violation under future
PRAGMA flip, transaction rollback under concurrent use), three contract shapes
are defensible:

- **Best-effort + warn**: error caught, `tracing::warn!`, search returns
  successfully. Supersession-rate degrades probabilistically (under-counts
  under failure, never wrong-direction).
- **Hard error**: audit failure propagates, search returns error to caller.
  Supersession-rate is exact-or-absent (complete or no data).
- **Transaction-coupled**: search + audit in one `BEGIN IMMEDIATE`
  transaction. Either both succeed or both fail. Correctness profile matches
  hard error; latency profile differs. **Feasibility coupling**: only
  available under Topic 1 option A (hook inside `Db::memory_search`); under
  option B the connection mutex is released before `mcp_tools.rs` can open
  a wrapping transaction.

The `record_recall` pattern at `db.rs:259-272` is mengdie's existing
best-effort precedent. Its signal (UI recall counter) has different
criticality than F-002's signal (supersession-rate trigger correctness).
Treat as evidence about mengdie's conventions, not as a prescription.

## Constraints

- A-MEM's deferred trigger requirement (strict completeness vs probabilistic
  tolerance) is **the open research question** that determines which contract
  shape is correct. Not a settled precondition.
- Under best-effort, a `METRIC_AUDIT_WRITE_FAILURES` counter is required so
  silent drops are observable.
- Under hard error or transaction-coupled, MCP callers (AI agents) need to
  distinguish "search failed because retrieval failed" from "search succeeded
  but audit-write failed" — these may need different error codes.
- Under transaction-coupled, latency overhead of `BEGIN IMMEDIATE` wrapping
  the entire search + audit must be measurable at v0.0.1 corpus size; if
  >>10ms it crosses an MCP responsiveness boundary.
- Topic 1's placement decision may force per-site failure-mode (e.g.,
  FTS-fallback might be best-effort while hybrid path is hard-error). This
  is a possible outcome, not necessarily a contradiction.

## Key Questions

- **Research question (load-bearing)**: does A-MEM's deferred trigger
  algorithm require strict signal completeness (no partial windows), or
  tolerate probabilistic loss (count threshold against potentially
  under-counted data)? The 028 trigger condition ("≥5 events per 30-day
  window") is a volume metric, not a reliability requirement; algorithm-
  level confirmation is required.
- Given Topic 1's placement decision, can a single failure-mode contract
  apply uniformly across all hook sites, or is per-site policy required?
- Under best-effort: is `tracing::warn!` + `METRIC_AUDIT_WRITE_FAILURES`
  sufficient observability, or does the gap need stronger signal (e.g., a
  separate `audit_write_failures` table recording reason + timestamp)?
- Under hard error: does the MCP-side failure manifest as an error response
  to the AI agent, or as a partial response with an embedded audit-write
  warning? What's the actual MCP-level error contract?
- Under transaction-coupled: what is the measured latency overhead at v0.0.1
  corpus size, and is it acceptable in the MCP responsiveness budget?
