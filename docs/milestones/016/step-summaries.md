# Plan 016 — Step Summaries

## Step 1 — Threshold computation snippet (commit: TBD)
**Decisions**:
- SQL filter uses exact 3-condition triple (`is_longterm = 1 AND valid_until IS NULL AND last_recalled IS NOT NULL`) matching `src/core/dreaming.rs:163-167` per plan 016 review P1.1 — null-`last_recalled` rows are immune to demotion and correctly excluded from denominator.
- Bash helper uses shell ternary `count / 10 > 10 ? count / 10 : 10` rather than `python` or `awk` to avoid adding dependencies — operator runs sqlite3 + bash only. Verified against `count=5` (→ 10) and `count=150` (→ 15).
- HTML-comment markers `<!-- threshold-snippet:begin -->` / `<!-- threshold-snippet:end -->` are the locked names (referenced identically by Step 4's test).
- Snippet appended after existing step-3 prose rather than replacing — preserves the HALT recommendation + recalibrate-floor option context.

**Rejected**:
- Single-liner SQL (collapsing onto one line) — chose multi-line form matching the existing query style at dreaming-decay.md:79-88 for readability.
- Inline `python -c "import math; print(max(10, n//10))"` — overkill for integer ceiling; pure bash keeps the snippet dependency-free.

**Cross-step deps**:
- The locked marker names + the 3-condition filter become Step 4's test assertion target. Any edit to the marker names or the filter conditions requires a matching update in `tests/ops_doc_sql.rs`.

**Actual files**: `docs/operations/dreaming-decay.md`
