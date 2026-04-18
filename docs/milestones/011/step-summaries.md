# Plan 011 (BL-residuals-reduction) — Step Summaries

## Step 1 — Flip DEFAULT_MIN_SIZE 3→2 + backlog trigger update (commit: 2c25470)

**Decisions**:
- Flipped `DEFAULT_MIN_SIZE` to 2 per discussion 018 empirical data (60% near-dup / 30% topic-adj / 10% unclear spot-check on 10 pair-clusters).
- Doc comment on the constant explicitly cites the revisit trigger ladder: first revert min_size=3, then pursue threshold drop 0.75→0.70 with real-LLM validation.
- BL-clustering-validation trigger #3 now marked "TRIGGER ADDRESSED by plan 011" with a forward-looking signal rewording ("`>50% residuals AND synthesis_hit_rate < 10%`") annotated as pending `synthesis_hit_rate` instrumentation.

**Rejected**:
- Adding a config-file layer for per-project threshold/min_size tuning — over-engineering for solo-dev at 238 memories (architect Round 1).
- Corpus-size-aware dynamic defaults — speculative complexity; constants are easier to audit (architect Round 1).

**Cross-step deps**:
- Step 2 depends on `min_size=2` being the default at CLI invocation time to actually surface pair-clusters into the LLM path.

**Actual files**: `src/core/clustering.rs`, `docs/backlog/BL-clustering-validation.md`

## Step 2 — Null-escape-hatch: SYSTEM_PROMPT + SynthesisOutcome enum + counter + CLI (commit: 1601ac5)

**Decisions**:
- `parse_synthesis_response` return type flipped from `Result<SynthesisDraft, _>` to `Result<SynthesisOutcome, _>` where `SynthesisOutcome = Synthesized(draft) | Skipped { reason }`. Variant is load-bearing for the trigger conditions downstream.
- Pair-cluster denominator counted PRE-DB-load via `trimmed_ids.len() == 2` (architect must-fix); exposed as a tuple return `(SynthesisResult, usize)` rather than a public struct field (cross-family consider: derived display value with no external caller).
- CLI output line prints BOTH absolute counts AND percentage: `"{S} LLM-skipped ({S}/{P} pair-clusters = {X}%)"` so 50% ambiguity (1/2 vs 50/100) is resolved (ai-engineer consider).
- AC5 stub committed to `docs/backlog/BL-clustering-validation.md` — verifiable at /ae:review time via `grep TODO` (≥6 occurrences). The live audit is explicitly outside the plan's /ae:review gate but the stub serves as the tripwire.
- SystemPrompt + EXPECTED_SYSTEM_PROMPT updated byte-for-byte (dep-analyst must-fix: the regression guard at synthesis.rs:198 would fail silently if they drifted).

**Rejected**:
- `--enable-null-escape` feature flag (ai-engineer Round 3 alternative) — rejected in favor of bundled shipping + post-ship qualitative audit. Rationale: operator discipline to toggle across runs is fragile; the `syntheses_llm_skipped` counter already provides per-intervention signal.
- `Skipped { source_memory_ids }` enriched variant — ai-engineer low-priority Consider; info-level log at the call site carries `cluster_ids` which covers the audit need.
- `SynthesisMode` enum on the prompt builder — architect noted this was the right shape IF a separate pair-prompt were in scope, but the single inline-branch prompt keeps it simpler.
- CLI flag `--second-pass-threshold` and second-pass clustering strategy — discussion 018 deferred this; null-escape filters the near-adjacency case.

**Cross-step deps**:
- `SynthesisOutcome` public surface: Step 3 (if/when a second-pass plan materializes) would add a 3rd variant or a `SynthesisMode::Pair` toggle, but the first-caller contract is locked in here.
- `syntheses_llm_skipped` field is now part of `SynthesisResult`; any future observability (synthesis_hit_rate, retry logic) reads this counter.
- The `pair_clusters_processed` tuple return is deliberately NOT on `SynthesisResult` — future callers should NOT plumb this into other code paths; it's a CLI-layer display value only.

**Actual files**: `src/core/synthesis.rs`, `src/core/dreaming.rs`, `src/bin/cli.rs`, `tests/dream_synthesis.rs`, `docs/backlog/BL-clustering-validation.md`, `.gitignore` (unrelated cleanup: untracked .DS_Store + scheduled_tasks.lock accidentally swept by git add -A in the Step 2 commit — fixed in follow-up commit `1ad499a`).

**Test count**: 184 passed + 5 ignored (up from 178 + 5). 4 new synthesis parser tests (skip happy path, skip missing reason, skip with LLM preamble, skip=false treated as synthesis) + 2 new dreaming skip-variant tests (counter-increments-no-db-write, pair-denominator-counted-against-pairs).

**Cross-family**: degraded entire session (Codex account-limited, Gemini invalid key). Step-level code review was Claude-only. Accumulated Doodlestein checkpoint: plan 011 has 2 steps — trigger condition `(total_steps >= 3 AND current == total)` is NOT satisfied (2 < 3), so the midpoint/final checkpoint is skipped per protocol. No accumulated P1 possible → defaults to `no_accumulated_p1 = true`.
