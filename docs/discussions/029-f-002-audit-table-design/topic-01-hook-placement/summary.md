---
id: "01"
title: "Audit hook placement and coverage scope"
status: converged
current_round: 2
created: 2026-04-28
decision: "Hook invoked at mcp_tools.rs after the `match query_embedding` block (Option B) via Db::record_search_audit(...) method on impl Db. CLI search at cli.rs:609 calls the same Db method directly."
rationale: "UAG-PASS 5/5 across ~20 counterexample attempts. Option A excludes FTS-fallback (deflates supersession signal) and produces incomplete took_ms. Option B preserves Db as pure storage primitive; CLI wiring cost is one method call. Db-level helper matches record_recall + metrics.rs precedents."
reversibility: "high"
reversibility_basis: "Pure code-move under Wave 2 BL-009/BL-010 (call site moves to search::memory_search_audited free fn per 028 Topic 1). Schema unchanged across the migration. No data migration cost."
---

# Topic: Audit hook placement and coverage scope

## Current Status

Open. Sequencing prerequisite for Topic 2.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context

The audit-write hook needs `(query, project_id, took_ms, returned_fact_ids)`.
Two candidate placements:

- **Inside `Db::memory_search` (`src/core/search.rs:152`)**: single semantic
  chokepoint at canonical hybrid search; CLI auto-covered; excludes
  FTS-fallback path (`mcp_tools.rs:220-244`) where `Db::search_fts` is called
  directly.
- **Inside `mcp_tools.rs` after the `match query_embedding` block**: covers
  both hybrid + FTS-fallback paths; CLI requires separate wiring (shared
  writer function).

The choice determines whether degraded-mode searches (when embedding generation
fails) are part of the audit corpus, which determines whether the
supersession-rate signal under-counts during embedding outages.

## Constraints

- Hook must run inside the same `Arc<Mutex<Connection>>` lock as the search
  statements (atomicity with the read).
- CLI search (`cli.rs:609`) is operator-initiated and must be audited.
- Whatever placement is chosen, the result must remain compatible with Wave 2
  BL-009 + BL-010's search-path consolidation refactor (location may move,
  schema does not).
- Coverage gap or coverage breadth is the load-bearing tradeoff; do not pick
  on aesthetic grounds (e.g., "single chokepoint feels cleaner") without
  surfacing the supersession-signal correctness implication.

## Key Questions

- Does the supersession-rate signal correctness contract require auditing
  degraded-mode (FTS-fallback) searches, or is "embedding-broken windows
  produce no audit data" an acceptable gap given that A-MEM's trigger is a
  count threshold (probabilistic-tolerant)?
- Where in the call graph does a single hook location cover all 3 call sites
  (MCP hybrid, MCP FTS-fallback, CLI search)? If no single location covers
  all 3, what is the structural shape (shared writer function called from
  multiple sites)?
- Is the placement decision compatible with how BL-009 + BL-010 will
  restructure the search path in Wave 2 — i.e., does the current placement
  add migration cost when Wave 2 lands?
