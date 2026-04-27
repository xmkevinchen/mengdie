---
id: "023-doodlestein-regret"
type: "post-conclusion-review"
reviewer: "doodlestein"
date: 2026-04-24
subject: "Which decision in conclusion 023 is most likely reversed in 6 months?"
---

# Doodlestein Regret Analysis — Discussion 023

## Verdict

**Most likely reversed: defer FK pragma to v0.9.0+**

## Reasoning

The conclusion frames the FK-pragma deferral as trigger-not-fired: no `DELETE FROM memory_entries` path exists, so the invariant isn't needed yet. That framing is technically correct on the day of the decision.

The problem is what fires the trigger. BL-009 is the very next item on the roadmap — a `memory_dream` MCP tool that routes synthesis writes through `memory_ingest` (the bypass path). The conclusion's own Key Finding #1 documents that `memory_ingest` → `insert_memory` does NOT compute `synthesis_cluster_hash` and creates NULL-cluster synthesis rows. Any caller using this path writes `source_type='synthesis'` rows without populating `memory_synthesis_links`. That is not an FK orphan in the conclusion's strict sense, but it is structurally adjacent: synthesis rows exist with zero link entries, which future provenance queries (e.g. BL-012 RAG citations, BL-013 graph traversal) will need to join on.

Once BL-009 ships, the `memory_dream` MCP tool hands Claude the cluster list and Claude calls `memory_ingest` to write syntheses. At that point you have:

1. Claude writing `source_type='synthesis'` rows via the bypass path.
2. Synthesis rows with no entries in `memory_synthesis_links`.
3. BL-012 or BL-013 attempting to join on those links and silently getting empty result sets.

At that moment someone will want to add a `mengdie prune` or a `memory_invalidate` cleanup for orphaned synthesis rows. That is the first real `DELETE FROM memory_entries` path, and it will fire the FK-pragma trigger under the trigger's own literal text.

The gap between "defer to v0.9.0+" and "trigger actually fires" is likely one or two plan cycles, not a multi-month wait. v0.9.0 is the obvious boundary because BL-009 is the nominated next `/ae:discuss` item and the conclusion's own next-steps direct that work immediately.

## Why the other candidates are less likely to reverse

**Cut v0.8.5 vs skip**: Low reversal probability. The conclusion's reasoning that this is a recognition event (not manufactured discipline) is sound. The 28-commit burst pattern validates the cadence-tag model. No external forcing function is likely to make skipping a tag look wrong in hindsight.

**Cluster-hash NOT NULL scope**: The conclusion correctly identified the bypass path as real (production orphan confirmed) and scoped the fix to a schema micro-migration + trigger pair. The fix is proportional. The only reversal scenario is if the trigger pair proves insufficient — e.g. if a future caller writes syntheses outside `insert_synthesis_with_links` through a new path — but that would be a new bug, not a reversal of the v0.8.5 decision.

**Rejection of residuals-CLI**: Correct on patch convention grounds. A new CLI subcommand is a minor feature, not a patch item. This call does not reverse — at worst, residuals-CLI gets a proper BL and a v0.9.0 slot, which is exactly the right outcome.

## Confidence

High. The FK-pragma reversal is not speculative: the conclusion's own next-steps guarantee BL-009 work begins soon, and BL-009's design (Claude calls `memory_ingest` for synthesis writes) triggers the FK-pragma condition directly. The only variable is timing: whether it lands within v0.8.5 close-out or v0.9.0 start.

## Suggested follow-up

File a new BL or annotate BL-fk-pragma-and-deletion-safety with an explicit secondary trigger:

> **Additional trigger**: BL-009 ships `memory_dream` MCP tool AND Claude writes synthesis rows via `memory_ingest` (i.e. the bypass path becomes a production caller).

This makes the trigger fire at BL-009 ship time, not at "first deletion-introducing plan" (which may come later). The fix is still a one-liner (`PRAGMA foreign_keys = ON` in `run_migrations`) plus an optional ON DELETE CASCADE clause if the v5 migration opportunity exists. No reason to wait for a deletion path when the provenance join dependency lands first.

Shape: annotate BL-fk-pragma-and-deletion-safety body, no new BL needed. One sentence addition to the Trigger section.
