---
id: BL-019
title: "CLI embed-fail produces no audit signal — operator visibility gap"
status: open
created: 2026-04-29
origin: F-002 /ae:review (challenger Challenge 3) + Codex Track 4 Wave 2 asymmetry
trigger: "Operator runs `mengdie search` repeatedly during an embedding outage (broken model download, ONNX runtime unavailable), hits embed-fail short-circuit, and notices later that the audit table has no record of those failed search attempts and `audit_write_failures` counter is unchanged. Earliest signal: a deployment incident where embedding model is broken for hours and the operator has zero durable evidence in mengdie's data."
---

# BL-019 — Surface CLI embed-fail in audit/observability

## What

`src/bin/cli.rs::cmd_search` propagates `embedder.embed_text(query)?`
errors via `?` (line 613). On embedding failure the function returns
`Err` BEFORE the audit hook below (line 626). No audit row is written;
the `audit_write_failures` counter does NOT increment (the wrapper is
never called).

This is asymmetric vs `mcp_tools.rs::search`, which on embed-fail
falls through to the FTS-only fallback path (lines ~225-249) — that
path returns a result list, so the audit hook DOES fire, and the
caller sees a `degraded: "embedding unavailable, FTS-only"` response
plus an audit row.

## Why it matters

Plan F-002 documents the asymmetry as deliberate ("CLI has no FTS
fallback; embedding errors propagate; no result list to audit"). At
plan time this was framed as acceptable: A-MEM trigger correctness is
unaffected (no results = no supersession data).

But there's a separate operator-visibility concern that the plan does
NOT address: an operator running `mengdie search` during a multi-day
embedding-model outage will hit dozens of CLI embed failures, and
none of them produce a durable signal. Stderr `tracing::warn!` from
`embed_text` exists but is ephemeral (process-restart loses it).
`mengdie stats` shows the same audit-counter values it did before the
outage. Operator post-incident triage cannot distinguish "CLI was
broken for 3 days" from "CLI was working but operator didn't search."

## Why this isn't already covered

- BL-014 (`mengdie audit-stats` CLI) is a deferred read-path command;
  it surfaces what's IN the audit table, but if no audit row was
  written, BL-014's report shows zero. Same blind spot.
- BL-017 (`mengdie stats` should surface `audit_write_failures`)
  surfaces the wrapper-failure counter, but the wrapper is NEVER
  called on CLI embed-fail — the counter stays at zero regardless.
- BL-013 (orphan-link cleanup) is unrelated.

## Trigger

File the implementing plan when ANY of:

1. An embedding-outage incident occurs and post-incident triage
   reveals no durable mengdie signal of the CLI search activity
   during the outage.
2. BL-014 (`mengdie audit-stats`) ships and operators discover that
   "CLI search failures" is not a queryable surface.
3. The Wave 2 BL-009/BL-010 free-function refactor is being designed,
   and the refactor author needs to decide whether
   `search::memory_search_audited` preserves the asymmetry or unifies
   the two surfaces. (See "Codex Wave 2 cross-reference" below.)

## Implementation sketches (when triggered)

Several options, ordered by scope:

1. **Minimal**: add a new metric counter `METRIC_CLI_EMBED_FAILURES`,
   bump it from `cli::cmd_search`'s embed-fail branch BEFORE the `?`
   propagation. ~5 lines. Surfaces durable failure count without
   requiring an audit row (which has no `fact_id` set to record).
2. **Audit-row-with-empty-results**: write an audit row with
   `returned_fact_ids = []` on CLI embed-fail. The audit row's
   `took_ms` reflects time-to-failure; `searched_at` localizes the
   failure window. Larger blast radius (changes the contract of "audit
   row implies result list was built").
3. **Add FTS fallback to CLI**: align CLI with MCP. Larger scope;
   changes user-visible behavior on embed-outage; should be a separate
   feature plan, not a hardening plan.

Recommended for first implementation: option 1 (minimal counter).
Options 2 and 3 are design changes warranting their own plans.

## Codex Wave 2 cross-reference

Codex Track 4 in F-002 /ae:review separately flagged this asymmetry
as a Wave 2 hand-off precondition: BL-009/BL-010 (the
`search::memory_search_audited` refactor) must explicitly decide
whether to preserve MCP's FTS fallback path or unify to CLI's
embedding-only semantics. That's a refactor-time question; this
backlog item is the operator-visibility consequence regardless of the
refactor outcome. Both BL-009 and BL-010 should be annotated to
require an explicit decision on this asymmetry as part of their plan
preconditions.

## Why not now (F-002 scope)

F-002 plan documents the asymmetry as "acceptable per plan: there is
no result list to audit". That framing is correct in isolation but
misses the operator-visibility angle. Filing as a backlog item with
trigger lets the user decide whether this becomes a v0.0.1 patch
or rolls into the BL-014 / Wave 2 work.

## Reviewer note

Surfaced by F-002 /ae:review:
- challenger Track (Challenge 3): operator-visibility gap.
- Codex Track 4: Wave 2 hand-off semantic gap.

Both reviewers independently flagged the same asymmetry from
different angles; convergent disposition into BL-019 keeps the
operator-visibility concern as a separate, durable backlog item.
