---
id: BL-014
title: "mengdie audit-stats / doctor subcommand for audit pipeline observability (v0.0.1.x patch)"
status: open
created: 2026-04-28
origin: "discussion 029 doodlestein-adversarial-post Finding 3"
trigger: "F-002 ships in v0.0.1 AND A-MEM trigger remains deferred — the audit pipeline runs in production with no operator-visible read path. BL-014 closes the discoverability gap before A-MEM lands."
depends_on: [F-002]
size: S
v_target: "v0.0.1.x patch window (before A-MEM lands)"
---

# BL-014 — mengdie audit-stats / doctor subcommand

## Origin

Surfaced by discussion 029 doodlestein-adversarial-post Finding 3, which
challenged the conclusion's pre-decision "no v0.0.1 read path".

## Problem (from adversarial finding 3)

The 029 conclusion deferred any v0.0.1 read path on the audit table — the A-MEM
trigger IS the read consumer, and mengdie's CI gets a Rust integration test
that asserts the supersession SQL runs correctly against a seeded schema
(per strategic-post Finding 1).

But the operator's production DB at `~/.mengdie/db.sqlite` is never queried
by any v0.0.1 tooling. If the audit hook is silently broken (wrong call site
after a merge, early-return on embedding error before the hook fires, schema
v6 migration somehow ran but the hook didn't), `memory_search_audit`
accumulates zero rows in production. A-MEM's deferred trigger requires
≥5 searches in a 30-day window — if the table is empty, A-MEM never fires
and the operator has no signal whether the audit system is broken vs. simply
not yet triggered.

The 029 conclusion's "best-effort + warn" rejection rationale for hard-error
("infrastructure failures the operator can't recover from") doesn't apply to
audit pipeline breakage — that IS recoverable, but the operator needs a way
to discover it.

## What this BL ships

A minimal CLI subcommand: `mengdie audit-stats` (or `mengdie doctor` with
multiple checks). At minimum, output:

```
Audit pipeline status:
  audit_count: <N>          # SELECT COUNT(*) FROM memory_search_audit
  link_count:  <M>          # SELECT COUNT(*) FROM audit_returned_facts
  oldest_row:  <timestamp>  # MIN(searched_at)
  newest_row:  <timestamp>  # MAX(searched_at)
  supersession_count_30d: <K>  # the F-002 supersession query (≥5/30d)
  metric_audit_write_failures_session: <F>  # in-process counter (caveat: ephemeral)
```

This is a small read-only operator-debug primitive. Does NOT replace the
deferred richer read path (A-MEM dashboard, structured JSON output for AI
agents); does NOT add v0.0.1 acceptance criteria beyond what F-002 already
ships.

## Why deferred (not in F-002 itself)

The 029 conclusion explicitly carved out v0.0.1 ship scope to be the audit
write path + acceptance test, not operator-visible read tooling. BL-014 sits
in the v0.0.1.x patch window — after F-002 lands, before A-MEM is wired —
to close the discoverability gap as a small operational improvement, not to
expand v0.0.1's commitment.

## Trigger

F-002 ships AND audit table exists in production. The "before A-MEM lands"
window is the priority — if A-MEM lands first and works, this BL is no
longer urgent (A-MEM trigger firing IS observability).

## Notes

- Should be a single subcommand parallel to `mengdie stats` (which already
  reads the metrics table). Same UX shape.
- Do NOT add new MCP tool(s) — this is operator-CLI-only per the 029
  pre-discussion decision that AI agents don't read the audit table.
- Persistence of `METRIC_AUDIT_WRITE_FAILURES` across process restarts
  (raised in regret-post + adversarial-post Finding 1) is a related but
  separate concern; can be addressed here OR in F-002 as a TODO. If F-002
  plan adds "warn log includes query + timestamp" per adversarial-post
  Finding 1, the post-restart recovery path exists in stderr logs even
  without persisting the counter.
