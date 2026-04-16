---
id: "02"
title: "FTS5 Query Tokenization Strategy"
status: converged
current_round: 1
created: 2026-04-16
decision: "AND-term matching with operator sanitization. Strip FTS5 operators, split on whitespace, join with AND."
rationale: "Unanimous — AND matches industry default, best agent ergonomics. Phrase-only kills recall (confirmed by DB data). CJK limitation is separate issue, deferred."
reversibility: "high"
reversibility_basis: "Tokenization strategy change, no schema migration"
---

# Topic: FTS5 Query Tokenization Strategy

## Current Status
Pending — analysis and discussion 005 both confirm phrase-only matching kills multi-term recall. Discussion 005 is in `discuss: pending` state.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
Current code (search.rs) wraps the full query in `""`, treating it as an FTS5 phrase literal. This prevents FTS5 operator injection but kills recall: "JWT authentication" won't match documents where those words appear in different sentences. At 46 memories, most multi-word queries return 0 FTS5 results, making hybrid search vector-only.

Standards-expert identified 4 approaches:
1. Term AND matching: split + join with AND (most common industry default)
2. Phrase + fallback: try phrase first, fall back to AND if <N results
3. NEAR matching: `term1 NEAR/10 term2` (proximity, FTS5-native)
4. BM25 term matching: no wrapping, implicit OR (Lucene default)

The original phrase wrapping was for injection safety — any replacement must sanitize FTS5 operators (AND, OR, NOT, NEAR, `""`) from user input.

## Constraints
- Must prevent FTS5 operator injection (queries come from AI agents, content derived from external documents)
- Must work with FTS5's default tokenizer (no stemming, no CJK support)
- Performance not a concern at 46 entries
- This decision also closes discussion 005 (hybrid search analysis)

## Key Questions
- AND vs OR as default join: AND is more precise but may return 0 for long queries. OR is more recall-friendly but noisier. Which is better for agent-generated queries?
- Should single-word queries be handled differently from multi-word?
- Is NEAR matching worth the complexity vs simple AND?
- How should CJK queries be handled (Kai's first language is Chinese)?
