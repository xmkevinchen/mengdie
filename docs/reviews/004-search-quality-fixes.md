---
id: "004"
title: "Review: Search Quality Fixes (Dreaming Threshold + FTS5 Tokenization)"
type: review
created: 2026-04-16
target: "docs/plans/004-search-quality-fixes.md"
verdict: pass
---

# Review: Search Quality Fixes

## Verdict: PASS

Commit: `ba56a22` (squashed fixup from review)

## Review Summary

4 reviewers (code-reviewer, security-reviewer, challenger, codex-proxy). One P2 finding (hyphenated term regression) caught by challenger, fixed and squashed.

### Findings

| # | Finding | Severity | Source | Disposition |
|---|---------|----------|--------|-------------|
| 1 | `sanitize_fts_query` stripped non-alphanumeric chars instead of splitting on them — "rust-lang" → "rustlang" (matches nothing in FTS5) | P2 | challenger | **FIXED** (split on `!is_alphanumeric()` boundaries, aligning with FTS5 unicode61 tokenizer) |
| 2 | Fullwidth Unicode homoglyphs (ＡＮＤ) not caught by reserved word filter | P3 | security-reviewer | Skip — not FTS5 operators, no security impact |
| 3 | Score >0.5 assertion doesn't independently prove FTS5 contribution | P3 | challenger | Skip — RRF math: vector-only rank-1 = exactly 0.5, not >0.5 |
| 4 | No test for 0.45-0.65 threshold boundary band | P3 | challenger | Skip — threshold is a constant, not logic |
| 5 | MCP tool description doesn't document AND semantics | P3 | challenger | Skip — description doesn't promise specific query semantics |

### Disagreement Value Assessment

Challenger raised valid concern about both reviewers missing the hyphenated term regression. Code-reviewer and security-reviewer focused on injection safety and operator handling but did not verify that sanitized tokens align with FTS5's unicode61 tokenizer output. The challenger's empirical verification (FTS5 test showing "rustlang" matches 0 results) was the decisive evidence. The fixup changes `filter(|c| c.is_alphanumeric()).collect()` to `split(|c| !c.is_alphanumeric())` — splitting on non-alphanumeric boundaries instead of stripping them, producing tokens that match how unicode61 indexes content.

## Outcome Statistics
- Steps completed: 4/4
- Rework rate: 0 steps needed fixup commits during /ae:work (0/4 = 0%)
- P1 escape rate: 0 P1 findings in /ae:review
- P2 escape rate: 1 P2 finding caught in review (sanitizer token alignment)
- Drift events: 0
- Fix loop triggers: 0
- Auto-pass rate: 4/4 steps auto-continued (100%)
- Deferred resolution rate: N/A (no deferred findings)

## Team Composition

| Agent | Role | Backend |
|-------|------|---------|
| code-reviewer | General code review | Claude |
| security-reviewer | FTS5 injection safety | Claude |
| challenger | Pure opposition | Claude |
| codex-proxy | Cross-family correctness | Codex |
