---
id: "023"
title: "v0.8.5 scope decision — Conclusion"
concluded: 2026-04-27
plan: ""
entities: [v0.8.5, scope, sprint-planning, backlog-hygiene, dreaming-module-split, cluster-hash-not-null, fk-pragma, residuals-cli, memory-ingest-bypass, version-tag-aggregation]
---

# v0.8.5 scope decision — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | v0.8.5 delivery shape + scope | Cut v0.8.5 with 4 items + 1 prerequisite commit. Items: (a) backlog-hygiene prerequisite commit (migrate 6 BLs from docs/backlog/ to .ae/backlog/unscheduled/, dedup BL-fk-pragma-and-deletion-safety into BL-enable-pragma-foreign-keys); (b) BL-dreaming-module-split (S, ~100-150 LOC; extract `run_synthesis_pass` at dreaming.rs:399 + `SynthesisResult` at dreaming.rs:330 into a new `synthesis_pass.rs` module; leave `run_dreaming_with_config` at dreaming.rs:85 and decay logic in dreaming.rs); (c) BL-synthesis-cluster-hash-not-null-enforcement (S, schema v6 micro-migration + trigger pair); (d) BL-v5-migration-operator-docs (XS, doc-only — must include the orphan→v5→v6 ordering runbook per Doodlestein adversarial F2, not just the v5 coalesce heuristic). Defer FK pragma to v0.9.0+ (trigger not fired per literal text on the v0.8.5 timescale; secondary trigger added per Doodlestein regret). Reject residuals-CLI (violates patch convention, no BL exists). | Three trigger-fired items: module-split (BL-008 shipped 2026-04-20), cluster-hash NOT NULL (verified via TL code grep — `memory_ingest` → `insert_memory` bypasses cluster-hash invariant; production orphan `529d3212-...` is the shipped manifestation; BL-009's published design at 005-phase2-roadmap.md:67 routes through same path), v5-operator-docs (timely before prod migration). Backlog hygiene is structural prerequisite — challenger verified 6 BLs in docs/backlog/ are invisible to /ae:roadmap. FK pragma trigger NOT fired: challenger correctly distinguished zero-link orphan in memory_entries from FK orphan in memory_synthesis_links — different mechanisms. Patch-convention compliance: no new features (4 items are refactor + schema-integrity + docs). | high (sprint commitments are soft; `/ae:roadmap remove` + `/ae:roadmap close --bump-remaining v0.9.0` are escape hatches; worst-case revert of cluster-hash work costs hours not days) |

## Doodlestein Review

3 fresh agents reviewed the written conclusion. All 3 findings actionable; none required reopening a discussion round. All integrated inline into the Decision Summary + Next Steps.

### Strategic (BL-009 sequencing gate)

**Finding**: Conclusion defers `/ae:discuss BL-009` to post-v0.8.5 but doesn't surface the coupling risk. BL-009 may route through `insert_memory` (the bypass path), and v0.8.5's cluster-hash NOT NULL constraint will reject those writes — exposing a BL-009 design gap on first use.

**Disposition**: integrated. Reframed: this is the POINT of v0.8.5 (force BL-009 to design synthesis-write-path correctly), not a v0.8.5 failure. Added explicit BL-009 sequencing pointer to Next Steps: prefer running `/ae:discuss BL-009` BEFORE `/ae:roadmap plan v0.8.5` so the BL-009 design accommodates the new constraint (or surfaces a need to amend it).

### Adversarial F1 (line-number error)

**Finding**: Conclusion cited `dreaming.rs:157-311` as orchestration-half refactor target. Those are decay-pass lines (`run_dreaming_with_config` is at line 85). The actual synthesis orchestration is `run_synthesis_pass` at line 399 + `SynthesisResult` at line 330.

**Disposition**: integrated as factual correction. Updated Decision Summary table line-citations. Verified by TL via `rg -n "fn run_dreaming_with_config|fn run_synthesis_pass|pub struct SynthesisResult" src/core/dreaming.rs`.

### Adversarial F2 (v6-without-v5 silent failure on prod)

**Finding**: schema.rs Pre-check 2 (lines 294-310) aborts v5 migration on zero-link synthesis rows. v6 (cluster-hash NOT NULL) builds on v5. The v0.8.5 gate says "Production v5 migration runnable" — not "complete." A plan executor can ship v6 cleanly (in-memory tests, no orphan), close v0.8.5, then operator upgrades and hits v5 pre-check abort on first startup. BL-v5-migration-operator-docs covers v5 coalesce heuristic but not the orphan→v5→v6 ordering.

**Disposition**: integrated. Two changes: (1) BL-v5-migration-operator-docs scope expanded to include the orphan→v5→v6 ordering runbook (Decision Summary + Next Steps both updated); (2) v0.8.5 gate condition strengthened from "Production v5 migration runnable" to "Production v5 migration COMPLETE on the user's `~/.mengdie/db.sqlite`" — see updated Next Steps gate text.

### Regret (FK pragma deferral)

**Finding**: "Defer FK pragma to v0.9.0+" is the most likely reversed decision. BL-009 routes synthesis writes through `memory_ingest` (the bypass path for the cluster-hash work this conclusion blesses). Once BL-009 ships, Claude writes `source_type='synthesis'` rows with no link entries. Any future provenance join (BL-012 RAG citations, BL-013 graph traversal) or cleanup operation against those rows fires the FK pragma trigger under its own literal text. Gap is 1-2 plan cycles, not multi-month.

**Disposition**: integrated as BL annotation. After backlog dedup, BL-enable-pragma-foreign-keys gets a secondary trigger added: "BL-009 ships memory_dream MCP tool AND Claude writes synthesis rows via memory_ingest (gap: 1-2 plan cycles after v0.8.5)." One sentence added to the Trigger section. No new BL. Note: this is a v0.8.5 close-out follow-up (after backlog hygiene migrates the file), not a v0.8.5 sprint item.

## Spawned Discussions

| # | Topic | New Discussion | Reason |
|---|-------|----------------|--------|
| (none — all topics resolved within discussion 023) |

## Deferred Resolutions

| # | Topic | Resolution | Detail |
|---|-------|------------|--------|
| (none — Round 2 sweep cleaned all deferred items) |

## Out-of-scope items called out (independent ops tasks)

These do NOT belong in v0.8.5; they are independent obligations:

1. **CLAUDE.md cleanup**: "Next step (current): residuals reduction" line is stale post-plan-011. One-line fix. Should land in a separate housekeeping commit, not v0.8.5.
2. **Production v5 migration**: orphan synthesis row `529d3212-e809-4b81-a1f5-e15143df5128` blocks the migration on `~/.mengdie/db.sqlite`. Operator must resolve (delete or restore links) and run migration. Timely before v0.8.5 ships cluster-hash NOT NULL — but v0.8.5's BL-v5-migration-operator-docs writes the runbook for this very action.
3. **`/ae:discuss BL-009`**: BL-009 has only a 6-line stub in `docs/backlog/005-phase2-roadmap.md:66-71`. Needs full design discussion before /ae:plan can run. Should be queued as a v0.8.5 close-out follow-up (or earlier if v0.8.5 is stalling).

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude | Start |
| codex-proxy | technical-debt + risk-of-deferral lens (OpenAI) | Claude-direct fallback (codex MCP unavailable model-version-mismatch) | Start (Round 1) |
| minimal-change-engineer | anti-bloat / minimum-machinery | Claude | Start (Round 1) |
| gemini-proxy | product / UX / momentum lens (Google) | oMLX gemma4:26b fallback (Gemini API rate-limited) | Start (Round 1) |
| software-architect | system-shape + dependency analysis | Claude | Start (Round 1) |
| challenger | pure adversarial opposition | Claude | Start (Round 1) |

Round 0 framing-review team (separate lifecycle): codex-proxy, gemini-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer — 3 attempts, framing override on attempt 3 with user approval.

## Process Metadata

- Discussion rounds: 2 (plus Round 0 framing-review × 3 attempts)
- Topics: 1 total (1 converged, 0 spawned, 0 deferred-explained)
- Autonomous decisions: 1 (Topic 1 — TL decided per evidence + spec "Decide, don't ask")
- User escalations: 1 (Round 0 attempt 3 hit rerun-limit; user chose Override)
- Doodlestein challenges: pending Step 9
- Deferred resolved in Sweep: 0 (no deferred items existed)
- Round-1-to-Round-2 position reversals: 2 (challenger reversed on cluster-hash NOT NULL fire status; codex reduced scope claim)
- Verification artifact items checked by TL: 11 (8 ✓ verified, 3 ✓ verified by another agent)

## Key Findings (load-bearing for downstream)

1. **`memory_ingest` bypass path is real and currently shipped** (architect F1 + TL verification): `db.rs:122-163` `insert_memory` does NOT compute `synthesis_cluster_hash` and does NOT insert into `memory_synthesis_links`. ON CONFLICT predicate `WHERE source_type != 'synthesis'` excludes synthesis rows from content-hash dedup. So any caller (including AE knowledge-capture protocol's `mcp__mengdie__memory_ingest` and Claude under BL-009) writing `source_type='synthesis'` via this path creates a row with NULL cluster_hash that is invisible to `idx_synthesis_cluster`. The plan 017 cluster-hash invariant was scoped only to `insert_synthesis_with_links`, which the production orphan demonstrates is insufficient.

2. **The orphan-vs-FK distinction matters** (challenger): the production orphan `529d3212-...` is a synthesis row in memory_entries with zero entries in memory_synthesis_links. This is NOT an FK orphan in the link table (which would be a link row pointing to a missing memory_entries.id). The two failure modes are independent. The orphan is evidence for cluster-hash NOT NULL trigger, not for FK pragma trigger.

3. **Backlog dual-tracking is a real structural gap**: 6 BLs in `docs/backlog/` are not in `.ae/backlog/unscheduled/`. Includes BL-dreaming-module-split (the cleanly-fired item), BL-fk-pragma-and-deletion-safety (duplicate of BL-enable-pragma-foreign-keys with divergent trigger), BL-clustering-validation, BL-synthesis-cli-skip-metric, BL-synthesis-result-struct-promotion, BL-valid-until-boundary. Until migrated, /ae:roadmap silently skips them.

4. **Cadence-tag delivery model fits mengdie's observed cadence** (architect): the 28-commit burst on 2026-04-23 across plans 015+016+017 shows discipline lives at plan-level, not at sprint-boundary. Version tags are aggregation events at plan-burst boundaries. v0.8.5 isn't manufactured discipline — it's recognition that backlog-hygiene + 3 fired-trigger items have accumulated enough to warrant a tag.

## Next Steps

In order:

1. **Independent ops cleanup** (no version tag; small commits to main):
   - One-line CLAUDE.md fix removing stale "Next step (current): residuals reduction" line. ✓ Done 2026-04-27.
   - **Production v5 migration is NOT a chat-time ad-hoc task.** Initial attempt on 2026-04-27 hit the pre-check chain in cascading order: (a) pre-check 2 caught zero-link synthesis row `529d3212-...`, (b) deletion created an FK-orphon link row caught by pre-check 1 on retry, (c) pre-check 3 then caught 69 rows with `source_type='unknown'` (corpus drift from early ingest era — 9 test-report rows + others). Each step required reactive judgment without audit trail. DB rolled back to backup (`~/.mengdie/db.sqlite.pre-v5-backup-20260427-140457`) before further damage. The migration is an actual deliverable of `BL-v5-migration-operator-docs` (per Doodlestein adversarial F2 — orphan→v5→v6 ordering runbook). The BL's scope must include: (a) corpus pre-audit script enumerating `source_type` outside allowlist, zero-link synthesis rows, FK-orphon link rows; (b) reclassify decision tree (e.g., test-report→review, topic→analysis) with explicit confirmation gates per project; (c) operator runbook describing the strict orphan→v5→v6 ordering. The migration runs as part of executing `BL-v5-migration-operator-docs` in v0.8.5, not before sprint commit.
2. **Backlog-hygiene commit** (no version tag; pre-sprint structural fix): `git mv` the 6 docs/backlog/ BLs to `.ae/backlog/unscheduled/`; dedup `BL-fk-pragma-and-deletion-safety` into `BL-enable-pragma-foreign-keys` (keep latter; `git rm` former); after dedup, append the Doodlestein-regret secondary trigger to `BL-enable-pragma-foreign-keys` Trigger section.
3. **`/ae:discuss BL-009`** (per Doodlestein strategic — sequence BL-009 design BEFORE v0.8.5 plan): convert the 6-line stub at `005-phase2-roadmap.md:66-71` into a real design. The v0.8.5 cluster-hash NOT NULL constraint will reject `memory_ingest` synthesis writes, so BL-009 design must accommodate this — either by updating `memory_ingest` to compute cluster_hash on synthesis writes, or by introducing a separate MCP tool path. If BL-009 design surfaces a need to amend the v0.8.5 scope, return to this conclusion before /ae:roadmap plan.
4. **`/ae:roadmap plan v0.8.5`**: `--items BL-dreaming-module-split,BL-synthesis-cluster-hash-not-null-enforcement,BL-v5-migration-operator-docs --theme "production readiness + structural debt" --gate "Production v5 migration COMPLETE on operator's primary DB; BL-009 design not blocked by schema invariants; orphan→v5→v6 runbook published in BL-v5-migration-operator-docs deliverable"`
5. **`/ae:plan` v0.8.5 items in order** (cluster-hash NOT NULL first since it's the largest blast radius; module-split second; ops-docs last so the runbook references whatever shipped).
