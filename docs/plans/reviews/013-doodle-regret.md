---
plan: "013"
type: review
subtype: doodle-regret
reviewer: Doodlestein
date: 2026-04-20
---

# Doodle Regret — Plan 013

## Decision under scrutiny

**`scripts/verify-decay.sh` approval-gate design** (Step 5 / AC7): the script runs `--decay-dry-run`, prints would-demote memory IDs, and requires `--i-reviewed-each` confirmation before allowing a live run.

## Why this is most likely to be reversed

The gate assumes the operator is a human who reads output and re-invokes the script manually. In six months, `mengdie dream` will almost certainly run as a launchd cron job (the `com.mengdie.dream.plist` template is already in `resources/`). A daemon can't supply `--i-reviewed-each`.

When that happens, the shell-script approval gate becomes either:
- a permanent blocker (daemon silently skips the live pass), or
- a workaround target (`--i-reviewed-each` gets baked into the cron invocation unconditionally, defeating the gate entirely).

The structured-JSON line (`decay_floor_breaches`) is the durable signal — it's already machine-parseable. The gate logic (threshold check + optional alert) belongs in the daemon config or a monitoring hook, not a flag-gated shell script.

## What reversal looks like

Drop `--i-reviewed-each` from the script; replace the gate with a threshold comparison (`breach_count > N → emit warning to stderr / pagerduty / log, then proceed`). The `--decay-dry-run` flag and the structured-JSON output survive intact — only the human-loop checkpoint is removed.

## Confidence

High. The launchd template's existence makes daemon automation a near-certain Phase 2.x step, not a hypothetical.
