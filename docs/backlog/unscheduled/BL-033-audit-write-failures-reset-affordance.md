---
id: BL-033
title: audit-stats — operator affordance to clear audit_write_failures counter
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "operator hits the first real fix-and-reset cycle (sees `degraded` after fixing the underlying audit-write hook bug; needs to clear the cumulative counter to return to `ok`)"
related: [F-005, F-002]
source: F-005 feature-completion review (challenger #6 finding)
---

# BL-033: `mengdie audit-stats` — affordance to clear cumulative `audit_write_failures`

## What

Add an operator-facing way to clear the `audit_write_failures` counter (row in the `metrics` table, key `audit_write_failures`) from the CLI, so an operator who has fixed an audit-write-hook bug can return the `status:` field from `degraded` back to `ok` without directly editing the SQLite DB.

Most likely shape: a `--reset-failures` flag on `mengdie audit-stats`, OR a sibling subcommand `mengdie audit-stats-reset` (TBD at sprint time — defer the bikeshed). Either form executes:

```sql
DELETE FROM metrics WHERE key = 'audit_write_failures';
```

## Why it matters

The `audit_write_failures` counter is monotonic and has no acknowledgment / clear path in F-005's shipped surface. When an operator:

1. Sees `mengdie audit-stats` report `status: degraded` with `audit_write_failures: 3`.
2. Investigates, finds and fixes the underlying audit-write hook regression.
3. Re-runs `mengdie audit-stats` to verify health.

…they will continue to see `status: degraded` indefinitely. The script-facing JSON consumer will continue to see `"status": "degraded"` and may continue alerting. The only way to return to `ok` is to manually open the SQLite file and `DELETE FROM metrics WHERE key = 'audit_write_failures'` — which is exactly the kind of friction `mengdie audit-stats` was built to eliminate.

The F-005 review-cycle (challenger #6) flagged this as a known operator UX antipattern: monotonic failure counters with no acknowledgment path will surface on the first real breakage + fix cycle.

**As of commit 6922579** (third-pass review fix), the `degraded` table-format hint text describes the state without prescribing the reset mechanism:

> The counter is cumulative — `status: degraded` will persist across runs until the failure counter is reset.

That documentation is the v0.0.1 mitigation.  When this BL ships and the actual reset CLI is decided (`--reset-failures` flag, sibling `metrics-reset` subcommand, etc), the hint should be updated at that time to point at the chosen command — but the hint deliberately does not pre-commit to a specific shape today.

**Hint text history**:
- commit `4b9ac46` (first-pass review fixup): added the warning + embedded the internal tracking IDs `F-005 challenger #6 / BL-033`.
- commit `0536cb3` (second-pass review fixup): stripped the internal tracking IDs but preserved the prescriptive "manually clear the metrics-table row" wording.
- commit `6922579` (third-pass review fixup): dropped the prescriptive wording entirely after Codex 2nd-pass / Gemma 3rd-pass / challenger 3rd-pass triple-flagged that it pre-commits to a manual-fix shape that this BL's two design alternatives (`--reset-failures` flag, sibling `metrics-reset` subcommand) both replace.

## Why deferred

The F-005 plan was scoped to "narrow operator-debug subcommand" — ship the read-only health view first, then iterate. Adding a write affordance (DELETE on metrics) widens the threat surface (`audit-stats` is no longer purely read-only) and deserves its own design pass: should the reset be guarded by a `--yes` confirmation? Should it require the prior value to be specified (`--reset-from 3`) for idempotency? Should there be an audit log of resets themselves? None of these questions need answering for v0.0.1 because no operator has yet hit the first breakage cycle.

## Trigger condition

Move this BL to a sprint when EITHER:

- An operator reports a real fix-and-reset cycle in production (sees stale `degraded` after fixing the underlying issue) and asks for the affordance.
- The audit-write hook ships its first regression-fix cycle (would be the first real test of this UX gap).
- A second metric counter is added with the same monotonic shape (e.g., `ingest_write_failures`) and the cumulative-counter UX gap surfaces on multiple fields at once — at which point a generic `mengdie metrics-reset --key <name>` design becomes more economical than per-counter affordances.

## Hint at fix shape

Most-conservative version: a sibling subcommand `mengdie metrics-reset --key audit_write_failures` that requires explicit key naming (no `--all`, no implicit reset). This generalizes naturally if other monotonic counters appear later.

Less-conservative version: `mengdie audit-stats --reset-failures --yes` that executes the DELETE then re-runs the read pass and prints the post-reset state.

The actual choice belongs to the sprint that picks this BL up — both shapes are valid, the design-pass should consider operator-confirmation discipline alongside the narrower question of which subcommand owns the affordance.
