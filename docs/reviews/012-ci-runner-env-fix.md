---
id: "012"
title: "Review: Plan 014 — CI Runner Env Fix"
type: review
created: 2026-04-22
target: "docs/plans/014-ci-runner-env-fix.md"
verdict: pass
---

# Review: Plan 014 (CI Runner Env Fix)

## Verdict: PASS

All 5 plan steps delivered with tests green (221 passed, 1 ignored — the
fastembed-dependent test that even with Mock can't help runner hardware)
and CI 4/4 green on the shipping commit (`827a578e`). 3 P2 findings
fixed inline (commit `7401e9c`); 1 P2 filed as BL follow-up; 5 P3s
accepted as-is or recorded for retrospective.

## Team

| Agent | Role | Backend |
|-------|------|---------|
| team-lead | TL (synthesizer) | Claude |
| architecture-reviewer | Trait shape + module boundaries | Claude |
| challenger | Pure opposition (7 claims) | Claude |
| codex-proxy | Technical correctness (Rust + Forgejo Actions) | Codex MCP (reasoning=medium) |
| gemini-proxy | Process + institutional correctness | Gemini MCP (gemini-2.5-flash) |

## Findings

### P1 — None

No correctness defects, data-loss risks, or security issues found.

### P2 — Fixed inline (commit `7401e9c`)

| # | Finding | Sources | Fix |
|---|---|---|---|
| 1 | `MockEmbedder` zero-vector + `test_ingest_file_e2e` name overclaims — zero vec short-circuits cosine_similarity's zero-norm guard, silently disabling contradiction detection + vector search paths under test | challenger #3 (0.85), gemini #4 | Deterministic content-derived non-zero vector; test renamed to `test_ingest_file_pipeline_smoke` |
| 2 | `cross-check` job is a no-op on current Linux runner (host == target); comment overclaimed regression-guard value | challenger #5 (0.7), codex #6 | Comment rewritten to be honest about on-Linux behavior; kept for topology-shift future-proofing |
| 3 | `Embed` trait + inherent method delegation creates two code paths that could silently diverge | architecture #2 | Doc comment on `Embed` trait documenting identity invariant + "mirror or collapse" rule |

### P2 — Deferred to backlog

| # | Finding | Sources | Backlog item |
|---|---|---|---|
| 4 | `release.yml` `needs: [test]` only gates within-file; `ci.yml`'s clippy + cross-check don't block release on tag push (pre-existing cross-workflow gap) | architecture #3 | `.ae/backlog/unscheduled/BL-release-yml-ci-gate.md` |

### P3 — Accepted / retrospective

| # | Finding | Source | Disposition |
|---|---|---|---|
| 5 | `#[ignore]` ghost commit (`a55bc3a`) in history after superseding refactor | gemini #1 | **Accepted** — history already published; rewriting worse than keeping |
| 6 | `.cargo/config.toml` debug heuristic only surfaces when editing that file; not indexed by `rg CFLAGS src/` | challenger #4, gemini #5 | **Accepted** — CLAUDE.md references discussion 020 which has the heuristic; adding grep-friendly pointer in CLAUDE.md is nice-to-have but low priority |
| 7 | Forever-loop temp file `/tmp/mengdie-forever-{pid}-{nanos}.sh` has low-probability collision under parallel tests | challenger #6 (0.55) | **Accepted** — only one callsite; `rand` dependency would be overkill |
| 8 | Plan review missed latent defects (missing `#[ignore]`, `/usr/bin/yes` portability, refactor-reintroducing-the-no-ignore-state) — 3 CI iterations before green | challenger #7 (0.8), gemini #3 | **Retrospective** — process signal: plan review checklist could grep for doc-comment-says-ignored-without-attribute patterns |
| 9 | ~6h for 1-line fix + refactor suggests discuss/plan/work cycle over-instrumented for small changes | gemini #6 | **Retrospective** — meta observation; not actionable per-plan |

### Disagreement Value Assessment

- **Challenger vs Codex on cross-check value**: challenger called it redundant (0.7), codex confirmed on Linux host it's "nearly identical to plain `cargo check`". High-confidence consensus on mechanical effect. Resolution: keep the job, correct the comment — neither "remove as useless" nor "claim as regression guard" was right; the honest middle ground is "cheap insurance for topology shift."
- **Architecture vs Challenger on `Embed` trait**: architecture treated the trait as minor forward-compat risk (P2 worth noting); challenger argued it's test-seam dressed as architecture (P2, 0.8). Both read the same evidence differently. Resolution: keep the trait (removal cost > retention cost under low traffic), add comment codifying the identity invariant architecture flagged.

## Outcome Statistics

- **Steps completed**: 5/5
- **Rework rate**: 3 of 5 steps needed fixup commits during /ae:work (Step 4 had 3 fixups for yes portability, `#[ignore]`, and the Embed refactor that superseded both) — 60%
- **P1 escape rate**: 0 (no P1s found in /ae:review)
- **Drift events**: 1 Step-4 drift approved inline (Step 2 rewrite of ci.yml included bundled release.yml race fix in Step 3 — separate commit, not actual drift)
- **Fix loop triggers**: 0 (max_fix_loops = 3 not reached on any single test file)
- **Auto-pass rate**: Not explicitly tracked; Step 4's CI verification had 4 iterations (3 red → 1 green) which is NOT the auto-pass circuit breaker — those were successive CI commits, not local test-failure loops
- **Deferred resolution rate**: N/A (no `DEFERRED` items in `docs/milestones/014/notes.md`)

## Process Retrospective (separate from verdict)

Two signals worth noting without blocking the verdict:

1. **Plan review missed the `#[ignore]` attribute gap** (challenger #7): the test comment explicitly said "run with --ignored" but the attribute was missing. A `rg` pass at plan-review time could check "every test whose doc-comment says 'ignored' has `#[ignore]`". ~30 seconds of linting would have saved 1 CI iteration.

2. **BL-ci-runner-avx2-sigill was filed mid-plan then superseded within the same plan** (challenger #1): legitimate signal that plan scope expanded beyond what was reviewed. The expansion was in a helpful direction (mock refactor supersedes hardware-centric workarounds), but the process didn't account for this class of evolution. Future plans with "unknown runtime environment issue" as a dependency should budget a scope-expansion contingency.

These go to project knowledge, not to the plan-014 blocker list.

## Knowledge Capture Summary

Applicable patterns to ingest (P2+ reusable):

1. **`Embed` trait identity invariant** — single-method trait + inherent-method delegation must mirror or collapse; diverging would silently break `&mut dyn T` vs `&mut T` callers
2. **Mock-based test decoupling beats hardware-centric CI workarounds** — extracting a trait to mock a heavy dependency (fastembed/ORT here) supersedes "upgrade runner" or "feature-gate dep" paths for test reach
3. **Debugging heuristic: cargo `[env]` beats shell env for Cargo builds** — `.cargo/config.toml` `[env]` propagates through `execve()` boundaries invisibly to `env | sort`; always check there first in cross-compile investigations

## Fixup Commits

- `7401e9c` — Review fixups (Findings 1-3 above)

No squash into original commits — the review fixups are a discrete logical unit (post-ship review cycle) and preserving their identity in history makes the review trail readable. Could be squashed if desired via `git rebase -i 8503f35` later.

## Next Steps

- Plan 014 shipped and reviewed; `status: done`, review `verdict: pass`
- BL-release-yml-ci-gate filed for future attention
- Suggest: `git push origin main` to ship the review fixup
- Consider: `/ae:next` to see the next pipeline step
