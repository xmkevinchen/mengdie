---
id: "018"
title: "Residuals reduction — dream synthesis parameter tuning"
status: done
created: 2026-04-18
pipeline:
  analyze: done
  discuss: done
  plan: done
  work: pending
plan: "docs/plans/011-residuals-reduction.md"
tags: [clustering, residuals, dream-synthesis, parameter-tuning, bl-residuals-reduction]
---

# Residuals reduction — dream synthesis parameter tuning

## Problem Statement

First real `mengdie dream --synthesize` (2026-04-18, commit `4cb86b5`)
produced 67% residuals (133 of 198 memories didn't cluster). Above the
"> 50% = signal" threshold set by
[BL-clustering-validation](../../backlog/BL-clustering-validation.md).
Parameter-sweep analysis (`analysis.md`) surfaced 3-4 viable
remediation options with different cost/quality tradeoffs. This
discussion picks one.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Parameter strategy (threshold / min_size tuning) | [topic-01-parameter-strategy/](topic-01-parameter-strategy/) | converged | Option 2 (min_size 3→2) bundled with null-escape-hatch; ship as 1 plan / 2 steps |
| 2 | Second-pass strategy — scope and value | [topic-02-second-pass-scope/](topic-02-second-pass-scope/) | converged | Second-pass deferred; null-escape PROMOTED to in-scope (bundled with topic 1) |

## Documents
- [Analysis](analysis.md)
- [Conclusion](conclusion.md)
