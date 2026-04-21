---
title: "Plan 013 Review — Gemini Proxy (Operator Rollout Angle)"
date: 2026-04-20
reviewer: gemini-proxy (local gemma4:26b)
---

# Plan 013 Cross-Family Review — Operator Rollout + Observability

## Verdict: **PASS** with 4 flagged actions for implementer

The plan is complete and ship-ready. The exponential decay formula, demotion semantics, and dry-run validation path are all sound. However, the implementation must address four operational gaps that could affect rollout clarity or introduce downstream breakage.

---

## 1. Operations Doc Completeness

**Status**: Plan defines the required sections; implementer should expand with clarity.

### Current Coverage
The plan mandates three required sections in `docs/operations/dreaming-decay.md`:
- (a) Pre-ship dry-run procedure (`--dry-run-decay` first run, thresholds for halt)
- (b) Revisit triggers (copied from conclusion: `avg_effective_relevance < 0.25`, corpus age > 90 days, IQR drift)
- (c) LONGTERM_BOOST cliff explanation (one-time rank discontinuity when `is_longterm` clears)

### Gaps Identified

**Gap 1: Metric Interpretation Guide** — Operators need a clear mapping of counter pairs to action:
- **High `decay_floor_breaches` + low `demoted`** → Threshold tuning required; memories are hitting the floor but demotion conditions not met elsewhere
- **High `demoted`** → Aggressive decay phase (expected post-threshold convergence)
- **Rising `avg_effective_score_after` ∧ declining count** → Health indicator; corpus is shedding old memories while maintaining relevance

**Recommendation**: Add a **"Interpreting the Metrics"** section with 3–4 brief bullet points showing (counter state) → (interpretation) → (action if abnormal).

**Gap 2: Data Freshness Context** — The revisit trigger "corpus age > 90 days" is ambiguous if ingest is bursty.
- If data arrives in waves (e.g., big AE discussion batch), the "age" of the corpus is dominated by the oldest memory, not the freshness of recent ingests.
- Operators may misread this as a requirement to rescan 90+ days of data when the real intent is "if no recalls have occurred in 90 days."

**Recommendation**: Add a **"Data Freshness"** section clarifying:
- Decay is **relative to `last_recalled` timestamp**, not wall-clock time since ingest.
- If the most recent ingest was recent, decay will remain low even if oldest memories are very old.
- Trigger "corpus age > 90 days" means "longest gap since last recall in the corpus exceeds 90d" — not calendar age of the database.

---

## 2. CHANGELOG.md Presence

**Status**: **BLOCKER** — File does not exist; plan assumes it does.

### Current State
- No `CHANGELOG.md` exists in the repo.
- Step 5 instructs: "Add a brief `CHANGELOG.md` entry under `## Unreleased` section (or equivalent if absent)."

### Decision
**Do NOT treat CHANGELOG creation as a separate BL.** It is a required task for this plan delivery.

### Action for Implementer
In **Step 5**, add a preliminary sub-step:
1. Check if `CHANGELOG.md` exists.
2. If absent, create it with this structure:
   ```markdown
   # Changelog

   ## Unreleased

   ### Added
   - **BL-008: Exponential decay for Dreaming** — Compute effective relevance via `2^(-d/60)` decay factor. Memories below floor (0.20) are demoted, losing the LONGTERM_BOOST search multiplier. See `docs/operations/dreaming-decay.md` for operator procedure.
   
   ## [0.1.0] — 2026-04-XX
   (future: backfill past releases as needed)
   ```
3. If it exists, append the BL-008 entry under the existing `## Unreleased` section.

---

## 3. CLI Output Backward Compatibility — **CRITICAL**

**Status**: Plan changes output format; must verify no downstream consumers break.

### Current Output
```
Dreaming complete: P promoted out of Q eligible memories (thresholds: recall≥R, relevance≥0.XX, window=Nd)
```

### New Output (Per Plan)
```
Dreaming pass: P promoted, D demoted (B floor breaches, avg effective 0.XXX → 0.YYY)
```

### Risk Assessment
- **No direct tests found** in this repo that parse the old format.
- **No scripts (.sh files) found** that depend on this output.
- **`ae:retrospect` plugin** — unknown if it has regex extraction for `mengdie dream` output. **This is the largest risk.** If the AE plugin pipeline has a hidden log-scraper or retrospect injection that expects the old string, this breaks it.

### Action for Implementer
**Before Step 4 (CLI output changes), verify**:
1. Does the agentic-engineering repo (or any sibling AE plugin) have a parser that looks for `"Dreaming complete"` or similar?
   - Grep the AE codebase for regex patterns matching `mengdie dream` output.
   - If found: **HALT**. Coordinate with AE team; either preserve the old line or add a `--legacy-format` flag.
   - If not found: **PROCEED**.
2. Document the verification (pass/fail) in a code comment at the output site (`cli.rs:217` or nearby) so future changes don't surprise anyone.

**If verification shows zero parsers**: The format change is safe.

---

## 4. Dry-Run Flag Naming

**Status**: Minor UX — plan uses `--dry-run-decay`, Gemini suggests `--decay-dry-run`.

### Current Plan
```
mengdie dream --dry-run-decay
```

### Recommendation
Rename to **`--decay-dry-run`** for consistency.

**Rationale**: 
- Existing pattern in this repo: `--synthesize --dry-run` (action, then mode).
- Gemma suggested `--decay-dry-run` follows the `[subject]-[mode]` convention cleanly.
- Avoids ambiguity: `--dry-run-decay` reads like "dry-run the decay subcommand" (which doesn't exist), while `--decay-dry-run` clearly means "decay in dry-run mode."

**Alternative considered**: `--inspect-decay` (verb-noun). Shorter but less parallel with `--synthesize --dry-run` already in use.

**Final directive**: Use `--decay-dry-run`. It's unambiguous, verbose (which is fine), and fits the established pattern.

---

## Summary of Implementer Actions

| # | Task | Blocker? | Notes |
|---|------|----------|-------|
| 1 | Expand `docs/operations/dreaming-decay.md` with **Metric Interpretation** + **Data Freshness** sections | No | Improves operator usability |
| 2 | Verify/create `CHANGELOG.md` before Step 5 | Yes | Currently missing; plan assumes it |
| 3 | Audit `ae:retrospect` for regex parsers of `mengdie dream` output | Yes | If found, coordinate with AE team |
| 4 | Rename `--dry-run-decay` → `--decay-dry-run` | No | UX consistency |

All other plan steps are ready to ship.

---

## Sign-Off

Plan 013 is **APPROVED FOR IMPLEMENTATION**. The decay formula, demotion semantics, and test strategy are solid. Operator rollout is safe given the dry-run validation path. Address the four actions above, and this ships clean.
