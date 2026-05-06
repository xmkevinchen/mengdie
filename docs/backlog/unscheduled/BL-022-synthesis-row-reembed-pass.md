---
id: BL-022
title: "Synthesis rows are stored with embedding=None — re-embed pass required for vector search + secondary loop signals"
status: open
created: 2026-05-05
origin: "discussion 027 archaeologist round-01 verification (dreaming.rs:569-570 + clustering.rs:71-79)"
trigger: "Cron-default reflection trigger consideration OR T5 secondary signal landing (synthesis-influencing-search rate)"
depends_on: []
size: S
v_target: "v0.0.1.x patch window — gates cron-default trigger and T5 secondary metrics"
---

# BL-022 — Synthesis row re-embed pass

## Origin

Surfaced in discussion 027 by `archaeologist` (Round 1, file:line evidence in `round-01/archaeologist.md` and reproduced in `round-02/archaeologist.md`):

> "embedding=None at creation (`dreaming.rs:569-570`); rg confirms zero re-embedding pass anywhere in codebase. Synthesis rows excluded from all future clustering (`clustering.rs:71-79` SQL filter) and rank lower in vector search."

This is a hard structural gap, not a configuration issue. The synthesis pass produces new memory rows (LLM-consolidated cross-cluster facts) but never embeds them, so they cannot participate in vector retrieval or future clustering.

## Why this matters

Two concrete v0.0.1 decisions are gated on this fix:

1. **Topic 2 (reflection trigger) — cron-default**: Discussion 027 settled on on-demand as the v0.0.1 default trigger. ai-engineer's Round 2 analysis: cron-default would not be defensible until synthesis rows are queryable, because if reflection runs autonomously and its output is invisible to subsequent retrieval, the loop cannot close. On-demand is fine because the operator manually closes the loop by reading synthesis output directly.
2. **Topic 5 (loop signal) — synthesis-influencing-search rate**: Listed as P1 secondary metric in the discussion 027 conclusion. system-architect's Round 1 + Round 2 metric depends on F-002's `audit_returned_facts` table containing synthesis rows. If synthesis embeddings are null, vector search systematically under-counts synthesis presence in returned-fact ranks (FTS5 may still surface them by lexical match, but the metric becomes lopsided).

## Implementation sketch

Two viable paths:

- **Eager**: embed synthesis text at creation in `dreaming.rs::synthesize_cluster` before insert. Requires fastembed-rs call inside the synthesis transaction. Adds latency (2-10ms per synthesis) but ensures every synthesis row is queryable immediately.
- **Lazy**: separate re-embed pass that scans `memory_entries` for `embedding IS NULL` rows and computes embeddings in a batch. Runs as a maintenance task (`mengdie reembed-synthesis`) or as a follow-up to `mengdie dream`. Decouples synthesis transaction from embedding cost.

Eager is simpler and matches how non-synthesis ingestion already works. Recommended unless profiling shows synthesis transactions becoming a hotspot.

## Acceptance criteria

- All synthesis rows produced after the fix have non-NULL embeddings
- A backfill pass re-embeds existing synthesis rows (~13 syntheses from the first real `mengdie dream --synthesize` run, per CLAUDE.md Project Status)
- `clustering.rs:71-79` SQL filter is updated or removed if no longer needed (synthesis rows can now participate in subsequent clustering)
- Vector search returns synthesis rows when their content is semantically relevant to the query

## Trigger

Fires when either:
- Cron-default reflection trigger is reconsidered (depends on this fix)
- The T5 synthesis-influencing-search rate metric is filed as a follow-on BL (this fix is its prerequisite)
- Operator-visible audit shows synthesis rows are systematically absent from search results despite relevance
