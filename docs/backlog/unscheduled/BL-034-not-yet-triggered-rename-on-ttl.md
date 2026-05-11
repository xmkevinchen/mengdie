---
id: BL-034
title: audit-stats — rename `not_yet_triggered` to a TTL-safe label before v1
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "audit-table TTL or rotation feature ships (currently out-of-scope per F-005 plan §'Decisions not implemented') OR pre-v1 tag cut, whichever comes first"
related: [F-005]
source: F-005 second-pass review (architecture-reviewer #3)
---

# BL-034: rename `not_yet_triggered` before TTL rotation ships

## What

Rename the `AuditStatus::NotYetTriggered` variant (and the corresponding `not_yet_triggered` JSON enum value emitted by `mengdie audit-stats --format json`) to a label that does not leak the assumption that "zero rows" implies "the hook has never run".

Candidate names (decide at sprint pickup):
- `no_recent_data` — accurate post-TTL, accurate pre-TTL
- `no_audit_rows` — describes the observed state without implying lifecycle
- `unknown` — generic; matches common health-check API conventions

## Why it matters

The current label `not_yet_triggered` reads as a temporal claim: "the hook has not triggered **yet**".  But the actual condition tested in `cmd_audit_stats` is `audit_count == 0 AND audit_write_failures == 0`.  This is correct on a fresh DB (the hook has indeed never run), but a future DB that has been wiped of audit rows by a TTL or rotation feature would emit the same `not_yet_triggered` label even though the hook **has** triggered in the past — just nothing recent enough to leave rows.

A script consumer parsing `not_yet_triggered` as "hook never ran" would be wrong in that scenario.  The field name leaks a DB-lifecycle assumption that v0.0.1 happens to satisfy (no TTL rotation today) but that future versions are explicitly designed to break (the F-005 plan's "Decisions not implemented" section flags audit-table TTL / rotation as out-of-scope but not abandoned).

This is a renaming decision that becomes a breaking-change of the JSON wire contract once external scripts ship against it.  Pre-v1, renaming is cheap.  Post-v1, it requires a deprecation cycle.

## Why deferred

The current label is **not wrong** for v0.0.1.  No TTL rotation exists, so `audit_count == 0` reliably means "hook never ran" today.  Pre-emptively renaming a field that is currently semantically correct adds noise; the rename should be paired with the actual lifecycle-shift that creates the ambiguity.

The trigger is intentionally double-bound: rename before TTL ships (the natural pairing), OR before v1 tag cut (the safety net so a release with the ambiguous label doesn't get blessed as the long-term contract).  Whichever fires first.

## Trigger condition

Move this BL to a sprint when EITHER:

- An audit-table TTL or rotation feature is being designed / scoped (the F-005 plan's `Decisions not implemented` §"Audit-table TTL / rotation" section is the current placeholder; if that gets promoted to a feature, this BL must be picked up alongside or before).
- A v1 tag cut is being planned (final chance to rename before the contract becomes load-bearing).

## Hint at fix shape

The rename touches three places:

1. `AuditStatus::NotYetTriggered` enum variant in `src/bin/cli.rs`.
2. The `&str` arm in the `status_label` match (`AuditStatus::NotYetTriggered => "not_yet_triggered"`).
3. The hint text for the renamed variant ("No audit records yet — either no searches happened, or the hook is broken; check stderr logs.").  At sprint pickup the hint text should be updated to match the new semantic — e.g., for `no_recent_data`: "No audit rows in the current window — either no recent searches, the hook is broken, or all rows have aged out via TTL rotation."

The integration tests `tests/audit_stats.rs::test_status_not_yet_triggered_on_fresh_db` and the test in db.rs that asserts the AC3 row-1 case will need to be renamed to match.  All three changes are in one commit; no production callers parse the enum variant name (only the serialized JSON value does).

The serde `#[serde(rename_all = "snake_case")]` will derive the new lowercased value automatically from the new variant name.
