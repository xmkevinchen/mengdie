---
id: "009"
title: "Review: BL-residuals-reduction (min_size=2 + null-escape-hatch)"
type: review
created: 2026-04-18
target: "docs/plans/011-residuals-reduction.md"
verdict: pass
---

# Review: BL-residuals-reduction

## Verdict: PASS

Ship plan 011. 1 P1 (test discrimination gap) fixed inline; 3 P2s fixed
inline (empty-reason warn, N=10 doc disclosure, skip-precedence test); 1
P2 deferred to backlog with trigger (`BL-synthesis-result-struct-promotion.md`).
No remaining P1s. 185 tests pass (up from 184 — new tests added by fixups).

## Review Team

- **code-reviewer** — correctness, test robustness, migration completeness
- **architecture-reviewer** — API shape, module boundary, type-home, AC contract
- **engineering-ai-engineer** (imported) — LLM/prompt domain, parser semantics, telemetry interpretability
- **challenger** — pure opposition, premise interrogation
- **cross-family-fallback** (Claude Sonnet via challenger kit) — Codex + Gemini
  unavailable all session (account-limited / invalid key); fallback per
  CLAUDE.md cross-family protocol. Lens: session-level meta-pattern,
  plan-to-code fidelity, observability, revert rehearsal.

## Scope

`git diff 2c25470^..HEAD` on `main`. Commits:
- `2c25470` Step 1: DEFAULT_MIN_SIZE 3→2 + BL-clustering-validation trigger rewording
- `1601ac5` Step 2: null-escape-hatch (SYSTEM_PROMPT + SynthesisOutcome enum + counter + CLI output + AC5 stub)
- `1ad499a` gitignore cleanup (accidental .DS_Store sweep from Step 2 `git add -A`)
- `<meta>` plan checkboxes + step-summaries + pipeline.work flip
- `c37e152` this review's fixups

184 → 185 tests (pre-fixup); +4 synthesis parser tests + +2 dreaming skip tests + 1 precedence test + 1 pair-denominator discrimination test.

## Prior Art from Project Knowledge Base (Mengdie)

- `[discuss]: Bundle parameter flip with null-escape-hatch when empirical data shows "mixed" ratio` (discussions/018, 2026-04-18) — this plan's decisional basis.
- `[plan]: Pair-cluster skip percentage denominator must count pre-DB-load` (plans/011, 2026-04-18) — architect's Must Fix from plan review, landed correctly.
- `[plan]: expose residuals alongside clusters; don't let greedy min_size silently drop memories` (plans/009, 2026-04-18) — the residuals-not-drop contract this plan leans on.
- `[discuss]: /ae:analyze sweep before /ae:discuss when the question is "which parameter value?"` (discussions/018, 2026-04-18) — the empirical basis.
- `[plan]: First-caller plan validates design bets` (plans/010, 2026-04-18) — general pattern this plan follows.

## Synthesized Findings

### P1 — Fixed inline (fixup commit `c37e152`)

| # | Finding | Sources | Fix |
|---|---|---|---|
| 1 | `test_synthesis_pair_skip_percentage_computed_against_pairs` used all-skip FixedProvider: could not distinguish pair-denominator (2) from total-denominator (4). A buggy impl using total-division would have passed. | code-reviewer, architecture-reviewer, cross-family-fallback (converged) | Replaced fixture with `ClusterSizeAwareProvider` (prompt-count-based): 2 pairs skip, 2 triples synthesize. Test now asserts pair_skip_pct == 100% (2/2) — a buggy /4 division would yield 50%. The CLI arithmetic path is now directly exercised. |

### P1 — Acknowledged, not fixed (plan-level, not code-fixable)

- **architecture-reviewer P1**: AC3 CLI output regex in plan 011 is a fragile contract that breaks on any output format change. Deferred acknowledgement: cargo tests assert on `SynthesisResult` fields directly; the regex is plan-doc prose only. If CLI format changes in a future plan, that plan's review handles it. Not a ship blocker.

### P2 — Fixed inline (same fixup commit)

| # | Finding | Source | Fix |
|---|---|---|---|
| 2 | Empty-reason skip logs had low audit signal (`reason=""` at info level). Prompt-drift masked. | ai-engineer | Warn log with "(unspecified)" substitution + "check prompt adherence" hint when `reason.is_empty()`. Non-empty stays at info. |
| 3 | Doc comment on `DEFAULT_MIN_SIZE` cited "empirical spot-check" without disclosing N=10. Future reader would infer stronger evidence than exists. | challenger | Amended to "N=10 pair-clusters on the author's 214-memory AE corpus (2026-04-18) … Small-sample caveat: re-measure on any new project." |
| 4 | Missing test for skip=true + title/content present (precedence rule). | ai-engineer (P3 elevated during fixup) | New `test_synthesis_skip_precedence_over_title_content`: asserts no "Ignored"-titled row in DB when stub returns skip=true with synthesis fields. |

### P2 — Deferred to backlog (with trigger)

- `BL-synthesis-result-struct-promotion.md` (new): when `(SynthesisResult, usize)` tuple gains a second display-layer counter, promote to `SynthesisPassResult` struct. Premature now per CLAUDE.md "don't design for hypothetical future requirements." Trigger: any plan adding a second return field OR external caller for `pair_clusters_processed`.

### P2 / P3 — Acknowledged, not acted on (tradeoffs or future risks)

- **challenger #1 (bundle 60%→90% reframe unsupported by data)**: real tradeoff; already named in discussion 018 conclusion and mitigated by post-ship audit (AC5).
- **challenger #3 (AC5 stub dead-letter risk)**: acknowledged; the skip-rate counter in CLI stdout provides alternative signal path if the stub is forgotten.
- **challenger #4 (attribution loss from bundling)**: sequential-dependent changes; post-ship audit is the recovery mechanism. Can be independently retested via prompt-revert if needed.
- **challenger #5 (`unwrap_synthesized` test brittleness)**: future-state risk; the 11 sites inherit "LLM never skips on valid inputs" assumption. Failure mode is diagnosable (explicit panic message).
- **ai-engineer P2 (CLI phrasing)**: format `"{S} LLM-skipped ({S}/{P} pair-clusters = {X}%)"`; ai-engineer suggested `"of {P} pair-clusters, {S} LLM-skipped ({X}%)"` for clearer mental model. Cosmetic; leave for post-ship feedback.
- **architecture-reviewer P3 (`SynthesisOutcome` home borderline)**: the type represents orchestration outcome, currently in `synthesis.rs` (parser module). Acceptable for now; revisit if a third variant (e.g., `RateLimited`) lands in an orchestration-only concern.
- **cross-family-fallback #1 (cross-family session-level advisory)**: save for next /ae:retrospect rather than bundling here.

### Disagreement Value Assessment

- **Bundle justification (challenger vs ai-engineer)**: challenger claimed the 60%→90% reframe was arithmetic dressed as evidence. ai-engineer's data-grounded framing (empirical 60% near-dup / 30% topic-adj from N=10 spot-check) was the counter. Neither fully wins — the reframe IS an assumption about hatch precision that is untested pre-ship. Resolution: accepted as known tradeoff with AC5 post-ship audit as the validation mechanism.
- **Fixture design (code-reviewer vs prior-implementation)**: code-reviewer caught that all-skip fixture cannot discriminate pair-denominator from total-denominator. Prior implementation acknowledged this in a comment but shipped anyway. Resolution: replaced with `ClusterSizeAwareProvider` — the test now discriminates. This is the kind of catch /ae:review exists for.

## Doodlestein Review

Not run. Cross-family unavailable; no Doodlestein agents spawned (protocol for cross-family-unavailable sessions is to skip Doodlestein — /ae:review Step 7 not formally required, but the convention tracks cross-family availability). The 3 Doodlestein challenges from discussion 018 (strategic / adversarial / regret) were already baked into plan 011's design; this review's 5-agent team covered the equivalent angles for as-shipped code.

## Outcome Statistics

- **Steps completed**: 2/2
- **Rework rate**: 0 steps needed fixup commits during `/ae:work` (review-stage fixup is c37e152, a single commit).
- **P1 escape rate**: 1 P1 found in `/ae:review` — the pair-denominator test discrimination gap that the plan-review's architect finding partially addressed but didn't fully close. Fixed in fixup. If we count the architecture-reviewer's CLI-regex-fragility finding as a P1, that's a plan-artifact concern, not code, so it doesn't block.
- **Drift events**: 1 during `/ae:work` — Step 2 accidentally committed .DS_Store + scheduled_tasks.lock via `git add -A`; fixed via `git rm --cached` + gitignore extension in commit `1ad499a`. Non-production.
- **Fix loop triggers**: 0.
- **Auto-pass rate**: 2/2 — TL-executed directly, both steps auto-continued with tests green and clean clippy/fmt.
- **Cross-family coverage**: degraded all session (Codex account-limited, Gemini invalid key). Plan review caught 4 Must Fix items; feature review caught 1 P1 + 3 P2s + 1 backlog item. Catch rate appears stable across session despite single-family team.
- **Deferred resolution rate**: N/A (no `DEFERRED` entries in `docs/milestones/011/notes.md` — only step-summaries.md).

Observations for trend analysis:
- Plan 010 review: 5 Must Fix items (mostly type-shape errors — NewMemory.is_longterm, sync/nested-Runtime, LlmFuture return type, naive brace parser, --synthesize default).
- Plan 011 review: 4 Must Fix items (shifted toward spec-precision — line number off-by-one, EXPECTED_SYSTEM_PROMPT implicit, pair-denominator pre-load, Step 3 dangling gate).
- Plan 011 feature review: 1 P1 (test discrimination) + 3 P2s.
- Pattern: ae:work pre-commit review catches superficial issues; plan reviews catch spec-code drift; feature reviews catch test-as-specification gaps. Each layer has distinct value.

## Fixup Commits

- `c37e152` — pair-denominator test fix (P1) + empty-reason warn (P2) + N=10 disclosure (P2) + skip-precedence test (P2) + `BL-synthesis-result-struct-promotion.md` backlog (P2 deferred).

Autosquash/rebase not attempted this session (destructive operations disabled); fixup lands as a labeled standalone commit.

## Deferred Findings Audit

✅ No DEFERRED entries in `docs/milestones/011/notes.md` (file doesn't exist — the milestone dir only contains `step-summaries.md`).

## Backlog Items Filed

- `docs/backlog/BL-synthesis-result-struct-promotion.md` — tuple→struct promotion trigger

Backlog at 14 items (was 13 pre-review).

## Next Steps

Review passed. Suggested next actions (user choice):

1. **`git push origin main`** — 4 commits since last push, ready to ship.
2. **Run `mengdie dream --synthesize` once** — generates the AC5 empirical data that closes the loop. The first post-ship dream run should be audited per the "Post-ship follow-up" section in `docs/plans/011-residuals-reduction.md` and written back to `docs/backlog/BL-clustering-validation.md`'s TODO-stub section.
3. **`/ae:roadmap`** — backlog is at 14 items; good time to group before the next sprint.
4. **`/ae:retrospect`** — with plans 007/009/010/011 all reviewed, trend analysis could surface useful meta-patterns (cross-family degradation, Must Fix shifts, first-caller-validates-design-bets instances).
