---
id: "029"
title: "F-002 audit table design — Conclusion"
concluded: 2026-04-28
plan: ""
entities: [audit, hook, placement, audit-hook-placement, mcp-tools-hook, db-record-search-audit, failure, mode, audit-failure-mode, best-effort-warn, metric-audit-write-failures, fk, on-delete, no-fk-clause, caller, kind, no-caller-kind, read, path, no-v0-0-1-read-path, indexes, three-index-design, supersession, rename-project, pragma-foreign-keys-off, wave-2, search-memory-search-audited]
---

# F-002 audit table design — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|---|---|---|---|
| 1 | Audit hook placement | **Hook invoked at `mcp_tools.rs` after the `match query_embedding` block** (Option B) via new `Db::record_search_audit(...)` method on `impl Db`. CLI search at `cli.rs:609` calls the same Db method directly (no cross-module dependency). | UAG-PASS 5/5 across ~20 distinct counterexample attempts. Option A excludes the FTS-only fallback path (`mcp_tools.rs:220-244`) — searches during embedding outages are unaudited under Option A, deflating the supersession-rate signal. Option A also produces incomplete `took_ms` (embedding inference time happens at `mcp_tools.rs:197-202` BEFORE `Db::memory_search`). Option B keeps `Db` as pure storage primitive (clean architecture); CLI wiring cost is one method call. The Db-level helper matches `record_recall` (db.rs:259) and `metrics.rs` precedents. | **High** — pure code-move under Wave 2 BL-009 + BL-010; the call site moves to `search::memory_search_audited` free fn per 028 Topic 1 decision; schema unchanged. |
| 2 | Audit-write failure mode | **Best-effort + `tracing::warn!` + `METRIC_AUDIT_WRITE_FAILURES` counter.** Audit failures do NOT propagate to the search caller. | UAG-PASS 5/5. False positive is structurally impossible (best-effort only subtracts rows, never adds phantom ones). False negatives are at most bounded delays observable via the metric counter. A-MEM's deferred trigger is a volume metric (≥5/30d window) — probabilistic-tolerance argument: monotonic-lower under-counting cannot cause wrong-direction trigger outcome. Matches existing `record_recall` precedent at `db.rs:259-272` (caller-side `tracing::warn!` pattern at `search.rs:188-190`). Hard-error rejected: degrades search UX for infrastructure failures the operator can't recover from. Transaction-coupled rejected: requires major restructure of `Db::memory_search` to hold one mutex guard end-to-end (12 lock acquire/release cycles per call today), AND adds no wrong-direction-prevention value over best-effort. | **Medium** — failure-mode contract change (best-effort → hard-error) is a behavior change to MCP callers; reversibility cost is medium not high. |

## Pre-discussion decisions (YAGNI — settled at Round 0 framing rewrite, not re-litigated)

These three were re-classified from "open topic" to "pre-decided" during Round 0 (rerun-1 after unanimous REVISE). No agent re-opened them in Round 1 or Round 2:

- **No explicit FK `ON DELETE` clause** on `audit_returned_facts.fact_id`. PRAGMA foreign_keys is OFF project-wide (`db.rs:80-119`); FK declarations are documentation-only at runtime. Default `NO ACTION` is harmless under PRAGMA OFF and aligns with existing `memory_synthesis_links` convention. If PRAGMA enforcement is ever enabled (separate BL with its own trigger), the `db.rs:636 rename_project` DELETE path must be audited first. Database-optimizer Round 2 verified: under PRAGMA OFF, rename_project leaves silent orphan link rows; supersession query inner-join naturally excludes orphans, so the audit signal is unaffected.
- **No `caller_kind` column** at v0.0.1. Archaeologist confirmed zero internal callers of `Db::memory_search` exist today (`mcp_tools.rs:211` and `cli.rs:609` are both operator-initiated). Adding the column later is a cheap one-column ALTER TABLE migration with unambiguous backfill rule (all pre-existing rows are 'operator').
- **No v0.0.1 read path** (no `mengdie audit-stats` CLI subcommand). The A-MEM trigger IS the read consumer; building a CLI subcommand "so the table isn't write-only" is outside v0.0.1 acceptance contract. The supersession SQL has v0.0.1 acceptance (must run against the schema correctly per 028) but no in-binary caller. A minimal validation query embedded as a test or schema-acceptance assertion is not the same as user-facing CLI; the former is part of v0.0.1 acceptance, the latter is the deferred read path. Adding a read path later requires zero schema change.

## Convergent ratifications (decided in Round 2)

These follow from the two top-level decisions and were converged in Round 2:

| # | Item | Decision |
|---|---|---|
| R1 | Helper location | `Db::record_search_audit(audit_id?, query, scope, took_ms, returned_fact_ids)` method on `impl Db`. |
| R2 | Helper call sites | `mcp_tools.rs` after the search-result match block (covers hybrid + FTS-fallback) + `cli.rs:609` (CLI operator search). |
| R3 | Sync vs async | Sync. Audit cost measured at ~100-400µs (db-optimizer EXPLAIN+timing); embedding inference dominates total search latency at 2-10ms. tokio::spawn complexity not justified. Matches `record_recall` sync precedent. |
| R4 | Index design | Codex's three-index design ratified by EXPLAIN QUERY PLAN at v0.0.1 corpus (1000 facts, 300 audit rows, 3000 link rows, ANALYZE'd, SQLite 3.51.0): `idx_memory_search_audit_searched_id ON memory_search_audit(searched_at, id)`, `idx_audit_returned_facts_fact_audit ON audit_returned_facts(fact_id, audit_id)` (reverse-FK covering), `idx_memory_entries_valid_until_id ON memory_entries(valid_until, id) WHERE valid_until IS NOT NULL` (partial). Measured: 1ms wall time, 104µs CPU; partial index drives the join with 5% selectivity; 2-of-3 used as COVERING. |
| R5 | Wave 2 migration | Call site moves into `search::memory_search_audited` free function (per 028 Topic 1 free-functions decision shipped via BL-009 + BL-010). Db-level helper unchanged. Schema unchanged. Pure code-move. |
| R6 | `rename_project` FK coupling | Safe at v0.0.1 (PRAGMA OFF). Orphan link rows silently created on collision merges; supersession query's inner join excludes them naturally. PRAGMA enforcement enable is a separate BL trigger. |
| R7 | Schema (locked from Round 0 + 028) | `memory_search_audit (id INTEGER PK, query TEXT NOT NULL, scope TEXT, took_ms INTEGER NOT NULL, searched_at TEXT NOT NULL)` + `audit_returned_facts (audit_id INTEGER NOT NULL, fact_id TEXT NOT NULL, rank INTEGER NOT NULL, PRIMARY KEY (audit_id, fact_id))` + 3 indexes per R4. v6 migration follows v5's hand-rolled `BEGIN TRANSACTION → schema → PRAGMA user_version=6 → COMMIT` pattern. |

## Doodlestein Review

Three post-conclusion reviewers (`strategic`, `adversarial`, `regret`)
audited the written conclusion. All findings are valid; none invalidate
the converged decisions; all are TL-absorbable as plan-time annotations
or backlog items. No new round fired. Verdicts and dispositions:

| Reviewer | Finding | Severity | Disposition |
|---|---|---|---|
| `strategic-post` | Next Steps conflates "supersession SQL is the v0.0.1 acceptance test" with "supersession SQL has no in-binary caller," creating a silent gap where the plan author could ship the schema without any assertion that the supersession query is correct. | P2 | **Plan-time TODO** — F-002 plan must include a Rust integration test that seeds `memory_search_audit` + `audit_returned_facts` + `memory_entries` with a known supersession scenario and asserts the supersession SQL from F-002 `analysis.md` returns the expected rows. (Distinct from the deferred CLI read path: this is a schema correctness gate, not a user-facing command.) |
| `adversarial-post` Finding 1 | `METRIC_AUDIT_WRITE_FAILURES` is ephemeral — process restart silently zeroes the counter. The "bounded delays observable via the metric counter" claim only holds within a single process lifetime. | P2 | **Plan-time TODO** — F-002 plan must specify that the `tracing::warn!` log line on audit-write failure includes the audit query text + timestamp, so audit gaps are recoverable from stderr logs even after process restart. One-line addition; no schema change. |
| `adversarial-post` Finding 2 | Orphan link-row accumulation from `rename_project` collision merges may shift SQLite query planner away from the R4 covering index toward a table scan after repeated rename events, breaking supersession query latency 6-12 months out. | P3 | **New backlog item** [BL-013](../../backlog/unscheduled/BL-013-audit-orphan-link-row-cleanup-rename-project.md) — orphan link-row cleanup with concrete trigger condition (orphan ratio > 10:1). Cheap indexed cleanup query; no schema change. |
| `adversarial-post` Finding 3 | CI validation query covers seeded test DB, but the operator's production DB is never queried by any v0.0.1 tooling. If the audit hook is silently broken, the operator cannot distinguish "audit working, A-MEM not yet triggered" from "audit broken silently." | P2 | **New backlog item** [BL-014](../../backlog/unscheduled/BL-014-mengdie-audit-stats-doctor-command.md) — `mengdie audit-stats` / `mengdie doctor` subcommand for v0.0.1.x patch window (before A-MEM lands). Closes operator-debug discoverability gap; does NOT expand v0.0.1's commitment. |
| `regret-post` | Most-likely-reversed decision: Topic 2 best-effort. Trigger conditions: (a) A-MEM algorithm paper-level confirmation reveals ratio-based trigger (vs floor-based); (b) sustained multi-day audit write failures with ephemeral counter loss; (c) Wave 2 introduces retry/buffering layer. P2 reversibility (medium cost, behavior contract change). | P2 | **Plan-time TODO + future-A-MEM-plan precondition** — F-002 plan should annotate that the "no wrong-direction trigger" argument holds for floor-threshold A-MEM design, NOT for ratio-based design. When A-MEM's algorithm is specified (before the A-MEM feature plan is filed), one acceptance criterion must state whether the trigger is a floor or a ratio. If ratio, revisit Topic 2. No schema or code change needed now. |
| `regret-post` secondary | "no caller_kind column": cheap reversal confirmed; if dreaming-time auto-search lands later, backfill of pre-existing rows to 'operator' will mislabel those rows. Labeling accuracy issue, not wrong-result issue. | P3 | **Note for caller_kind BL when filed** — backfill rule remains "all pre-existing rows = 'operator'" but the resulting label is best-effort accurate, not retroactively perfect. Acceptable; documented. |
| `regret-post` tertiary | "no v0.0.1 read path": low reversal risk; operator can `sqlite3` as stopgap. | P3 | **Disposition stands** — adversarial-post Finding 3 / BL-014 covers the operator-discoverability path with concrete trigger; regret confirms the pre-decision is sound at v0.0.1 scope. |
| `regret-post` quaternary | `Db::record_search_audit` on `impl Db`: low reversal risk; Wave 2 free-function migration creates the abstraction point where buffering could be introduced without touching `impl Db`. | P3 | **Disposition stands** — Wave 2 R5 already addresses the abstraction-point concern. |

None of the findings challenged a converged decision. Each was a plan-time
specification refinement (4 plan-time TODOs) or a backlog item with concrete
trigger (2 new BLs filed: BL-013, BL-014). Dissents in the original Decision
Summary (Topic 1 inversion by gemma in Round 1) were corrected in Round 2.

## Plan-time TODOs (for `/ae:plan` to absorb into F-002 plan acceptance criteria)

These are absorbed from Doodlestein post-conclusion review and should appear
explicitly in the F-002 plan's acceptance criteria or Open Questions:

1. **Acceptance test for supersession SQL** (strategic-post): Rust integration test that seeds the schema with a known supersession scenario and asserts the supersession SQL returns the expected rows. Schema correctness gate, not user-facing CLI.
2. **`tracing::warn!` log content** (adversarial-post F1): the warn line on audit-write failure must include the audit query text + timestamp for post-restart audit-gap recovery from stderr logs.
3. **A-MEM floor-vs-ratio precondition** (regret-post): the F-002 plan annotates that the "no wrong-direction trigger" Topic 2 rationale holds for a floor-threshold trigger, not a ratio. When the A-MEM feature plan is filed, the floor-vs-ratio question must be resolved as a plan precondition; if ratio, Topic 2 contract must be revisited.

## Spawned Discussions

None. All topics resolved within this discussion.

## Deferred Resolutions

None. Sweep was empty (zero deferred + zero revisit at Round 2 close; the 3 pre-discussion YAGNI decisions were settled at Round 0 framing-edit time and not re-litigated in Round 1 or Round 2).

## Team Composition

| Agent | Role | Backend | Joined |
|---|---|---|---|
| host | TL (moderator) | Claude (this session) | Start |
| archaeologist | codebase verification | Claude (`ae:research:archaeologist`) | Round 1 |
| database-optimizer | SQLite/rusqlite WAL/transaction/index analysis | Claude (`engineering-database-optimizer`) | Round 1 |
| architecture-reviewer | clean-architecture / module-boundary lens | Claude (`ae:review:architecture-reviewer`) | Round 1 |
| codex-proxy (slot) | OpenAI cross-family lens — slot non-responsive both rounds | oMLX `Qwen3-Coder-Next-4bit` (Alibaba lens) — TL fallback per CLAUDE.md cross-family strategy | Round 1 (TL fallback) |
| gemini-proxy (slot) | Google cross-family lens — Gemini quota exhausted | oMLX `gemma-4-26b-a4b-it-4bit` (Google-family fallback) — TL fallback per CLAUDE.md | Round 1 (TL fallback) |
| (Doodlestein × 5 across Round 0 and post-conclusion) | framing review (Round 0 + rerun-1) and post-conclusion review (Step 9) | Claude | Step 1.5 + Step 9 |

## Process Metadata

- Discussion rounds: 2 (independent research → UAG falsification & ratification)
- Round 0 framing review: 2 runs (initial 5/5 REVISE → rerun-1 2 APPROVED + 3 inline-fixed REVISE → approved with TL inline-edit close-out)
- Topics: 2 active (both converged via UAG-PASS 5/5) + 3 pre-discussion YAGNI decisions (settled at framing-edit time)
- Autonomous decisions: 2 (Topic 1, Topic 2)
- User escalations: 0
- UAG passes: 2 (Topic 1 + Topic 2; ~20 distinct counterexample attempts across 5 reviewers per topic, all failed)
- Cross-family proxy degradation: BOTH proxies degraded — Codex MCP non-responsive (2 idle pings without response across both rounds); Gemini quota exhausted from earlier in the day. Both slots filled by oMLX local models per TL fallback strategy. Cross-family coverage preserved (Alibaba lens via Qwen3-Coder for OpenAI slot; Google lens via gemma for Gemini slot). Drift / reasoning-inversion issues filtered with TL annotations on the affected per-agent files.
- Doodlestein challenges: 7 raised across 3 post-conclusion reviewers. 0 invalidated converged decisions. 4 absorbed as plan-time TODOs. 2 filed as new backlog items (BL-013 orphan-link-row cleanup; BL-014 mengdie audit-stats subcommand). 1 cross-reference disposition (BL-014 already covers operator-discoverability concern). No new round fired.
- Deferred resolved in Sweep: 0 (Sweep was a no-op)

## Recorded dissents

None recorded. The Round 1 gemma Topic-1 reasoning inversion is preserved in `round-01/gemini-proxy.md` with TL annotation, but gemma corrected to Option B in Round 2 — no surviving dissent.

The minor archaeologist↔database-optimizer disagreement on the load-bearing reason for transaction-coupled difficulty under Option B (architectural-boundary vs mutex-cycle) is a labeling difference, not a substantive dissent. Both agents agree transaction-coupled is hard; both reject it. Preserved in their respective Round 2 files for audit but does not affect the Topic 2 outcome.

## Next Steps

→ `/ae:plan` for converged decisions: F-002 plan implements the v6 migration (audit + link tables + 3 indexes per R4), `Db::record_search_audit(...)` Db-level helper per R1, hook invocation at `mcp_tools.rs` after the search-result match block (Option B per Topic 1) AND at `cli.rs:609`, best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES` counter per Topic 2.

The plan should explicitly note: Wave 2 BL-009/BL-010 migration moves the call site (per R5); schema is stable across the migration. The supersession SQL from F-002 analysis.md is the v0.0.1 acceptance test (runs against the seeded schema correctly) but has no in-binary caller — read path remains deferred.
