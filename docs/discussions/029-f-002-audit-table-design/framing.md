---
id: "029"
stage: framing
created: 2026-04-28
round_0: approved
round_0_reviewers: [codex-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer, gemini-proxy]
round_0_notes: |
  Run 1 (2026-04-28): unanimous REVISE (5/5). Framing rewritten — 3 of 5 topics
  moved to "Decided pre-discussion (YAGNI)"; 2 active topics remain; constraints
  separated; record_recall anchor neutralized; reviewer-count noun dropped.

  Rerun-1 (2026-04-28): 2 APPROVED (doodlestein-strategic, minimal-change-engineer) +
  3 REVISE (codex-proxy tone/structure, doodlestein-adversarial transaction-coupled
  feasibility coupling note, gemini-proxy threshold-suggests-probabilistic wording).
  All 3 REVISE items addressed inline in this version (no third Round 0 run):
  1. Codex: tone neutralized in pre-decided section ("speculative-feature
     anti-pattern" → "outside v0.0.1 acceptance contract"); atomicity moved from
     Constraints to Topic 2 as an assumption the contract shapes must address.
  2. Adversarial: Topic 2 transaction-coupled bullet now explicitly notes
     feasibility depends on Topic 1 outcome (only available under hook in
     Db::memory_search).
  3. Gemini-proxy/gemma: Topic 2 research question rewritten as a neutral
     question; "suggesting probabilistic tolerance" anchoring removed.

  Marking approved post-inline-edit per auto-mode TL judgment (3-of-3 REVISE items
  were targeted inline edits, not structural rewrites; framing-review convergence
  curve 5/5 REVISE → 3/5 REVISE; further iteration would over-anchor on framing
  perfection itself). Run-1 audit at round-00/; rerun-1 audit at round-00-rerun-1/.
---

# Framing — F-002 audit table design (rewritten)

## Problem Statement

F-002 (BL-006) is the v0.0.1 P0 instrumentation that persists `memory_search`
invocations + their returned fact IDs, so the supersession-rate signal becomes
computable for the deferred A-MEM trigger (per discussion 028 Topic 4). The
schema shape (link table) is settled by 028 conclusion.

F-002's `/ae:analyze` confirmed the link-table direction is correct and surfaced
several plan-time decisions. Round 0 framing review (2026-04-28, 5/5 REVISE)
re-classified those into **3 pre-discussion decisions** (settled by YAGNI per
v0.0.1's narrow scope contract from 028) and **2 genuinely open binary
choices** that this discussion converges.

## Decided pre-discussion (YAGNI — no Sweep needed)

These three are pre-resolved as part of the rewritten framing because they fail
the v0.0.1 ship-discipline test ("smallest design that satisfies 028 Topic 4
contract"). Reasons captured here so the rationale is durable.

1. **No explicit FK ON DELETE clause on `audit_returned_facts.fact_id`.**
   `PRAGMA foreign_keys` is OFF project-wide (`db.rs:80-119` — verified by
   archaeologist). FK declarations are documentation-only at runtime today.
   Designing ON DELETE semantics for a hypothetical future PRAGMA flip is
   forward-speculation. Decision: declare the FK reference (matches existing
   `memory_synthesis_links` convention) without an explicit `ON DELETE` clause;
   default `NO ACTION` is harmless under PRAGMA OFF and aligns with project
   convention. If PRAGMA is ever flipped on, that is a separate BL with its
   own concrete trigger (must first audit `db.rs:636 rename_project` DELETE
   path).

2. **No `caller_kind` column at v0.0.1.** Archaeologist's call-site analysis
   confirmed zero internal callers of `Db::memory_search` (only
   `mcp_tools.rs:211` and `cli.rs:609` — both operator-initiated). Adding a
   column for a hypothetical future internal caller is YAGNI; adding it later
   is a cheap one-column ALTER TABLE migration with unambiguous backfill rule
   (all pre-existing rows are 'operator').

3. **No v0.0.1 read path.** F-002's contract from 028 is "audit collection
   begins so supersession-rate becomes computable when A-MEM trigger fires."
   The A-MEM trigger IS the read consumer. Building a `mengdie audit-stats`
   CLI subcommand "so the table isn't write-only" is outside the v0.0.1
   acceptance contract; v0.0.1 ships write-only. The supersession SQL is a
   contract (must run against the schema correctly per 028 acceptance) but
   has no v0.0.1 in-binary caller. Adding a read path later requires zero
   schema change. **Note** (codex-proxy rerun-1): a minimal validation query
   embedded as a test or schema-acceptance assertion is not the same as a
   user-facing CLI; the former is part of v0.0.1 acceptance, the latter is
   the deferred read path.

## Open topics (for Sweep)

Two genuinely binary choices remain. Topic 1 is prerequisite for Topic 2 — the
hook placement determines what surfaces the failure-mode contract operates on.

### 1. Audit hook placement and coverage scope

The audit-write hook needs the resolved `(query, project_id, took_ms,
returned_fact_ids)` tuple. Two candidate placements exist:

- **Inside `Db::memory_search` (`src/core/search.rs:152`)** — single semantic
  chokepoint at the canonical hybrid search function. CLI search at
  `cli.rs:609` is covered automatically. **Excludes** the FTS-only fallback
  path at `mcp_tools.rs:220-244` (which calls `Db::search_fts` directly,
  bypassing `Db::memory_search`).
- **Inside `mcp_tools.rs` after the `match query_embedding` block** — covers
  both hybrid and FTS-fallback paths. CLI search must be wired separately
  (e.g., shared `record_memory_search_audit(...)` writer used from both
  `mcp_tools.rs` and `cli.rs:609`).

Coverage tradeoff: the FTS-fallback path fires when embedding generation fails
(broken model download, ONNX runtime unavailable). Searches during such
windows are real operator retrieval activity. Excluding them means the
supersession-rate signal under-counts during embedding outages.

### 2. Audit-write failure mode contract

When the audit insert fails (disk full, WAL stall, transaction rollback under
concurrent connection use), three contract shapes are defensible:

- **Best-effort + warn**: catch error, `tracing::warn!`, search returns
  successfully. Supersession-rate degrades probabilistically (under-counts
  under failure, never wrong-direction).
- **Hard error**: audit failure propagates, search returns error to caller.
  Supersession-rate is exact-or-absent (complete or no data).
- **Transaction-coupled**: search + audit in one `BEGIN IMMEDIATE`
  transaction. Either both succeed or both fail. Same correctness profile as
  hard error; different latency profile. **Feasibility coupling** (per
  doodlestein-adversarial rerun-1): only available if the hook lands inside
  `Db::memory_search` (Topic 1 option A); under option B (`mcp_tools.rs`)
  the connection mutex is released before `mcp_tools.rs` can open a wrapping
  transaction.

**Open research question for Round 1**: does A-MEM's deferred trigger
algorithm require strict signal completeness (no partial windows), or
tolerate probabilistic loss (count threshold against potentially
under-counted data)? The 028 trigger condition is a volume metric ("≥5
events per 30-day window"), not a reliability requirement; algorithm-level
confirmation is required — Round 1 must research the trigger algorithm
itself, not infer the answer from threshold wording.

**Implicit assumption to validate** (per codex-proxy rerun-1): each
contract shape carries an implicit atomicity expectation. Best-effort
operates on the assumption that the audit-write may complete after the
search response is built; hard-error and transaction-coupled assume the
audit-write is part of the response-construction critical path. Round 1
should surface which atomicity model fits each shape rather than treating
"hook runs inside the same connection-lock as the search" as a given.

The `record_recall` pattern at `db.rs:259-272` is a precedent for best-effort
silent in mengdie's existing code, but its signal (UI recall counter) has
different criticality than F-002's signal (supersession-rate trigger
correctness). Treat the precedent as evidence about mengdie's existing
conventions, not as a prescription for F-002.

## Known constraints / assumptions to validate

These apply to BOTH topics. Items marked "constraint" are facts about the
existing code or v0.0.1 commitments. Items marked "assumption" are working
premises that Round 1 may revisit if evidence warrants.

- **Constraint**: mengdie's existing `Db::open` (`db.rs:80-119`) does NOT
  set `PRAGMA foreign_keys = ON`; F-002 inherits that convention.
- **Constraint**: the CLI search call site (`src/bin/cli.rs:609`) is
  operator-initiated and must be audited (CLI is part of the operator
  surface).
- **Constraint**: whatever the topic-1 placement decision, the resulting
  code must remain compatible with Wave 2 BL-009 + BL-010's search-path
  consolidation refactor. If F-002 lands first (Wave 1), the audit-write
  hook may live in a location BL-009/BL-010 will refactor; the hook's
  location may move during Wave 2 but the schema does not change.
- **Constraint**: whatever the topic-2 failure-mode decision, the audit
  table's contract must be observable — under best-effort, a
  `METRIC_AUDIT_WRITE_FAILURES` counter exists so silent drops have a
  tracked signal.
- **Assumption** (moved here from prior framing per codex-proxy rerun-1):
  the audit-write may share the connection-lock with the search statements
  for atomicity, OR may run as a separate write outside the search lock.
  Topic 2's contract shapes have different atomicity expectations and Topic
  1's placement decision constrains feasibility. Round 1 should treat this
  as a topic-coupled choice, not a framing-level given.

## Reference Material

- `.ae/features/active/F-002-persisted-domain-audit-table-audit-retur/analysis.md`
  — full F-002 analyze output (multi-reviewer synthesis, recommended schema
  sketch, 5 open decisions before Round 0 reduced them to 2).
- `docs/discussions/028-v0.0.1-architecture-design/conclusion.md` — settles
  link-table shape; defines Wave 1 BL A.
- `src/core/schema.rs` — current schema state (v5), migration pattern
  reference (v5 model migration at lines 214-236).
- `src/core/mcp_tools.rs:207-244` — search handler with FTS-fallback path.
- `src/core/search.rs:152` — `Db::memory_search` candidate hook location.
- `src/core/db.rs:259-272` — `record_recall` precedent for best-effort silent
  observability writes.
- `src/core/db.rs:636` — `rename_project` `DELETE FROM memory_entries` site
  (the only DELETE-from-memory_entries path in production code).
- `docs/discussions/029-f-002-audit-table-design/round-00/` — first-run
  framing-review verdict files (audit trail).
