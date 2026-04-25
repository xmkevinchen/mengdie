---
id: "015"
title: "Review: plan 017 — v0.8.0 synthesis cluster hardening"
type: review
created: 2026-04-24
target: "docs/plans/017-synthesis-cluster-hardening.md"
verdict: pass
---

# Review 015 — Plan 017 /ae:review

**Verdict**: **PASS** after inline fixups. 1 P1 + 4 P2s + 3 P3s fixed in
`710fb46`; 3 findings filed as unscheduled backlog; 4 findings rejected
(inflated severity or false positive); 4 challenger non-findings
correctly resolved.

## Review team

Six-agent parallel review (team `plan-017-review`):

| Role | Agent | Focus |
|------|-------|-------|
| Code review | `ae:review:code-reviewer` | Idiomaticity, test isolation, dead code |
| Architecture | `ae:review:architecture-reviewer` | v5 migration invariants, partial-index orthogonality, trigger-CHECK trade-off |
| Performance | `ae:review:performance-reviewer` | Backfill scaling, N+1 risk, integrity_check cost |
| Challenger | `ae:workflow:challenger` | 9 enumerated blind-spots, pure opposition |
| Cross-family (OpenAI) | `ae:workflow:codex-proxy` | Migration safety + coalesce heuristic risk |
| Cross-family (Google) | `ae:workflow:gemini-proxy` | Ergonomics + naming + API shape |

## Findings synthesis

### P1 (blocker — fixed inline)

**P1-A (codex P1.2)**: v5 migration coalesce could tombstone the ONLY live
synthesis row when an already-invalidated sibling had a newer `created_at`.
Root cause: `by_syn` was built from the full link table without filtering
by `valid_until IS NULL`; the candidate list for the `created_at DESC` sort
included invalidated rows. An invalidated-newer sibling would win the sort
and cause the keeper-plus-invalidate logic to tombstone the real live row,
leaving the cluster with no active synthesis.

**Fix** (`src/core/schema.rs:303-378`): filter candidates to `valid_until
IS NULL` rows only; skip coalesce if fewer than 2 live candidates remain.
Preserves pre-existing `invalidation_reason` on tombstone siblings
(they're not touched).

**Regression test**: `test_migration_v4_to_v5_coalesce_ignores_already_invalidated_siblings`
seeds older-live + newer-tombstone, asserts live survives.

### P2 (fixed inline)

**P2-A (codex P1.1)**: Pre-check 1 only validated `source_memory_id`
orphans; `synthesis_memory_id` was never validated. A link row pointing to
a missing or wrong-type synthesis_memory_id could trick the coalesce into
tombstoning a real synthesis in favour of a non-synthesis row.

**Fix**: added Pre-check 1b (schema.rs, LEFT JOIN scan). Regression test
`test_migration_v4_to_v5_rejects_dangling_or_wrong_type_synthesis_memory_id`.

**P2-B (3-way convergence)**: architect P2.1 + challenger F1 + codex P2.1 all
flagged that `PRAGMA user_version = 5` sat OUTSIDE the v5 BEGIN/COMMIT
block, creating a crash window where the schema would land at v5 but
`user_version` still read 4. Not a data-corruption risk (re-run is
idempotent), but produces confusing restart log noise.

**Fix**: moved PRAGMA inside the transaction closure so it commits
atomically with the schema changes; removed the unconditional write at
function tail.

**P2-C (challenger F6)**: plan 017 Step 1 specified
`compute_synthesis_cluster_hash` "reject empty input with debug_assert
or document the convention" — code shipped with neither. Direct plan-spec
violation.

**Fix**: added `debug_assert!(!source_ids.is_empty(), ...)` + rationale
doc comment explaining the contract and why release builds still hash
deterministically.

**P2-D (challenger F8)**: plan 017 Step 3 required an integration test
covering the `<deleted>` placeholder path in `get_synthesis_with_sources`
for hard-deleted source memories. Never written.

**Fix**: added
`get_synthesis_with_sources_returns_placeholder_for_deleted_source`
(library-level test, simpler than subprocess). Asserts placeholder shape
(`source_type = "<deleted>"`, empty content, zero recall_count) + verifies
the non-deleted source still renders normally.

### P3 (fixed inline)

- **P3-A (gemini-proxy)**: `format_search_result(r, rank)` → `(r, index)`.
  `rank` was ambiguous with search score.
- **P3-B (challenger F3)**: added `test_trigger_allows_non_source_type_updates_on_synthesis_rows`
  to verify `BEFORE UPDATE OF source_type` is correctly scoped and doesn't
  fire on unrelated column updates (e.g., `recall_count`).
- **P3-C (challenger F5)**: expanded coalesce-section comment to document
  the `created_at` tie-break non-determinism (HashMap iteration order) as
  accepted risk per plan 017 Doodlestein regret note.

### Deferred (filed as backlog)

All three filed under `.ae/backlog/unscheduled/` (gitignored per project
convention).

- **BL-synthesis-cluster-hash-not-null-enforcement** (codex P1.3 + architect
  P2.3 convergence): trigger-enforced NOT NULL on `synthesis_cluster_hash`
  for live synthesis rows. Closes the "document over enforcement" gap
  architecturally. Trigger: next `memory_entries` schema migration, or
  observation of a live-synthesis row with NULL cluster_hash in production.
- **BL-get-synthesis-with-sources-n-plus-1** (performance-reviewer P2-A):
  replace per-source SELECT loop with `get_memories_by_ids` bulk fetch.
  Trigger: `max_cluster_size` routinely > 50, or audit becomes hot path.
- **BL-v5-migration-operator-docs** (codex-proxy P2.2): operator-facing
  recovery doc for the coalesce heuristic. Trigger: first production v5
  migration logs a `coalescing legacy duplicate synthesis cluster` line,
  or next doc-polish sprint.

### Rejected

- **gemini-proxy P1 "docstring placement"**: false positive — `///` lines
  already directly precede `SynthesisAudit` with no blank separator.
  rustdoc picks them up correctly.
- **gemini-proxy P1 "edge-case tests (empty entities / special chars)"**:
  inflated severity; no real failure mode evidenced. Existing snippet
  test already covers newline → space translation.
- **gemini-proxy P3 "format delimiter consistency"**: self-flagged as
  "defer unless RFC pending." Pure style.
- **code-reviewer P2s (both "add a comment")**: borderline P3 dressed as
  P2; the plan + in-code docs already capture the constraints. Skipped.

### Non-findings (challenger analysis)

- F2 (integration test isolation): tests correctly use `--db-path`, no
  leakage to `~/.mengdie/db.sqlite`.
- F4 (`insert_memory` immutability path): partial index makes synthesis
  rows invisible to content-hash conflict. Safe by design.
- F7 (pre-check → trigger-install TOCTOU): SQLite single-writer model
  prevents the race.
- F9 (partial-index asymmetric predicate): intentional design; new
  source_types auto-get content-hash dedup.

## Fixup

One cumulative fixup commit: `710fb46 plan 017 /ae:review fixups: P1 + P2s + P3s from 6-agent review`.
Follows plan 016's convention (`2d81576`). Not squashed into the original
Step 1+2 commit — the fixup is substantial enough (+4 regression tests,
+2 pre-check paths, semantic change to coalesce) that preserving it as a
distinct commit improves git-blame legibility.

## Outcome Statistics

- Steps completed: 6/6 (plan 017 shipped across 6 Steps + Completion Invariant commit)
- Rework rate: 1/6 ≈ 17% (one fixup commit touching schema.rs, cli.rs, tests/dream_synthesis.rs)
- P1 escape rate: 1 (P1-A coalesce-tombstones-live-row; caught by codex but escaped the plan 017 /ae:work pre-commit code-review track)
- Drift events: 0 (all 6 /ae:work commits matched plan Expected files)
- Fix loop triggers: 0 (no circuit-breaker activations during /ae:work)
- Auto-pass rate: high — all 6 steps auto-continued under auto_pass: true

The P1 escape (P1-A) is the significant data point. The `/ae:work`
pre-commit code-review track (single-agent Claude review) did not surface
the coalesce-row-filtering bug. The multi-agent `/ae:review` track caught
it via codex's migration-safety focus. This is consistent with the "deep
review > shallow review" value proposition: the pre-commit track is fast
and covers 80% of issues; the completion-gate review catches the specific
class of reasoning errors that only emerge under adversarial pressure.

No code-review heuristic tuning indicated — P1-A required reading the full
coalesce flow AND modeling a production-corruption scenario (existing
invalidated rows). Codex's `reasoning_effort: high` setting paid off
here; a shallower review track would likely miss it again.

### Deferred resolution rate

`docs/milestones/017/notes.md` did not exist — no deferred findings from
`/ae:work` Check 4 to audit. All P2/P3 findings came from `/ae:review`
itself, not re-queued from an earlier step.

## Tests

- Pre-fixup: 266 passed, 5 ignored
- Post-fixup: 270 passed, 5 ignored (+4 regression tests for P1-A, P2-A,
  P2-D, P3-B)

## Prior art consumed (mengdie memory_search)

- BL-006 review "document design bets in-code" pattern — applied to P2-C
  (compute_synthesis_cluster_hash empty-input contract).
- Plan 017's own migration-pre-check knowledge entry (just captured on
  2026-04-24) — reinforced the "abort loud over silent corruption"
  posture for P2-A (synthesis_memory_id pre-check).

## Next steps

Review passed. Plan 017 `status: done`, v5 migration ready for production.

Suggested follow-ups (in priority order):

1. **Run the production v5 migration** on `~/.mengdie/db.sqlite` (has a
   known zero-link orphan synthesis row `529d3212-...` surfaced during
   Step 3 smoke test — resolve the orphan before retrying migration).
2. **Close v0.8.0 sprint** — all 5 of the 5-item final slate are done:
   BL-decay-json-schema-contract, BL-decay-ops-doc-polish,
   BL-verify-decay-script-hardening, BL-synthesis-dedup-key,
   BL-synthesis-provenance. (Plus BL-ci-full-clippy-test + 006-ci-runner-env-cleanup
   closed earlier by plan 014.) Run `/ae:roadmap close v0.8.0`.
3. **Plan a v0.9.0 sprint** — BL-009 (MCP Dream Tool) is the next large
   piece; the 3 new backlog items from this review
   (cluster-hash-not-null-enforcement, get-synthesis-with-sources-n+1,
   v5-migration-operator-docs) are XS/S and could ride along as scope-dust.
