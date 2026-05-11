---
id: BL-035
title: cmd_list — back-port `--format` from `String` to `clap::ValueEnum` (consistency with cmd_audit_stats)
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "next touch of `cmd_list` (any reason — feature add, bug fix, refactor) OR next addition of a third `--format`-bearing subcommand (the third format flag is the cost-tipping point)"
related: [F-005]
source: F-005 second-pass review (architecture-reviewer #4 + challenger #B, independently flagged)
---

# BL-035: back-port `cmd_list --format` to typed `clap::ValueEnum`

## What

Convert `cmd_list`'s `--format` flag from `String` (with manual `if format == "json"` branching and silent fallthrough to table for any other value) to the `OutputFormat` enum (`clap::ValueEnum`) introduced by F-005 in `cmd_audit_stats`.  After the change, `mengdie list --format yaml` will be rejected by clap with exit code 2 and a `[possible values: table, json]` message, matching the behavior of `mengdie audit-stats --format yaml` since F-005.

## Why it matters

Two sibling subcommands in the same CLI now have **incompatible behavior** for an identically-named flag:

- `mengdie list --format <anything>` → clap accepts any string; binary silently treats unknown values as `table` (the default branch in `if format == "json" { ... } else { ... }` at `src/bin/cli.rs`).
- `mengdie audit-stats --format <anything>` → clap rejects any value not in `{table, json}` with exit code 2.

The F-005 plan body explicitly cites this as the motivation for using `ValueEnum` ("Codex review #1 rationale: typed enum gives clap-level rejection of invalid values with exit code 2, avoids the silent-fallthrough bug currently present in `cmd_list`'s `--format` handling").  But the fix was applied only to the new command, not back-ported to the existing one.  The result is documented technical debt that the first-pass F-005 review missed: the plan acknowledges the bug, the fix exists in the same file, but `cmd_list` wasn't updated.

Both architecture-reviewer and challenger independently flagged this in F-005 second-pass review (commits `0536cb3..HEAD`):

> [architecture-reviewer #4]: F-005 introduced OutputFormat as a typed clap::ValueEnum enum, explicitly motivated by fixing the "silent fallthrough" bug in cmd_list. However, cmd_list was not updated.
>
> [challenger #B]: cmd_list 已有 --format String (cli.rs:107-108, plain String), cmd_audit_stats 新引入 --format OutputFormat (typed enum). 两个并排的 sibling 子命令用户会发现行为不一致。

## Why deferred

The bug is benign for any operator who passes a valid value (`table` / `json`) — both subcommands behave correctly.  It only manifests when an operator typos the value or expects yaml-style flexibility.  At v0.0.1 single-operator scale, the friction is low.

Back-porting now would also require touching `cmd_list`'s implementation flow (verifying that the existing `format == "json"` branch is the only consumer), which has a non-zero risk of regression for a fix that nobody is currently asking for.  The tipping point is when the cost of consistency-debt (operators get used to inconsistent error behavior) crosses the cost of touching `cmd_list` (regression risk + verification effort).

That tipping point comes naturally on the next reason to touch `cmd_list` for any other purpose, OR when a third `--format`-bearing subcommand lands (because then we'd be making a third copy of the same decision and the cost of three inconsistent format flags exceeds the cost of unifying all three).

## Trigger condition

Move this BL to a sprint when EITHER:

- The next change to `cmd_list` for any other reason (feature, bug fix, refactor) — bundle the format-flag conversion with that touch since the regression-verification cost is amortized.
- A third subcommand needs `--format <table|json>` semantics (would force the choice of repeating the F-005 pattern or unifying all three; unifying is cheaper at three).

## Hint at fix shape

Single-file diff in `src/bin/cli.rs`:

```diff
 List {
     #[arg(long)]
     global: bool,

-    /// Output format: table (default) or json
-    #[arg(long, default_value = "table")]
-    format: String,
+    /// Output format: table (default) or json
+    #[arg(value_enum, long, default_value_t = OutputFormat::Table)]
+    format: OutputFormat,
 },
```

Plus update the dispatch + body of `cmd_list` from `&str` parameter to `OutputFormat`.  The `OutputFormat` enum already exists at `src/bin/cli.rs` top scope (introduced by F-005); no new types needed.

Verification: add an integration test `mengdie list --format yaml` returning exit code 2 with `[possible values: table, json]` in stderr, mirroring `tests/audit_stats.rs::test_invalid_format_rejected_by_clap`.  Also verify the existing valid-path tests (`mengdie list --format table` and `mengdie list --format json`) still pass.
