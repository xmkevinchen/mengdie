---
reviewer: doodlestein-strategic
type: post-conclusion-review
scope: single-smartest-improvement
reopen: false
---

# Strategic Review — Discussion 023 Conclusion

## Verdict

The conclusion is sound. The decision table, trigger-fire rationale, and
FK-vs-orphan distinction are all correctly reasoned. The process metadata
confirms the key reversals were captured and the verification artifacts are
cited. No reopen warranted.

## Single Smartest Improvement

**Add an explicit BL-009 sequencing gate to the v0.8.5 next-steps block.**

### The gap

The conclusion correctly defers `/ae:discuss BL-009` to after v0.8.5 closes
(or earlier if v0.8.5 stalls). But it doesn't surface the coupling risk: the
cluster-hash NOT NULL enforcement (BL-synthesis-cluster-hash-not-null-enforcement)
is the structural centerpiece of v0.8.5, and its schema invariant is designed
around how synthesis rows are written today — specifically, the
`insert_synthesis_with_links` path.

BL-009 is the next caller that will write synthesis rows via a *different*
path (the AE knowledge-capture protocol's `mcp__mengdie__memory_ingest`). If
the BL-009 design discussion concludes that this path must write
`source_type='synthesis'` rows directly through `insert_memory` rather than
`insert_synthesis_with_links`, then the NOT NULL constraint shipped in v0.8.5
immediately becomes a migration blocker in v0.9.0 — the very write path that
BL-009 needs is the bypass path the constraint is designed to catch.

This is not a reason to defer BL-synthesis-cluster-hash-not-null-enforcement
out of v0.8.5 (the constraint is correct and needed; the production orphan is
real). It's a reason to run `/ae:discuss BL-009` *before* `/ae:roadmap plan
v0.8.5` rather than after — or at minimum to add it as a blocking gate in the
next-steps block so the sequencing is explicit.

### Proposed addition to Next Steps (conclusion.md line 77-80)

Insert before the `/ae:roadmap plan v0.8.5` line:

> **Gate: run `/ae:discuss BL-009` first** (or confirm its write path goes
> through `insert_synthesis_with_links`, not `insert_memory`). If BL-009
> routes through the bypass path, the cluster-hash NOT NULL constraint design
> needs to absorb that caller before v0.8.5 ships — not after. Estimated
> effort: one discussion round to confirm the write-path assumption; if
> confirmed compatible, proceed immediately.

### Classification

This is a **framing tightening + downstream linkage** improvement. The
conclusion already notes the BL-009 dependency; it just sequences it as
post-v0.8.5 without acknowledging the coupling risk that makes the ordering
matter. Adding the gate converts an implicit assumption into an explicit
checkpoint.

No reopen required — the conclusion's decision table stands. The improvement
is additive to the next-steps block only.
