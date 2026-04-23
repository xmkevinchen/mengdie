---
author: doodlestein
type: regret
plan: "016"
discussion: "021"
created: 2026-04-23
---

# Regret Analysis — Plan 016, Most Likely Reversed Decision

## Verdict: Step 4 — the `run_dreaming_with_config` cross-check in `tests/ops_doc_sql.rs`

This is the single most regret-prone decision in plan 016.

## Why

**The coupling point is wrong.** Step 4's cross-check asserts that `run_dreaming_with_config` returns `avg_effective_before` computed over exactly 1 row — to confirm the doc SQL denominator matches the code's internal count. This ties a docs-validation test to an internal metric field name and return-shape of a core library function. The test is guarding doc correctness, but its failure mode is internal API churn, not doc drift.

**`run_dreaming_with_config` is an active development surface.** Phase 2 is explicitly in progress — synthesis, clustering, decay tuning are all live. The return type, metric field names, and signature of this function will change as BL-residuals-reduction and related work lands. When `avg_effective_before` is renamed, restructured, or replaced (likely as decay metrics evolve), the cross-check fails for reasons entirely unrelated to the threshold SQL in the doc. The test then becomes a maintenance burden rather than a safety net.

**The plan defensiveness signals contested ground.** Step 4 is described as "triple-confirmed by plan 016 review — challenger C3 + gemini P2.4 + codex P1.2 correction trail." Three-party confirmation for adding a test is unusual; it indicates the test was argued into existence against resistance. That resistance was correct on the coupling concern even if wrong on the need-for-a-test-at-all conclusion.

**The clean alternative is already half-specified.** The structural check (extract SQL between markers, assert three filter conditions) is self-contained and does not couple to internal code. That part should survive indefinitely. The cross-check against `run_dreaming_with_config` is the fragile addition. The reversal is: drop the cross-check, keep the marker extraction + filter substring assertions + 3-row fixture run.

**Contrast with the other four decisions:**

- Unicode-only accepted risk (Decision on action 2): reversal is pre-documented as a "scoped 10-line diff." Low regret — the plan has already done the reversal planning.
- Rollback section placement (Step 2): placement before Metric guide is a stylistic call. If it moves, it's a two-line diff. Low regret.
- JSON→SQL quoting callout (Step 2 item d): correctness detail, not a decision that gets reversed. Low regret.
- "No audit log" honest-limit framing (Step 2 item c): this is a truthful statement about the current code at `src/core/dreaming.rs:251-256`. It gets updated when audit logging is added, not "reversed." Low regret.

## What reversal looks like

Within 6 months: Phase 2 synthesis work or BL-residuals-reduction changes the dreaming return struct — `avg_effective_before` is renamed to `avg_eligible_before` or moved into a nested metrics sub-struct. CI fails on `ops_doc_sql`. The developer realizes the failure is not about the doc SQL being wrong but about an internal metric rename. The cross-check is deleted or reduced to the marker-extraction + filter-assertion portion, which is what it should have been from the start.

## Caveat

The marker-extraction and filter-assertion portions of Step 4 are sound and should survive. Only the `run_dreaming_with_config` cross-check is the fragile half. If the internal API stays stable for 6 months, no reversal occurs — but stability of an active Phase 2 function is the assumption that needs to hold.
