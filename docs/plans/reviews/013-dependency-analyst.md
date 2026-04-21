---
plan: "013"
reviewer: dependency-analyst
date: 2026-04-20
---

# Plan 013 — Dependency Analysis

## 1. Shared-state collision: Step 2 vs Step 4 (`dreaming.rs`)

**Finding: Sequential — Step 4 MUST follow Step 2.**

Both steps mutate `dreaming.rs`, but the collision is deeper than file-level:

- Step 2 adds the `now: Option<DateTime<Utc>>` parameter to `run_dreaming_with_config` and extends `DreamingResult` with 4 new fields (`demoted`, `avg_effective_score_before`, `avg_effective_score_after`, `decay_floor_breaches`).
- Step 4 adds `dry_run_decay(&db, project, now)` as a new function that calls the demotion scan WITHOUT the final UPDATE, and returns a `DreamingResult` — which means it depends on the 4 new fields introduced in Step 2.
- Step 4's CLI code also reads `DreamingResult.demoted` for the output format, again depending on Step 2's struct extension.

Parallelizing these would require one implementer to define the struct contract and the other to consume it simultaneously — a classic write-then-read hazard within the same file on the same struct. **Step 4 cannot compile until Step 2 is merged.**

## 2. Signature change propagation: are 3 callers sufficient?

**Finding: Yes — 3 callers listed are exhaustive. No hidden callers.**

Grepped all callers of `run_dreaming_with_config` and `run_dreaming` across `src/` and `tests/`:

| File | Line | Call | Status |
|------|------|------|--------|
| `src/core/dreaming.rs:52` | `run_dreaming` wrapper | Calls `self.run_dreaming_with_config(project_id, &DreamingConfig::default())` | Listed — passes `None` for new `now` param |
| `src/bin/cli.rs:215` | `cmd_dream` | `db.run_dreaming_with_config(None, &config)?` | Listed — passes `None` |
| `tests/e2e.rs:92` | `test_full_pipeline` | `db.run_dreaming(None)` | Listed as indirect — goes through the `run_dreaming` wrapper, which will itself pass `None` once updated |

`src/bin/mcp_server.rs` has **no calls** to `run_dreaming` or `run_dreaming_with_config`.

The `dreaming.rs` internal unit tests (lines 427, 442, 459, 473, 477, 491) all use `run_dreaming(None)` — the wrapper — so they are covered by the wrapper update.

**One clarification needed**: the plan says "3 non-test callers: `run_dreaming` wrapper (`dreaming.rs:52`), `cli.rs:215`, `tests/e2e.rs:92`" but then lists `tests/e2e.rs:92` as a caller. That test calls `run_dreaming(None)` (not `run_dreaming_with_config`), so it is covered by the wrapper passthrough and does not need an independent edit. The plan should note this distinction to avoid confusion, but the outcome (pass `None`) is correct.

## 3. Test dependency: parallel feasibility of Steps 1 and 3

**Finding: Steps 1 and 3 CAN run in parallel. Steps 2 + 4 must be sequential.**

Dependency graph:

```
Step 1 (decay.rs — new file, no deps)
  └─ depended on by Steps 2, 3, 4

Step 3 (search.rs + db.rs — adds last_recalled_as_datetime + decay re-rank)
  └─ depends on Step 1 (calls decay_factor + days_since from decay.rs)
  └─ no dependency on Step 2

Step 2 (dreaming.rs — signature + DreamingResult + demotion pass)
  └─ depends on Step 1 (calls decay::effective_relevance, decay::should_demote)
  └─ no dependency on Step 3

Step 4 (dreaming.rs — dry_run_decay fn + CLI flag)
  └─ depends on Step 1 (via Step 2's demotion logic, shared decay primitives)
  └─ depends on Step 2 (reads DreamingResult new fields; adds dry_run_decay which is the scan-without-UPDATE twin of Step 2's demotion pass)
  └─ no dependency on Step 3

Step 5 (e2e test — requires all functions from Steps 1–4)
  └─ depends on all of Steps 1–4
```

**Confirmed**: Steps 1 and 3 are independent and can proceed in parallel (they touch disjoint files and only share Step 1 as a prerequisite — but since Step 1 is required by both, they can be parallelized after Step 1 is merged, or written in parallel if Step 1 is merged first). Steps 2 → 4 must be sequential.

## 4. Build invariant: intermediate step failures

**Finding: Steps 1 and 3 are safe standalone. Step 2 is safe standalone. Step 4 requires Step 2. Step 5 requires all.**

| After merging | `cargo build` | `cargo test` |
|---------------|--------------|--------------|
| Step 1 only | Passes (new module, pure math, no consumers yet) | Passes (new unit tests in decay.rs) |
| Step 1 + Step 3 | Passes (search.rs imports decay module; db.rs adds helper) | Passes |
| Step 1 + Step 2 | Passes (dreaming.rs compiles with new signature + new fields) | Passes (existing callers break only if not updated — but Step 2 requires updating all 3 callers as part of the same step) |
| Step 1 + Step 2 + Step 3 | Passes | Passes |
| Steps 1–4 | Passes | Passes |
| Steps 1–5 | Passes | Passes (new e2e test added) |

**One risk**: if Step 2 is merged without updating all 3 callers in the same commit, `cargo build` fails with a type-mismatch on `run_dreaming_with_config` (wrong arity). The plan groups all 3 caller edits into Step 2 — this must be done atomically (single commit or PR). As long as that constraint is respected, no intermediate breakage.

**No intermediate state breaks the build** provided each step is committed with all its listed file changes.

## Summary

| Question | Answer |
|----------|--------|
| Steps 2 + 4 parallel? | No — Step 4 depends on Step 2's `DreamingResult` extension |
| Steps 1 + 3 parallel? | Yes — disjoint file sets, share only Step 1 as prerequisite |
| 3 callers sufficient? | Yes — exhaustive; no hidden callers in mcp_server or elsewhere |
| Any intermediate build break? | Only if Step 2's caller edits are split across commits; plan correctly groups them |
| Step 5 dependencies? | Requires Steps 1–4 all merged |
