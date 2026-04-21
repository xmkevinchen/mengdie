---
id: "013-doodle-strategic"
plan: "013"
reviewer: "Doodlestein (strategic)"
verdict: pass
date: 2026-04-20
---

# Strategic Review — Plan 013 (BL-008 Exponential Decay)

## Verdict: Pass

The plan is well-scoped, thorough, and has incorporated prior reviewer feedback cleanly. The single smartest improvement is below.

## One Improvement: `avg_effective_score_after` is undefined in dry-run — make it explicit in the contract

**The gap**: Step 2 specifies that in dry-run mode, `avg_effective_score_after` "equals `_before` (no writes happened)." But the AC4 contract only defines the live-run semantics ("computed across SURVIVING long-term memories post-UPDATE"). There is no AC that validates dry-run's `after == before` invariant. An implementer could return `0.0` for `after` in dry-run (nothing was written, no set to average over) and all existing ACs would still pass.

**Why it matters**: The operator's decision loop — inspect dry-run output, decide whether to run live — depends on the dry-run `after` value being interpretable. If it silently returns `0.0` or `before`, that misleads the operator. The ops doc describes the distinction correctly in prose, but there is no test pinning the behavior.

**The fix**: Add one sentence to AC4 explicitly covering the dry-run contract: *"In dry-run (`write_demotions=false`), `avg_effective_score_after == avg_effective_score_before` (no mutations, surviving set is unchanged)."* Pair it with the existing `write_demotions=false` unit test in Step 2 (which already asserts `demoted == 0` and `decay_floor_breaches == 1`) — add one more assertion: `result.avg_effective_score_after == result.avg_effective_score_before`.

This is a one-line AC addition and a one-line test assertion — zero implementation change. It closes the only ambiguity in an otherwise tight plan.

## No other issues

All other elements — the formula, floor, demotion predicate, clock injection, structured JSON output, approval-gate smoke test, LONGTERM_BOOST cliff documentation, and merge order — are sound. Prior reviewer concerns (NaN clamp, same-age-clock invariant, AE parser audit, flag-conflict guard) are fully addressed. The plan is ready to execute as written once the dry-run `after` contract is pinned.
