---
type: strategic-finding
plan: "016"
reviewer: doodlestein
date: 2026-04-23
---

# Strategic Finding: plan 016

## Single Smartest Improvement

**Fix the "No new tests" contradiction in the Out of Scope section.**

**The problem**: Line 38-39 of the plan (Out of Scope) contains an unqualified bullet:
> "No new tests. The existing AC5 regex tests at `cli.rs:719-736` already encode the Unicode-only reality; they become the source of truth and don't need modification."

Step 4 and AC4 then explicitly create `tests/ops_doc_sql.rs` — a new test file. The Out of Scope bullet was written for the arrow/Unicode scope decision but was left unscoped, making it read as a plan-wide rule. None of the 5 reviewers flagged this because they were evaluating individual findings, not the plan's self-consistency.

**Why this compounds**: `/ae:work` reads the full plan top to bottom. A worker encountering "No new tests" in Out of Scope, then reaching Step 4's "Create `tests/ops_doc_sql.rs`", faces a contradiction. The conservative resolution is to skip Step 4 and satisfy the "no new tests" constraint — which drops the safety-critical SQL drift guard that three reviewers (C3, gemini P2.4, codex P1.2) independently confirmed as necessary. The triple-confirmation means Step 4 is load-bearing, not optional. A contradicted Step 4 is the worst outcome of this plan.

**The fix**: One-line scope tightening in Out of Scope:

Change:
> No new tests. The existing AC5 regex tests at `cli.rs:719-736` already encode the Unicode-only reality; they become the source of truth and don't need modification.

To:
> No new tests for arrow/Unicode behavior. The existing AC5 regex tests at `cli.rs:719-736` encode the Unicode-only reality and are not modified by this plan. (Step 4 adds `tests/ops_doc_sql.rs` for SQL snippet drift detection — that is explicitly in scope.)

This is a documentation fix to the plan itself, not a scope change. It costs one sentence and eliminates ambiguity before `/ae:work` sees the plan.

## Secondary Observation (not actioned)

AC4 references the field as `avg_effective_before` (line 126: "returns `avg_effective_before` computed over exactly 1 row"). The actual struct field in `src/core/dreaming.rs:309` is `avg_effective_score_before`. This is a field-name mismatch in the AC that would cause a compile error in the cross-check assertion. Worth fixing in the same breath as the Out of Scope correction, but it's P3 — the primary finding stands alone.
