---
round: 1
date: 2026-04-16
score: converged
---

# Round 1

## Discussion

**Round 1 (independent research):**
- All three agents independently converged on AND-term matching as the correct approach.
- Architect: AND-term with operator stripping. Sanitize FTS5 operators, split on whitespace, join with AND. Fallback to OR if AND returns 0 (optional). NEAR rejected as unnecessary complexity.
- Code-researcher: Confirmed exact escaping code at search.rs:44. Identified FTS5 default tokenizer is unicode61 (schema.rs:58-59, no custom tokenizer). CJK ideographs treated as single tokens — Chinese FTS effectively non-functional regardless of approach.
- Codex-proxy: Ranked strategies by agent ergonomics. AND-term scored highest on predictability, agent intuition, safety, and explainability.

**Round 2 (refinement):**
- CJK concern discussed: unicode61 doesn't split CJK ideographs. "梦蝶知识库" indexed as one token, not five. Chinese queries degrade to vector-only (same as current behavior). Deferred to backlog — separate issue from AND vs phrase.
- Operator sanitization scope agreed: strip `"`, `*`, `-`, `(`, `)` from individual terms. Filter reserved words `OR`, `NOT`, `NEAR`, `AND` as standalone tokens (case-insensitive). Empty result after stripping → return empty FTS results.
- Result metadata (matched_terms, rankers_matched) deferred — not needed for MVP, adds API surface without demonstrated need.
- This decision also closes discussion 005 (hybrid search analysis, `discuss: pending` → resolved).

## Outcome
- Score: converged
- Decision: Replace phrase wrapping at search.rs:44 with AND-term matching. Implementation: (1) sanitize FTS5 operators from input, (2) split on whitespace, (3) filter reserved words, (4) join remaining tokens with ` AND `. Single-token queries pass through directly.
- Reversibility: HIGH (tokenization strategy change, no schema migration)
- Backlog: CJK tokenizer support (custom FTS5 tokenizer or pre-processing), result metadata exposure
