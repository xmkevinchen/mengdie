---
plan: "013"
reviewer: architect
date: 2026-04-20
verdict: approved-with-notes
---

# Plan 013 Architect Review

## 1. Step Dependency Graph

```
Step 1: decay.rs primitive (no deps)
  └─► Step 2: demotion pass in dreaming.rs (imports decay::*)
      └─► Step 4: --dry-run-decay CLI flag (calls dreaming::dry_run_decay)
Step 1: decay.rs primitive
  └─► Step 3: search.rs re-rank (imports decay::decay_factor, adds MemoryEntry helper)
Step 2 + Step 3: both behavioral
  └─► Step 5: e2e smoke test (exercises both demotion + search re-rank)
```

**Parallel-safe pairs**: Steps 2 and 3 can be worked in parallel once Step 1 merges.  
Step 4 can merge in parallel with Step 3 (Step 4 only depends on Step 2).  
Step 5 must come last.

**Serial chain**: 1 → {2, 3} → {4} → 5

## 2. Missing Steps / Hidden Dependencies

### Must Fix — `last_recalled_as_datetime()` placement

Step 3 specifies adding `MemoryEntry::last_recalled_as_datetime()` to `db.rs`. Step 2 also
needs datetime parsing (it calls `decay::effective_relevance(avg_relevance, last_recalled, now)`
and `last_recalled` is `Option<String>` in `MemoryEntry`). Step 2 could inline the parse, but
that creates a second parse site — exactly the drift the plan warns against. **The plan sequences
Step 3 after Step 2, which means Step 2 must either (a) inline its own parse or (b) duplicate
the helper before Step 3 creates it.**

Fix: move the `last_recalled_as_datetime()` helper creation into Step 1 or the beginning of
Step 2. Both steps can then call it. The current ordering puts the helper in Step 3, which
post-dates Step 2's code.

### Consider — `dry_run_decay` function in Step 4

Step 4 introduces `dreaming::dry_run_decay(&db, project, now)` as a new function. This is a
parallel code path to the demotion scan in Step 2. If the scan logic drifts between the two
sites (e.g., the SQL predicate evolves), the dry-run result will lie. Consider: implement the
dry-run as a parameter on the existing demotion pass (`write: bool`) rather than a separate
function, so there is one scan-and-count loop with a conditional UPDATE. Keeps the AC5
behavior identical (returns same `DreamingResult` with `demoted=0` when `write=false`).

### Consider — Step 2 caller count

Plan says "3 non-test callers: `run_dreaming` wrapper (`dreaming.rs:52`), `cli.rs:215`,
`tests/e2e.rs:92`". The `tests/e2e.rs:92` call is `run_dreaming` (the wrapper), not
`run_dreaming_with_config` directly. The signature change is on `run_dreaming_with_config`;
the wrapper `run_dreaming` calls it and will need updating too, but `e2e.rs` calls
`run_dreaming`, which is only indirectly affected. Minor — the plan should just note
`run_dreaming` wrapper needs to forward `None` for `now`, which is trivially correct.

### Approved — `now` injection scope

Step 3 deliberately does NOT inject clock into search (uses `Utc::now()` directly). This
is consistent with the conclusion's wording: "no API change to search — search is per-call,
not per-pass." No issue.

## 3. AC Coverage

| Step | ACs covered | Notes |
|------|-------------|-------|
| Step 1 | AC1 | 8 decay cases + 2 effective_relevance boundary cases — exact match |
| Step 2 | AC2, AC4 | Clock injection + demotion semantics + 4-counter contract |
| Step 3 | AC3 | Search re-rank ranking test |
| Step 4 | AC5, AC6 | CLI output + dry-run + operator doc |
| Step 5 | AC6, AC7 | Doc existence (AC6 covered in Step 4 as well) + production smoke |

Every AC is covered. No AC is unreachable given the step ordering.

AC6 is split: Step 4 creates `dreaming-decay.md`, Step 5 checks it exists and updates
backlog + CHANGELOG. This is fine — creation in 4, verification in 5.

## 4. Scope Budget

| Step | Estimated LOC (new/changed) |
|------|-----------------------------|
| Step 1 | ~60 (decay.rs: 15 impl + 45 tests) |
| Step 2 | ~80 (dreaming.rs: ~50 logic + 30 tests; e2e.rs + cli.rs: ~5 trivial edits) |
| Step 3 | ~30 (search.rs: ~15 re-rank replacement + helper; db.rs: ~10 helper impl) |
| Step 4 | ~50 (cli.rs: ~25 flag + output; dreaming.rs: ~15 dry_run path; ops doc: not Rust LOC) |
| Step 5 | ~40 (e2e.rs: seeded 41-memory test; verify-decay.sh: ~5; backlog/CHANGELOG: docs) |
| **Total** | **~260 LOC** |

**Flag**: This exceeds the conclusion's ~50–100 LOC estimate. The overage is almost entirely
in tests (AC1 has 10 test cases; AC2/AC4 add integration tests; Step 5 adds a 41-memory
fixture). The implementation itself is closer to 90–100 LOC. The test suite is ~160 LOC.
This is the right tradeoff given the conclusion's explicit regression-table requirement, but
the budget note should be updated in the plan to reflect ~260 total (impl + tests) vs. the
conclusion's ~50–100 estimate which covered impl only.

## 5. Ordering Risk

**Merging in order 1 → 2 → 3 → 4 → 5** is safe provided the `last_recalled_as_datetime()`
issue is resolved (see Must Fix above). If Step 2 merges before Step 3 but uses inline
date parsing, the build is green but there are two parse sites temporarily. That is a
short-lived smell, not a build break.

**If Step 3 merges before Step 2**: search re-rank calls `decay_factor` (from Step 1), which
exists, but the demotion path has not been wired. The build is green; `is_longterm` memories
will be decay-scored in search but never demoted. This is an incomplete-but-valid state for
a PR boundary.

**If Step 4 merges without Step 2**: the `--dry-run-decay` flag calls a function that doesn't
exist yet — build fails. Step 4 is hard-gated on Step 2.

**If Step 5 merges first**: tests reference `DreamingResult::demoted`, `decay_floor_breaches`,
etc., which don't exist yet — compile error. Step 5 is hard-gated on Steps 2 and 3.

## Summary

**Must fix (1)**:
- `last_recalled_as_datetime()` helper is created in Step 3 but consumed in Step 2. Move
  the helper to Step 1 (or the top of Step 2) to avoid requiring Step 2 to inline a duplicate
  parse. The current sequencing is a correctness-risk: the plan says "centralizes the parse
  so Dreaming + search can't drift" but the two steps can be authored independently.

**Consider (2)**:
- Implement dry-run as a `write: bool` parameter on the demotion pass instead of a
  `dry_run_decay` sibling function — eliminates scan-logic drift risk between the two paths.
- Clarify that `e2e.rs:92` calls `run_dreaming` (not `run_dreaming_with_config`); the wrapper
  signature is unchanged, only its implementation needs a `None` forward.

**Approved otherwise**: dependency graph is sound, AC coverage is complete, production smoke
(Step 5) correctly gates on Steps 2 + 3, rollout order is correct.
