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

---

## Step 2 — Rollback section (commit: TBD)
**Decisions**:
- Inserted between "Required first-run procedure" and "Metric interpretation guide" (line 85 onwards) per plan 016 review challenger blind spot — incident-response colocated with the approval gate, not buried after Baseline.
- Stress-readable order applied (codex P2): when-to-use → required input → **"if breach list is LOST" branch UPFRONT with honest limits** → rollback SQL (with JSON→SQL quoting) → verification → `last_recalled` follow-up → demotion-is-minimal-write completeness note.
- `jq -r '.breaches | map("\'" + . + "\'") | join(", ")'` is the canonical JSON→SQL conversion command (with a sed fallback for systems without jq). This addresses architect's "silent no-op on quote mismatch" failure mode directly in the ops doc.
- Rollback SQL template wrapped in `<!-- rollback-snippet:begin -->` / `<!-- rollback-snippet:end -->` markers for Step 4's optional second-test coverage.
- Field-name cleanup per codex P2: `breached_ids` → `breaches` at the Metric-interpretation-guide trailer (line 209-214). Kept an explicit clarifying note citing the pre-plan-015 naming history so reviewers reading git blame don't get confused.
- "Lost breach list" recovery is honestly limited per plan 016 review P1.2 — does NOT invent a false `valid_until` timestamp recovery (demotion doesn't write `valid_until`; verified at src/core/dreaming.rs:251-256). Documented paths: shell history, scrollback, redirected stderr, operator memory. Mitigation going forward: redirect stderr to a file.

**Rejected**:
- Keeping the `breached_ids` reference as-is with a footnote — half-measure; plan 015 locked the contract as `breaches`, the doc should match.
- Adding a second "undo the last_recalled update" rollback procedure — too many rollback layers; operator can re-read the memory to naturally update `last_recalled` if needed.

**Cross-step deps**:
- The rollback-snippet markers feed Step 4's optional parallel test (extract UPDATE template, substitute test UUID, verify is_longterm=1 post-update).
- The JSON→SQL `jq` conversion pattern is recorded in the doc; Step 4 does not need to re-implement it (the test operates on a constructed UPDATE, not on the jq pipeline).

**Actual files**: `docs/operations/dreaming-decay.md`

---

## Step 3 — Plan 013 AC5 correction note + BL close (commit: TBD)
**Decisions**:
- Correction note rendered as a `>` blockquote immediately after the AC5 heading at line 192, before the first AC5 bullet at line 194. Blockquote format visually distinguishes it from the original AC text and signals "post-ship annotation" without mutating the AC itself.
- Rejected alternative is named explicitly as "dual emission" (codex P2). Reversal path documented (~10-line diff touching format_dreaming_line + the two regex tests in cli.rs).
- BL body gets a 4-item shipped-actions blockquote (mirrors plan 015's BL close-out pattern) + frontmatter `closed: 2026-04-23` + `closed_by: plan-016`. Includes a cross-reference note about the `breached_ids` → `breaches` cleanup so future git-blame readers understand the rename history.
- `.ae/` BL edit is local-only (gitignored per project convention) — same pattern as plan 015 Step 6.

**Rejected**:
- Amending line 194 of plan 013 in-place — would erase the audit trail. Append-only preserves history per architect-approved pattern.
- Duplicating the correction note into `docs/reviews/011-bl008-exponential-decay.md` — doubling up creates drift risk.

**Cross-step deps**:
- None — Step 3 is independent of Steps 1 and 2 (different files).
- Step 4's test does NOT depend on Step 3 content.

**Actual files** (tracked): `docs/plans/013-exponential-decay.md`
**Actual files** (local-only, in `.ae/`): `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md`
