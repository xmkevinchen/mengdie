---
id: "008"
title: "Contradiction Detection — Conclusion"
concluded: 2026-04-05
plan: ""
---

# Contradiction Detection — Conclusion

4 agents across 3 rounds + UAG. Topic: should mengdie add a `memory_resolve_conflict` MCP tool?

---

## Decision Summary

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Add `memory_resolve_conflict` MCP tool? | **No** | Resolution is mechanically complete via existing two-call pattern; gaps are in descriptions, output completeness, and a dropped field — not in tool surface | high — description/schema changes only |

---

## What Was Found

The conflict resolution workflow is already present:

1. `memory_ingest` returns `{entry_id, conflicts: [{id, title, reason}]}`
2. Agent calls `memory_invalidate(entry_id=conflict.id, superseded_by=new_entry_id)`

All schema fields exist: `valid_until`, `superseded_by`. The DB layer sets both correctly. No new primitive is needed.

Three gaps were identified:

| Gap | Type | Fix |
|-----|------|-----|
| `reason` param in `InvalidateParams` is silently dropped — no DB column | Bug | Add `invalidation_reason TEXT` column; persist on write |
| `superseded_by` is write-only — never returned in `InvalidateOutput` | Incomplete output | Add `superseded_by: Option<String>` to `InvalidateOutput` |
| No agent guidance on what to do with conflicts returned from ingest | Description gap | Rewrite tool descriptions with explicit resolution workflow |

---

## Four Concrete Fixes

**Fix 1 — Persist invalidation reason**

Add `invalidation_reason TEXT` column to `memory_entries`. Write `params.reason` in `invalidate_memory()`.

Scope: `src/core/schema.rs` (migration), `src/core/db.rs` (`invalidate_memory`).

**Fix 2 — Return `superseded_by` in invalidate output**

Add `superseded_by: Option<String>` to `InvalidateOutput`. Populate from the value written to DB.

Scope: `src/core/mcp_tools.rs` (`InvalidateOutput` struct + `invalidate` handler).

**Fix 3 — Tool description rewrite**

`memory_ingest`:
> "Ingest a new memory. Returns entry_id and any detected conflicts. For each conflict: if reason contains 'evolution candidate', call memory_invalidate with entry_id=conflict.id, reason='superseded', superseded_by=this entry_id to resolve. For 'recent conflict', surface to user before resolving."

`memory_invalidate`:
> "Mark a memory as no longer valid. Set superseded_by when a newer memory replaces it — links records for traceability. The reason field is persisted for audit."

Scope: `src/core/mcp_tools.rs` (`#[tool(...)]` description strings).

**Fix 4 — Atomic ingest+resolve at DB layer**

Add `resolves: Option<Vec<String>>` to `IngestParams`. When present, DB layer wraps INSERT (new memory) + UPDATE (set `valid_until` on each resolved ID) in a single SQLite transaction.

Scope: `src/core/mcp_tools.rs` (`IngestParams`), `src/core/db.rs` (new `insert_memory_resolving` method), `src/core/mcp_tools.rs` (ingest handler).

This closes the atomicity gap without a new MCP tool. Agent calls one tool; DB does the atomic write.

---

## What Was Rejected

**`memory_resolve_conflict(conflict_id, strategy, rationale)` tool**

- Strategy enum (`keep_newest`, `keep_oldest`, `keep_both_temporal`) encodes policy in the tool layer — agents can't express strategies not pre-enumerated. Policy belongs in agent reasoning, not tool API.
- "Discoverability" argument: Fix 3 (description rewrite) solves this directly. Adding a tool to compensate for unclear descriptions is the wrong fix.
- Precedent from Engram: different architecture, different constraints — not applicable here.
- Redundant: the tool would map to the same DB write as `memory_invalidate` with `superseded_by`.

Tool surface stays at 3.

---

## Backlog

See `docs/backlog/005-atomicity-multiwrite.md` (to be created).

**Trigger**: When mengdie moves from stdio single-client to multi-client (HTTP/SSE daemon, Phase 2). At that point, Fix 4's single-transaction `insert_memory_resolving` becomes a hard requirement rather than a convenience. The two-call pattern is unsafe under concurrent writers.

At MVP scale (10-50 memories, single stdio client), dirty state from process death between calls is: low-probability, self-correcting (conflict fires again on next ingest), and not silent. Acceptable.

---

## Process Metadata

- Discussion rounds: 3 + UAG (Unanimous Agreement Gate)
- Agents: architect (Claude), code-researcher (Claude), Codex-proxy
- Topics: 1 (converged)
- UAG challenges: 2 (no counterexample; atomicity gap found — resolved via Fix 4)
- Final position: unanimous (Codex conceded strategy enum over-engineering; atomicity addressed by DB-layer fix)
