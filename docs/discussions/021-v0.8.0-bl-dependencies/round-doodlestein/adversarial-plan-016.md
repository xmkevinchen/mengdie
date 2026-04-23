---
author: doodlestein-adversarial
plan: "016"
date: 2026-04-23
---

# Adversarial Review — Plan 016: First-Failure Point

## Verdict

**One clear first-failure point exists**: Step 4's AC4 test uses a wrong field name when cross-checking against the decay pass's real computation. The plan (line 83) says:

> assert `avg_effective_before` was computed over exactly 1 row

But the actual field on `DreamingResult` is `avg_effective_score_before` (see `src/core/dreaming.rs:54`). The plan consistently uses the shortened name `avg_effective_before` in the AC4 description — this is not the Rust struct field name.

A coding agent writing `result.avg_effective_before` will get a compile error: `no field avg_effective_before on type DreamingResult`. The agent will stall or invent a workaround.

This matters because Step 4 is the only non-docs deliverable and the only CI gate. If it fails to compile, CI cannot pass, and the plan cannot close.

## Supporting Evidence

- `src/core/dreaming.rs:54`: `pub avg_effective_score_before: f64`
- `src/core/dreaming.rs:57`: `pub avg_effective_score_after: f64`
- Plan 016 line 83: "returns `avg_effective_before` computed over exactly 1 row"
- Plan 016 line 126: "Test cross-checks against the decay pass's real computation: `run_dreaming_with_config` on the same fixture returns `avg_effective_before` computed over exactly 1 row"
- `tests/decay_contract.rs` (the reference test the plan cites) uses `result.avg_effective_score_before` and `result.avg_effective_score_after` throughout — the agent CAN find the correct names by reading that file, but only if it chooses to look.

## Severity

**Step 4 fails at compile time, not runtime.** The error is deterministic and not subtle — the compiler names the missing field explicitly. An agent reading the compiler error will find the correct name in `dreaming.rs` within one lookup. Recovery is a one-word fix.

However, the mismatch in the plan text is a reliable stumbling block: the agent's first draft will use the plan's wording verbatim (`avg_effective_before`), fail to compile, and require a second pass to find the real field name. For a pure-docs plan where Step 4 is the only Rust code, this is the first and only hard failure point.

## Secondary Issue (Not a First-Failure)

The plan states the AC4 cross-check asserts "`avg_effective_before` was computed over exactly 1 row." That is not a direct assertion — `DreamingResult` does not expose a `counted_before` field (the local variable `counted_before` at `dreaming.rs:189` is not returned). The agent would have to infer the 1-row constraint indirectly: if the DB has exactly 1 decay-eligible row and `avg_effective_score_before > 0.0`, that implies count >= 1; if the NULL-last_recalled and invalidated rows are excluded correctly, count == 1 follows from the filter.

This is ambiguous enough to cause a second stall (after fixing the field name), but it is not a compile-time failure. An agent can write a plausible assertion and move on; the CI will pass even if the cross-check is weaker than the plan implies.

## Fix (Minimal)

Replace `avg_effective_before` with `avg_effective_score_before` (and `avg_effective_after` with `avg_effective_score_after` wherever it appears) in Step 4 and AC4. No other change needed.
