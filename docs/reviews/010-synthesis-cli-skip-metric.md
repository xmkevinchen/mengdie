---
id: "010"
title: "Review: plan 012 — BL-synthesis-cli-skip-metric"
type: review
created: 2026-04-19
target: "docs/plans/012-synthesis-cli-skip-metric.md"
verdict: pass
---

# Review: plan 012 (BL-synthesis-cli-skip-metric)

## Verdict: PASS

Ship plan 012. 3 P2s fixed inline in fixup commit `68752ab`. 1 challenger
claim backlogged as `BL-synthesis-preload-db-miss-edge` with trigger. No
P1s. 186 tests pass (up from 185 — discrimination test added in main
commit; integration-coverage assertion added in fixup).

## Review Team

- **code-reviewer** (Claude) — correctness, test quality, API impact, format string
- **challenger** (Claude, pure opposition) — 4 structured claims, no synthesis
- **codex-proxy** (Codex, cross-family) — Rust idiomatic review
- **gemini-proxy** (Gemini, cross-family) — **unavailable** (API key invalid); no angle fallback (observability lens partially covered by challenger)

**Cross-family status**: Codex operational this round (first successful
Codex proxy call since plan 007 — prior sessions had both proxies
account-limited / key-invalid). Gemini remained unavailable. Retrospect 003
Insight #4 ("cross-family value-prop empirically unvalidated") gets a
fresh data point: Codex's idiomatic review endorsed all 4 questions with
minor optional suggestions, no unique findings vs. Claude-only reviewers.

## Scope

`git diff b6acb8d..HEAD` on `main`. Commits:
- `63d83b0` Step 1: add pair_clusters_processed + pair_clusters_skipped on
  SynthesisResult; remove tuple return; CLI numerator fix; new discrimination
  test
- `3e45528` plan checkboxes + step summary
- `68752ab` this review's fixups

## Prior Art from Project Knowledge Base

- `[plan]: display-layer counters belong on SynthesisResult, not a wrapper struct` (plans/012, 2026-04-19) — this plan's own decisional basis.
- `[plan]: Pair-cluster skip percentage denominator must count pre-DB-load` (plans/011, 2026-04-18) — the attribution invariant this plan extends.
- `[discuss]: Doodlestein "skip rate > 25% = revisit" trigger must include CLI output visibility` (discussions/018, 2026-04-18) — the original observability requirement.

## Synthesized Findings

### P2 — Fixed inline (fixup commit `68752ab`)

| # | Finding | Sources | Fix |
|---|---|---|---|
| 1 | `run_synthesis_pass` docstring didn't state the pre-DB-load attribution invariant explicitly. Future maintainer could move either counter post-load without realizing it breaks pair-cluster skip %. | code-reviewer | Extended docstring to name the invariant + point to `BL-synthesis-preload-db-miss-edge` for the remaining theoretical asymmetry. |
| 2 | Duplicate `trimmed_ids.len() == 2` check at two increment sites — drift risk if the size rule ever changes. | codex-proxy (optional) | Extracted `let is_pair_cluster = trimmed_ids.len() == 2;` locally; reused at both sites. Single source of truth. |
| 3 | E2E test `end_to_end_dream_synthesis_writes_one_row_with_six_links` didn't exercise the new `pair_*` fields — public API fields missing integration coverage. | challenger D | Added `assert_eq!(result.pair_clusters_processed, 0)` + `assert_eq!(result.pair_clusters_skipped, 0)` on the 6-memory fixture. Locks field semantics into integration coverage. |

### P3 — Backlogged with trigger

- `BL-synthesis-preload-db-miss-edge.md` — challenger claim C: if DB load
  returns fewer memories than expected on a pair cluster, the denominator
  is incremented (pre-load) but the numerator never is (post-load guard
  bails). Inflates displayed pair-cluster skip % by one bin per DB miss.
  Theoretical (requires data corruption or concurrent delete); not observed.
  Trigger: first observed arithmetic mismatch between counters, OR landing
  of a concurrent-delete path. Fix direction documented (Option B:
  compensating decrement on bail-out).

### Dismissed (non-actionable)

- **challenger A**: claimed existing `test_synthesis_pair_skip_percentage_computed_against_pairs` was "blind to the numerator bug" and calling both tests "companion" overstates value. Dismissed as retrospective labeling — the existing test validates denominator discrimination which was never the bug, but it still provides a regression guard for that contract. The new test genuinely is the numerator discriminator. No action on labeling.
- **challenger B**: claimed the `100% → 27%` delta couldn't be cleanly attributed to the code fix due to corpus drift (224 → 237 memories between runs). Dismissed: the fix is mechanical (3-line numerator change in `cli.rs`) and verifiable by code inspection; the smoke test is confirmatory, not load-bearing. The "27%" displayed value matches the audit figure from the pre-fix log recount (9 total skips minus 6 non-pair skips = 3 pair-skips → 3/11).

### Acknowledged tradeoffs (not acted on)

- **code-reviewer P2b (operator mental model)**: current format string shows `"{S_total} LLM-skipped ({S_pair}/{P_pair} pair-clusters = X%)"` — two different scopes on one line. Suggestion was to rephrase (e.g., `"pair-clusters {S_pair}/{P_pair} = X%"`). Format shape has been stable across 3 plans (010, 011, 012); ad-hoc parsers may depend on the current order. Deferred; if a future plan adds operator help text (`mengdie dream --help`), include an explanation there.
- **codex optional (helper method `pair_cluster_skip_pct()`)**: ergonomic helper on SynthesisResult that would prevent manual numerator/denominator pairing at callsites. Only one caller today (CLI); premature until a second reader appears (same shape as the BL-synthesis-result-struct-promotion argument — don't introduce helpers before callers). Noted as "optional nicety" by codex; left for a future plan.

### Disagreement Value Assessment

- **Challenger A vs code-reviewer**: code-reviewer called the two tests "companions"; challenger argued the first was blind to the fix's bug. Both are technically correct. Resolution: the existing test is a regression guard for denominator correctness (a contract that happened to be correct pre-plan-012); the new test is the only discriminator for the numerator fix. Labeling is cosmetic — tests coexist with distinct roles.
- **Challenger C vs code-reviewer (pre-load attribution)**: code-reviewer's "off-by-one" check passed on the happy path; challenger found the DB-miss edge path where the pre-load counter could inflate without matching numerator. Code-reviewer wasn't wrong — the happy-path off-by-one is safe — but the edge exists. Challenger wins on completeness; backlogged rather than fixed because the edge is unobservable in current corpus.

## Outcome Statistics

- **Steps completed**: 1/1
- **Rework rate**: 0 steps needed fixup commits during `/ae:work`. Review-stage fixup is a single commit (`68752ab`).
- **P1 escape rate**: 0
- **Drift events**: 0 — Step 1 `Expected files:` (src/core/dreaming.rs, src/bin/cli.rs, tests/dream_synthesis.rs) matched `git diff --name-only` exactly.
- **Fix loop triggers**: 0
- **Auto-pass rate**: 1/1 — TL-executed directly; tests green, clippy clean, no P1 at review.
- **Cross-family coverage**: partial. Codex operational (first success since plan 007); Gemini unavailable. Codex's review endorsed plan 012 with no unique findings vs. Claude reviewers — more data for the retrospect-003 Insight #4 null hypothesis (cross-family specific value-prop remains empirically unvalidated).
- **Deferred resolution rate**: N/A — no `DEFERRED` entries in `docs/milestones/012/notes.md`.

## Deferred Findings Audit

✅ No DEFERRED entries (notes.md doesn't exist — clean slate from `/ae:work`).

## Backlog Items Filed

- `docs/backlog/BL-synthesis-preload-db-miss-edge.md` — pre-load attribution edge (challenger C)

Backlog at 16 items (was 15 pre-review; +1 new, 0 closed — BL-synthesis-cli-skip-metric and BL-synthesis-result-struct-promotion were both closed at plan creation / plan review time).

## Fixup Commits

- `68752ab` — docstring invariant (P2 code-reviewer) + is_pair_cluster local binding (P2 codex) + e2e pair_* assertions (P2 challenger) + BL-synthesis-preload-db-miss-edge filed (P3 challenger C).

Autosquash/rebase not attempted this session (destructive ops disabled).

## Next Steps

Review passed. Suggested next actions:

1. **`git push origin main`** — 4 unpushed commits (plan meta + step + plan done + fixups).
2. **`/ae:retrospect --compare 002 003`** — now that a 4th review cycle completes post-plan-012, the retrospect trend line has one more data point; comparison would quantify delta since retrospect 003.
3. **`/ae:roadmap`** — backlog at 16 items; still a fine time to group.
