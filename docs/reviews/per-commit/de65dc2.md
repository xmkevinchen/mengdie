---
commit: de65dc2
committed_at: 2026-05-09T03:21:57Z
plan: .ae/features/active/F-005-mengdie-audit-stats-doctor-subcommand-fo/plan.md
step: 2
status: findings
---
## Track 4 — Per-commit Doodlestein
**Strategic**: The `status` variable is consumed by the `match format` arm that needs it but is also referenced in the `Table` arm's `match status_label` — this works only because `AuditStatus` is not `Copy`-derived; deriving `Copy` and matching on the enum directly in the Table arm (rather than converting to `&str` first and then matching the string) would eliminate the string-intermediate detour, keep the match exhaustive at compile time, and remove the dead-code risk if a variant is ever renamed.
**Adversarial**: (False positive — the abbreviated prompt elided enum-path qualification; the actual source uses `AuditStatus::Ok`/`AuditStatus::NotYetTriggered`/`AuditStatus::Degraded` and compiles cleanly. No action.)
**Regret**: The `status_label: &str` indirection. It avoids repeating the enum match, but creates a stringly-typed detour (match enum → string → match string) that is not exhaustive — if a fourth status variant ships (e.g. `Stale`), the hint block silently falls through to `_ => {}` with no compiler warning. Most likely rework target — either match the enum directly in the Table arm, or implement `Display` on `AuditStatus`.
