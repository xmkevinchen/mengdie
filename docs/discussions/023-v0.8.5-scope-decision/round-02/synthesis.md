---
round: 2
date: 2026-04-27
type: tl-synthesis
---

# Round 2 — TL Synthesis

## Per-agent files

- `round-02/codex-proxy.md`
- `round-02/minimal-change-engineer.md`
- `round-02/gemini-proxy.md`
- `round-02/software-architect.md`
- `round-02/challenger.md`

## Major position movements (Round 1 → Round 2)

| Agent | Round 1 | Round 2 | Driver |
|-------|---------|---------|--------|
| codex-proxy | FK + cluster-hash both P1 critical | FK + module-split + ops-docs; **defer cluster-hash to BL-009 design** | minimal-change's "fear of unbuilt feature" partially conceded |
| gemini-proxy | residuals-CLI + module-split | residuals shifts to **docs+logging** (CLI conceded as patch-convention violation); FK/cluster-hash gated on BL-009 | minimal-change + 021 trigger discipline |
| challenger | FK not fired; cluster-hash deferred; one fired item insufficient for tag | **Cluster-hash NOT NULL is FIRED** based on BL-009's published design; FK still not fired; module+cluster-hash = 2 fired items, sufficient | F1 evidence: BL-009 design at 005-phase2-roadmap.md:67 + `memory_ingest` path bypasses invariant |
| software-architect | conditional cut | Verifies F1 independently; **cut v0.8.5 with module+FK+cluster-hash+ops-docs**; backlog migration is prerequisite commit | Code trace: `insert_memory` doesn't compute cluster_hash, doesn't insert links |
| minimal-change | skip v0.8.5 | Refined: **Path X (no-tag commits)** + bookkeeping + module split + CLAUDE.md fix; rejects pre-hardening for unknown caller | Architect's cadence-tag model strengthens skip; codex "trigger imminent ≠ fired" |

## Pruned

- **Pruned**: residuals-CLI subcommand. Gemini conceded violates patch convention; not a BL; minimal-change + architect + challenger all rejected. Replaced (in gemini's revised view) by docs/logging-only treatment of the residuals UX issue.
- **Pruned**: codex's reading of "production orphan = FK pragma trigger fired." Challenger's mechanism correction (orphan is zero-link in memory_entries, not FK-orphan in memory_synthesis_links) is structurally correct and verified by TL. The orphan IS evidence — but for cluster-hash NOT NULL, not for FK pragma.
- **Pruned**: minimal-change's "fear of unbuilt feature" critique of cluster-hash NOT NULL. Round 2 evidence (architect F1 + challenger Round 2) shows the failure mode is in already-shipped code (`memory_ingest` path), not speculative future code. The "fear" framing applies to FK pragma, not cluster-hash.
- **Pruned**: nothing else; all five positions advanced to scoring.

## Of-framing disposition

Round 2 of-framing challenges:

1. **Architect F1 (memory_ingest bypass)**: "BL-009 readiness" was framed as a future-feature concern, but the bug is in already-shipped code. **Disposition: integrate.** TL verified via code trace (db.rs:122-163, mcp_tools.rs:382, 005-phase2-roadmap.md:67). This converts cluster-hash NOT NULL from "speculative" to "currently-vulnerable."
2. **Minimal-change Path X (no-tag commits)**: argues against the framing's "if v0.8.5 ships, what's in it" Q3 by proposing a non-version-tagged shape. **Disposition: integrate as live option.** Path X and Path Y (cut v0.8.5) both honor trigger discipline; the difference is just version-tag aggregation, which the framing's Q1 explicitly listed as a legitimate spectrum point.
3. **Challenger structural gap (BL location dual-tracking)**: discovered late but verified by TL. **Disposition: integrate as prerequisite.** Both Path X and Path Y require backlog-hygiene work; this isn't optional under either path.

## Verification artifact

| Claim | Source | Verified? |
|-------|--------|-----------|
| `insert_memory` does NOT compute synthesis_cluster_hash; does NOT insert memory_synthesis_links rows | TL grep + read of db.rs:122-163 (2026-04-27 18:08Z) | ✓ |
| `memory_ingest` MCP tool dispatches to `insert_memory` / `insert_memory_resolving` (line 382/384) | TL grep mcp_tools.rs | ✓ |
| BL-009 design at 005-phase2-roadmap.md:66-71: "Claude synthesizes inline and calls `memory_ingest`" | TL read | ✓ |
| Production orphan `529d3212-...` is zero-link in memory_entries (not FK-orphan in links table) | CLAUDE.md + plan 017 Step 3 manual smoke test | ✓ structural; not yet directly inspected on user DB this session |
| 6 BLs in docs/backlog/ are not in .ae/backlog/unscheduled/ | TL ls of both dirs | ✓ |
| BL-fk-pragma-and-deletion-safety.md (docs/backlog/) ↔ BL-enable-pragma-foreign-keys.md (unscheduled/) — same root concern, divergent triggers | TL `ls` confirmed both files exist | ✓ structural; trigger-text comparison done by minimal-change Round 2 |
| BL-009 has only 6-line stub design; no full discussion doc | minimal-change Round 1 + architect Round 2 (resolved Q1) | ✓ |
| CHANGELOG.md exists with `## Unreleased` | architect Round 2 (resolved Q2) | ✓ — TL added v0.8.0 entry on 2026-04-24 close |
| dreaming.rs is 1326 LOC | challenger Round 1 (`wc -l`) | ✓ |
| Architect's "BL-008 split was HALF honored" — math went to decay.rs but orchestration stayed in dreaming.rs:157-311 | architect Round 1 + Round 2 | ✓ via repo file structure |

## Frame-challenge disappearance self-check

Round 1 carried 5 disagreements forward to Round 2. Status check:

- ✓ #1 (skip vs cut): explicitly addressed by every agent; minimal-change refined to Path X.
- ✓ #2 (FK pragma fire status): explicitly addressed; challenger + minimal-change vs codex + architect now have CLEARER positions; cluster-hash NOT NULL got DECOUPLED from FK pragma in this round (different mechanisms).
- ✓ #3 (cluster-hash NOT NULL urgency): RESOLVED — challenger reversed Round 1, architect verified independently, codex agrees-in-spirit, gemini conceded gating-on-design; minimal-change now isolated on this.
- ✓ #4 (residuals CLI): RESOLVED — gemini conceded; rejected by all.
- ✓ #5 (backlog migration prerequisite): RESOLVED — all 5 agree it's needed regardless of skip/cut.

No silent disappearances. All five round-1 disagreements engaged in round 2.

## Convergence map

**Convergent (4-5 agree)**:
- ✓ **Backlog hygiene** (migrate docs/backlog/ legacy BLs to .ae/backlog/unscheduled/, dedup FK BL pair). All 5 agree this is required regardless.
- ✓ **BL-dreaming-module-split** trigger fired. All 5 agree on the trigger; minimal-change routes as no-version-tag commit, others as v0.8.5 item. Same work.
- ✓ **BL-cluster-hash NOT NULL** trigger fired. 4/5 agree post-Round-2 (codex/architect/challenger/gemini-conceded). Only minimal-change still skeptical, but on grounds that mostly evaporated when F1 evidence converted the issue from "BL-009 future" to "already shipped vulnerability."
- ✓ **No new features in v0.8.5**. All 5 agree (residuals CLI rejected unanimously).
- ✓ **BL-009 needs `/ae:discuss` before `/ae:plan`**. 5/5 agree (architect resolved Q3).

**Contested**:
- ✗ **FK pragma trigger fire status**: 2 yes (codex, architect-conditional) vs 3 no (challenger, minimal-change, gemini-conceded). The challenger mechanism correction (orphan ≠ FK fire) is structurally correct. **TL decision: trigger NOT fired per literal BL text + correct mechanism.**
- ✗ **Skip vs Cut v0.8.5**: 1 skip (minimal-change Path X) vs 4 cut (codex/architect/gemini-revised/challenger-tentative-revised). The work is identical; the question is version-tag aggregation only.

**Not converged but not blocking**:
- v5-migration-operator-docs: codex+architect support; others didn't reject. Cheap include.

## TL recommended decision (for Step 5 scoring)

**Topic 01 score: converged**

**Decision**: Cut **v0.8.5** with this set:
1. **Backlog hygiene commit** (prerequisite; pre-sprint or first commit of sprint): migrate 6 BLs from `docs/backlog/` to `.ae/backlog/unscheduled/`; dedup BL-fk-pragma-and-deletion-safety into BL-enable-pragma-foreign-keys (keep latter; `git rm` former).
2. **BL-dreaming-module-split** (S, ~100-150 LOC pure refactor): orchestration half — extract from dreaming.rs:157-311 into separate module.
3. **BL-synthesis-cluster-hash-not-null-enforcement** (S, schema v6 micro-migration + trigger pair): closes the verified `memory_ingest` bypass path.
4. **BL-v5-migration-operator-docs** (XS, doc-only): operator runbook for the production migration that will run during/after this sprint.

**Defer to v0.9.0 (or later)**:
- BL-enable-pragma-foreign-keys: trigger not fired per literal text + corrected mechanism.
- Residuals-clarity work: not a BL; defer to BL-009 design discussion or a separate UX scope.

**Independent tasks (not part of v0.8.5 scope, but required regardless)**:
- CLAUDE.md "Next step (current): residuals reduction" — stale post-plan-011; one-line fix.
- Production v5 migration on `~/.mengdie/db.sqlite` — operator action; orphan resolution + run.
- `/ae:discuss BL-009` — BL-009 design has only 6-line stub; needs full discussion before /ae:plan.

**Rationale**: Cluster-hash NOT NULL trigger has fired (verified — `memory_ingest` path bypasses the invariant; production orphan is the manifestation). Module-split trigger has fired (BL-008 shipped 7 days ago). Backlog hygiene must happen anyway. v5-operator-docs is timely since the migration will happen during this window. FK pragma defers to genuine trigger fire. Residuals work is feature-shape and goes to v0.9.0+. Rejecting Path X (skip-with-no-tag) on the grounds that v0.8.5 has 3 fired-trigger items not 1, which crosses the "1-item-tag-overhead" objection challenger raised in Round 1 (and revised in Round 2 to "fired-trigger count is now 2, sufficient").

**Reversibility**: HIGH. Sprint commitments are soft (mid-sprint move/remove exist). Worst case: cut v0.8.5, find issue, revert cluster-hash work, ship just module-split + ops-docs as v0.8.5; or close v0.8.5 early.

**Reversibility basis**: `/ae:roadmap remove` exists for any item; `/ae:roadmap close --bump-remaining v0.9.0` exists for emergency early-close.

## Step 5 escalation check

**Decide autonomously?** Yes — team evidence (4/5 cut, with the 1 skip having identical work shape) supports the direction. The "cut vs no-tag" disagreement reduces to version-aggregation aesthetics; both paths do the same work. Per the spec's "Decide, don't ask" principle and the High reversibility, TL decides.

**User-affecting?** Mildly — Kai will see a v0.8.5 tag emerge. But Kai initiated v0.8.5 in conversation, so this aligns with stated preference.

**Decision**: TL scores converged; presents to user as FYI not as escalation; user can override with /ae:roadmap remove if any item is wrong.
