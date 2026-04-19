## Step 1 — Add pair_* fields to SynthesisResult, fix CLI, add discrimination test (commit: 63d83b0)
**Decisions**:
- Consolidated `pair_clusters_processed` + `pair_clusters_skipped` onto `SynthesisResult` (not a wrapper struct). Plan review's challenger C won — architect + code-reviewer concurred. Numerator and denominator of the pair-skip metric are now co-located with `syntheses_llm_skipped`.
- CLI numerator changed from `syn.syntheses_llm_skipped` to `syn.pair_clusters_skipped`; format string shape preserved (`"{S_total} LLM-skipped ({S_pair}/{P_pair} pair-clusters = {X}%)"`).
- Added `test_pair_clusters_skipped_excludes_non_pair_skips` using all-skip FixedProvider + mixed cluster sizes — discriminates the numerator bug (buggy impl would yield pair_clusters_skipped == 4 instead of 2).
**Rejected**:
- Wrapper struct `SynthesisPassResult` (original plan draft) — rejected unanimously at plan review as over-engineered. BL-synthesis-result-struct-promotion closed as superseded.
- Option B (keep math, fix label) — rejected because the pair-cluster skip rate is the specific trigger threshold in BL-clustering-validation.md; the operator signal requires pair-specific numerator.
**Cross-step deps**: none — single-step plan.
**Actual files**: src/bin/cli.rs, src/core/dreaming.rs, tests/dream_synthesis.rs
**Smoke test (AC3)**: Production DB run displayed `9 LLM-skipped (3/11 pair-clusters = 27%)` — matches BL-clustering-validation audit. Pre-fix displayed `11 LLM-skipped (11/11 pair-clusters = 100%)`.
