---
id: BL-017
title: "`mengdie stats` should surface audit_write_failures counter"
status: open
created: 2026-04-29
origin: F-002 accumulated Doodlestein checkpoint (post-Step 4 commit c2544ea)
trigger: "Operator runs `mengdie stats` during AC3 manual verification (or as part of routine ops) and the audit_write_failures counter is invisible. AC3 fallback path (`direct metrics table read`) works but is undiscoverable to operators who don't know the counter exists."
---

# BL-017 — Surface audit_write_failures in `mengdie stats` output

## What

`src/bin/cli.rs::cmd_stats` (around line 686) reads four metrics from
the `metrics` table and prints two derived rates:

```rust
let search_count = get("search_count");
let search_nonempty = get("search_nonempty_count");
let ingest_count = get("ingest_count");
let conflict_count = get("conflict_count");
```

`METRIC_AUDIT_WRITE_FAILURES` (introduced by F-002 Step 2) is not read
or printed. F-002 plan AC3 manual-verification step 4(c) cites
"`mengdie stats` (or direct `metrics` table read) shows
`audit_write_failures` incremented" — the OR fallback works, but
operators running `mengdie stats` to gauge mengdie health won't see
the new counter at all.

## Why it matters

Plan F-002 best-effort wrapper contract: failures stay silent (warn
on stderr + counter increment). The counter is the durable signal —
stderr is ephemeral. Without `mengdie stats` surfacing the counter,
operators have no easy "audit pipeline health at a glance" view.

The fix is small (5-10 lines):

```rust
let audit_failures = get("audit_write_failures");
if search_count > 0 {
    let failure_rate = (audit_failures as f64 / search_count as f64) * 100.0;
    println!("  Audit-write failure rate: {failure_rate:.2}% ({audit_failures}/{search_count} searches)");
} else if audit_failures > 0 {
    println!("  Audit-write failures: {audit_failures} (no searches yet — degraded path?)");
}
```

## Why not now (F-002 scope)

F-002 Step 4's expected files were `src/core/schema.rs` and
`src/core/db.rs`. Adding to `cli.rs:686` cmd_stats is outside Step 4's
expected files. The accumulated Doodlestein checkpoint surfaced this
gap; filing as a backlog item with trigger keeps Step 4's commit
surgical and lets the user decide whether to fold the fix into the
F-002 review commit, the next BL-014 audit-stats CLI plan, or a
standalone bookkeeping plan.

## Implementation sketch (when triggered)

1. Read `METRIC_AUDIT_WRITE_FAILURES` from `metrics::*` (already pub).
2. Add to the `get()` block in `cmd_stats`.
3. Print as a percentage rate (failure / total searches) or as an
   absolute count when search_count is 0.
4. Optional: prefix with a colored warning when failure_rate > 0.

## Reviewer note

Surfaced by accumulated Doodlestein checkpoint at F-002 Step 4 close
(commit c2544ea). The accumulated review captured this as the second
half of its "regret" finding; the first half (Wave 2 deferral of
caller-local hook placement) was already documented in plan R5 and
waived.

## Related

- F-002 plan AC3 manual-verification step 4(c) — current OR fallback
  path that BL-017 would tighten.
- BL-014 (`mengdie audit-stats` CLI subcommand) — separate, larger
  feature for the supersession query. BL-017 is a 1-output-line
  addition to existing `mengdie stats`; BL-014 is a new subcommand.
  These do not overlap.
