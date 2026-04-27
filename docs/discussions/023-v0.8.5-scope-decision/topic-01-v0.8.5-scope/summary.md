---
id: "01"
title: "v0.8.5 delivery shape + scope"
status: converged
current_round: 2
created: 2026-04-27
decision: "Cut v0.8.5 with: (1) backlog-hygiene prerequisite commit (migrate 6 docs/backlog/ BLs to .ae/backlog/unscheduled/, dedup FK BL pair); (2) BL-dreaming-module-split (S, ~100-150 LOC); (3) BL-synthesis-cluster-hash-not-null-enforcement (S, v6 micro-migration); (4) BL-v5-migration-operator-docs (XS). Defer FK pragma + residuals work to v0.9.0+."
rationale: "Three trigger-fired items: module-split (BL-008 shipped 7 days ago), cluster-hash NOT NULL (verified — memory_ingest path bypasses insert_synthesis_with_links invariant; production orphon 529d3212 is the manifestation), v5-operator-docs (timely with imminent prod migration). Backlog hygiene is required structural prerequisite (challenger F8: 6 BLs in docs/backlog/ invisible to /ae:roadmap, dual-tracked FK BL with divergent triggers). FK pragma trigger has NOT fired per literal BL text — challenger correctly distinguished orphan-zero-link (memory_entries) from FK-orphan (memory_synthesis_links); they're different mechanisms. Residuals-CLI rejected unanimously: violates 0.x.5 patch convention + no BL exists."
reversibility: high
reversibility_basis: "Sprint commitments soft. /ae:roadmap remove + /ae:roadmap close --bump-remaining v0.9.0 are escape hatches. Worst case: drop cluster-hash NOT NULL mid-sprint and ship 3-item v0.8.5; total cost a few hours of agent work."
---

# Topic: v0.8.5 delivery shape + scope

## Current Status

**CONVERGED** in Round 2.

Path forward: Cut v0.8.5 with 4 items + 1 prerequisite commit. Deferred work routed clearly. Independent ops tasks (CLAUDE.md cleanup, production v5 migration, BL-009 discuss) called out separately.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | revisit | 5 positions, structural gap surfaced (challenger F8: BL location dual-tracking). New evidence required: BL-009 design state + memory_ingest path inspection. |
| 2 | converged | Architect F1 (verified via TL code grep): `memory_ingest` → `insert_memory` does NOT compute synthesis_cluster_hash, does NOT insert link rows. BL-009's design (005-phase2-roadmap.md:67) routes through this path. Production orphan `529d3212-...` is the manifestation. Cluster-hash NOT NULL trigger now confirmed fired. Codex + gemini conceded earlier overreaches; minimal-change isolated on Path X (no-tag); architect + challenger converged on cut-with-verified-set. |

## Context (preserved from creation)

mengdie v0.8.0 closed 2026-04-24. v0.9.0 named anchor: BL-009 MCP Dream Tool. v0.8.5 was the contested question.

## Constraints (preserved)

- Trigger-discipline rule (021)
- 0.x.5 patch convention (no new features)
- Phase 2 dependency chain (BL-009 → BL-010 → BL-011/BL-013)
- Production orphan blocks v5 migration

## Decision details

See `docs/discussions/023-v0.8.5-scope-decision/conclusion.md` for full Decision Summary table + rationale + spawned-discussions table.
