---
id: "018"
title: "Residuals reduction — parameter flip + null-escape — Conclusion"
concluded: 2026-04-18
plan: ""
entities: [residuals-reduction, residuals, reduction, min-size-flip, min, size, flip, null-escape-hatch, null, escape, hatch, parameter-strategy, parameter, strategy, second-pass-defer, second, pass, defer, synthesis-hit-rate, synthesis, hit, rate, clustering, dream-synthesis]
---

# Conclusion: Residuals reduction

## Decision summary

**Ships**: `DEFAULT_MIN_SIZE` flipped 3→2 in `src/core/clustering.rs`, bundled with a null-escape-hatch in `synthesis.rs`. One plan, two steps, one PR.

**Defers**: threshold change (0.75→0.70) — no real-LLM quality data yet for that parameter band. Second-pass architecture — not needed given the null-escape path. Ingestion-time plan/backlog dedup — separate backlog entry, future discussion.

**BL-clustering-validation updated**: ">50% residuals = signal" trigger replaced with ">50% residuals AND synthesis_hit_rate < 10% = signal". Residual % alone is too coarse; synthesis_hit_rate (query top-K hits for source_type=synthesis, Tier 1 cosine-cohesion check) measures whether syntheses are recalled and faithful. No code change — backlog wording only.

## Rationale

Parameter sweep (238 memories, 2026-04-18) showed `DEFAULT_MIN_SIZE=3` leaves ~49% residuals at threshold 0.75. The recoverable segment is pair-clusters — memories similar to exactly one other that fail the 3-member floor. Lowering min_size to 2 recovers them without touching threshold, so over-merging risk is zero.

Empirical spot-check of 10 pair-clusters: 60% near-duplicate (synthesis-worthy), 30% topic-adjacent (LLM should reject), 10% unclear. Raw 60% sits at the top of the "mixed" band that would normally favour Option 4 (wait). However, the 30% topic-adjacent pairs are exactly the case the null-escape-hatch filters: if the LLM is instructed to return `null` for weak pairs, ~90% of pair-clusters are either synthesized (60%) or cleanly skipped (30%). That reframes the spot-check from "mixed → wait" to "data sufficient → ship with quality gate".

Bundling the null-escape with the parameter flip is correct because: (a) the empirical data that justified deferral was not yet available when defer was recommended — it is now; (b) `syntheses_llm_skipped` is a separate counter from `residuals_skipped`, so the first real run produces two independent signals rather than one confounded one; (c) the files are non-overlapping (`clustering.rs` vs `synthesis.rs`/`dreaming.rs`), so a null-escape bug does not contaminate the parameter-flip revert path.

## What the plan must include

**Step 1 — parameter flip** (one line):
- `DEFAULT_MIN_SIZE: usize = 2` in `src/core/clustering.rs:23`
- Doc comment on the constant: cite discussion 018, trigger = "3–5 dream runs; if skip rate > 25% of pairs OR manual spot-check shows majority-weak syntheses, revisit min_size=3 or threshold change"
- Commit message references `docs/backlog/BL-clustering-validation.md`

**Step 2 — null-escape-hatch**:
- `SYSTEM_PROMPT` in `src/core/synthesis.rs` updated: instruct LLM to return `{"skip": true, "reason": "..."}` if the cluster members are topically unrelated and synthesis would add no value
- `SynthesisOutcome` enum replaces `SynthesisDraft` as the return type of `parse_synthesis_response`:
  ```rust
  pub enum SynthesisOutcome {
      Synthesized(SynthesisDraft),
      Skipped { reason: String },
  }
  ```
- `run_synthesis_pass` in `src/core/dreaming.rs`: handle `Skipped` variant — no DB write, increment `syntheses_llm_skipped` counter
- `SynthesisResult` struct: add `syntheses_llm_skipped: usize` field
- CLI output line updated (Doodlestein-strategic fix): "Synthesis: N created, **S LLM-skipped (X% of pair-clusters)**, K residuals skipped, E errors". The skip percentage relative to pair-clusters specifically (not total clusters) must be printed, because the >25% revisit trigger is defined against pair-clusters. Without this line, the trigger is unobservable. Acceptance criterion for Step 2: the first dream run after ship produces an output line containing the skip percentage; grep for "LLM-skipped" in the output must succeed.
- **API migration note**: `SynthesisDraft` is now wrapped inside `SynthesisOutcome::Synthesized`. All existing tests that destructure `SynthesisDraft` directly (≥10 in BL-007 test suite) must be updated to pattern-match `SynthesisOutcome::Synthesized(draft)`. This is mechanical but must be called out explicitly in the plan so the executor does not miss it.
- New unit tests: `parse_synthesis_response` returns `Skipped` for `{"skip": true, "reason": "..."}` payload; `run_synthesis_pass` with stub returning `Skipped` increments counter and writes no row

**BL-clustering-validation backlog update** (no code):
- Replace trigger wording: ">50% residuals = signal" → ">50% residuals AND synthesis_hit_rate < 10% = signal"
- Add synthesis_hit_rate definition: fraction of `memory_search` top-K results that include at least one `source_type=synthesis` row, measured over a rolling window of 20 queries

## Doodlestein review

| Agent | Finding | Resolution |
|-------|---------|-----------|
| Strategic | Skip rate is tracked via counter but not printed in run output → >25% revisit trigger is unobservable without log grep | Accepted. Added explicit AC to Step 2 requiring the CLI output line include "S LLM-skipped (X% of pair-clusters)". |
| Adversarial | Council crossed the 70% ship bar by narrative (null-escape "covers the gap") on an N=10 sample. True population could be 55/35 split within sampling error, making the hatch carry more load than the 30% estimate predicts. Trigger fires immediately with no encoded remediation. | Partially accepted. The 70% bar was reframed (synthesis-worthy → synthesis-worthy-OR-correctly-rejectable), which is defensible only if the hatch actually rejects correctly. Mitigation: elevate the post-ship audit from "spot-check 3 skipped clusters" to "audit ALL clusters the hatch skipped, target N≥5, minimum audit for N<5"; the audit validates the reframe premise rather than trusting it. If N<5 skips occur on the first run, wait for a second run before accepting the reframe. Remediation path if trigger fires: see Trigger condition #1 below — falls back to min_size=3 revert or pre-LLM cosine filter, both one-commit changes. |
| Regret | Null-escape-hatch bundled with min_size flip is most regret-prone. If LLM over-rejects at skip rate > 25%, we can't isolate min_size vs prompt as the cause — the exact measurement contamination architect warned about in Round 2. | Partially accepted. The `syntheses_llm_skipped` counter (separate from `residuals_skipped` and synthesis successes) gives per-intervention signal. If skip rate is high AND synthesis quality on the 60% non-skip fraction is also poor, both interventions are problematic; revert sequence = first prompt, then min_size. If skip rate is high but the non-skip syntheses are good, the hatch is the issue alone — revert prompt only. Separate attribution is recoverable via the counter breakdown. |

## Dissent acknowledged

**ai-engineer (Round 3, mild dissent)**: bundling the parameter flip with the null-escape-hatch risks measurement contamination — we cannot attribute output-quality changes to either intervention alone. Counter-proposal was to ship escape code behind a feature flag default-OFF so run 1 measures the parameter flip alone, run 2 measures the hatch delta.

**TL accepted bundle with mitigation, not the feature flag.** The `syntheses_llm_skipped` counter already gives per-run independent signal on hatch behavior. The remaining attribution question — "did synthesis quality improve because the hatch filtered noise, or because pair-clusters are inherently good signal?" — is answerable via a **post-ship qualitative audit**: after the first dream run at the new defaults, manually spot-check at least 3 of the clusters the hatch skipped. If the skips look like genuine topic-adj rejections, the hatch is working as designed. If skips look like false negatives (near-dup pairs the LLM wrongly rejected), we have signal that the prompt needs tightening. This audit should be executed as part of the first real post-ship dream run and its result added to BL-clustering-validation as an additional AC5-style writeback.

A feature flag would add tuning surface (minimal-change: "no new CLI flag"), require operator discipline to toggle across runs, and not actually resolve the "did the pair-clusters need rejection in the first place" question — it just delays it. The post-ship audit is a cheaper, more direct answer to the same measurement question.

**challenger (Round 3, execution note)**: the `synthesis_hit_rate` trigger wording in BL-clustering-validation is unmeasurable until instrumentation exists. Either (a) add a third plan step to build a `mengdie stats --synthesis-hits` command, or (b) mark the trigger as "pending instrumentation".

**TL accepted option (b)** — the BL-clustering-validation wording will include "(synthesis_hit_rate instrumentation deferred; trigger uses residual-% only until search logging exists)". Building the instrumentation as a third step in this plan would bundle a third concern (observability) alongside two already-bundled concerns (parameter tuning + LLM contract change). The plan stays at 2 steps. Instrumentation is its own future plan when we have a concrete reason to need per-query logging (e.g., search quality debugging or usage analytics become a feature).

## Trigger conditions for next review

1. **Skip rate > 25% of pair-clusters** across 3–5 dream runs → the 30% topic-adjacent estimate was too optimistic; revisit min_size or add a pre-LLM cosine-cohesion filter
2. **synthesis_hit_rate < 10%** after 2 weeks of normal use → syntheses are not being surfaced in search; investigate whether FTS/vector scoring underweights synthesis content, or whether synthesis content is too generic to match real queries
3. **Manual spot-check of 5 syntheses shows majority-weak** → prompt or threshold problem; file a targeted fixup
4. **Residuals stay > 50% after min_size=2 flip** → threshold 0.70 flip becomes the next candidate; run the parameter sweep at 0.70 first, validate cluster quality against real LLM output before committing

## Spawned Discussions

None. (ai-engineer's Q3 observation about plan↔backlog ingest-time dedup is filed as a separate backlog entry idea, not a sub-discussion — future `/ae:discuss` or `/ae:plan` cycle if the pattern persists post-ship.)

## Deferred Resolutions

None. All 2 topics converged. No deferred items entered the Sweep.

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| team-lead | Moderator | Claude | Start |
| engineering-ai-engineer | ML/LLM parameter + prompt design (first real use of imported agent) | Claude | Start |
| engineering-minimal-change-engineer | Scope discipline, resist expansion (first real use) | Claude | Start |
| architect | Solution scoping, structural considerations | Claude | Start |
| challenger | Pure opposition, premise-questioning | Claude | Start |
| doodlestein-strategic | Improvement angle at close-out | Claude | Doodlestein |
| doodlestein-adversarial | Blunder angle at close-out | Claude | Doodlestein |
| doodlestein-regret | Reversal-prediction angle at close-out | Claude | Doodlestein |

Cross-family proxies (codex, gemini) unavailable this session — single-family Claude team per CLAUDE.md fallback protocol. No cross-family review this discussion.

## Process Metadata

- Discussion rounds: 2 (Round 1 independent research; Round 2 targeted questions + empirical spot-check; final synthesis + Doodlestein)
- Topics: 2 total (2 converged, 0 spawned, 0 deferred, 0 explained)
- Autonomous decisions: 2 (both topics resolved without escalation)
- User escalations: 0
- Doodlestein challenges: 3 raised, 3 addressed (1 accepted-with-fix — skip-rate output line; 2 partially-accepted with mitigations baked into the plan and post-ship audit)
- Deferred resolved in Sweep: 0 (none existed)
- Analyze cycle preceded the discuss: 1 quick parameter sweep (corpus 238, 7 parameter combinations dry-runned) + 1 empirical pair-cluster spot-check (N=10) inside Round 2

## Next Steps

→ `/ae:plan` for the 2-step parameter-flip + null-escape-hatch plan (next backlog id: BL-residuals-reduction; no code change for the synthesis_hit_rate trigger wording — that's a backlog edit bundled into the plan commits or a separate tiny commit).
→ After first real dream run post-ship: execute the post-ship audit (count of skipped clusters, manual inspection of skip correctness), write results back to `docs/backlog/BL-clustering-validation.md` under a new "BL-residuals-reduction empirical results" subsection.
→ Revisit this discussion's trigger conditions after 3–5 dream runs have produced skip-rate data.
