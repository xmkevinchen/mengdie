---
type: regret-analysis
plan: "017"
created: 2026-04-23
author: doodlestein
---

# Regret Analysis — Plan 017

## Verdict

One decision is meaningfully regret-prone within a 6-month horizon.

## Most Likely to Reverse: Legacy-Duplicate Coalesce Heuristic (keep newer by `created_at`)

**Decision**: When Pre-check 3 finds multiple synthesis rows covering the same source set, the migration automatically keeps the row with the latest `created_at` and invalidates the rest (Step 1, Pre-check 3).

**Why this is the risky one**: `created_at` measures recency of execution, not semantic quality. A synthesis row's timestamp reflects when `mengdie dream --synthesize` ran — not which prompt, which model, or which configuration produced it. Two plausible scenarios that break the heuristic:

1. A test run or experiment with a degraded model produces a newer but worse synthesis.
2. An operator ran `dream --synthesize` with a one-off prompt override and the older row (produced by the production prompt) was the correct one to keep.

The plan acknowledges this fragility explicitly in the Known Risk section: *"may not always hold"* and *"manual mengdie invalidate can override post-migration."* The presence of an escape hatch in the plan text is itself a signal that the team was not confident in the heuristic at write time.

**What a reversal looks like**: After the v5 migration runs on a real production DB with duplicate clusters, the operator discovers that the wrong row was kept. At that point, the options are (a) manually re-run synthesis to regenerate the discarded content, or (b) restore from a backup. Neither is catastrophic, but both are friction. More likely the operator will ask for the migration to fail loudly and present a list of conflicting row IDs for manual resolution — which is the explicit alternative the plan named and rejected.

**What would prevent reversal**: If the production DB has zero legacy duplicate clusters (likely given only 27 synthesis rows and `dream --synthesize` first ran recently), this code path never executes and the heuristic is irrelevant. The 6-month regret risk is conditional on duplicates actually existing at migration time.

## Other Candidates — Not Regret-Prone

- **NULL embedding on synthesis rows**: Pre-existing gap, already accepted debt before this plan. Reversing it means a separate plan to embed synthesis rows at insert time — that's an addition, not a reversal of a plan-017 decision.
- **`source_type` CHECK constraint limiting enum evolution**: Low lock-in. Adding a new valid `source_type` value requires a migration, which is the right place to do it anyway. Not a regret.
- **Partial-index approach over cluster-hash-in-content-hash alternative**: Technically sound, the alternative (embedding cluster hash into `content_hash`) was rejected on correctness grounds (two syntheses with the same cluster but different content must coexist). The partial-index approach is the correct one here.
- **Audit subcommand value proposition**: Low value if Options 2/3 never ship, but also zero harm. It's read-only scaffolding, not a commitment.
- **6-step bundle vs split**: Justified by hard invariant coupling (Step 1 migration column is a prerequisite for Step 2 semantic change). The bundle rationale is explicitly documented against "file-touch overlap" as the weak argument.
- **Bundle-all shape**: Follows from the bundle-vs-split decision above. Same reasoning.

## Summary

Nothing in plan 017 is *obviously* regret-prone in the sense of a clearly wrong call. The coalesce heuristic is the one place where a reasonable person could disagree and where a real production scenario (though low-probability given the current 27-row corpus) could trigger a reversal. All other decisions are either low-lock-in, pre-existing, or have strong correctness arguments behind them.
