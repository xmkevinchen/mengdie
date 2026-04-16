---
id: "013"
title: "What Next After 2-Week Pause — Conclusion"
concluded: 2026-04-16
plan: "docs/plans/004-search-quality-fixes.md"
entities: [rrf, normalization, dreaming, threshold, fts5, tokenization, and-term, validation, mcp-permissions, search-quality]
---

# What Next After 2-Week Pause — Conclusion

5 agents across 2 discussion rounds + Doodlestein. Round 1 discovered a critical math error in the originally favored Option B; Round 2 converged on Option A with UAG validation. All three Doodlestein challenges accepted as implementation refinements.

---

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | RRF Score Normalization Fix | Lower `DEFAULT_MIN_RELEVANCE` from 0.65 to 0.45 in `dreaming.rs:11`. Keep `RRF_MAX = 2.0/61.0`. Link CLI default via `default_value_t`. No avg_relevance reset. | Option B (RRF_MAX=1/61) destroys dual-signal semantics (code-researcher math proof: both single and dual-ranker clamp to 1.0). Option A preserves 0.5 vs 1.0 differentiation. 11 entries immediately qualify — all verified as genuinely useful (UAG passed). | high |
| 2 | FTS5 Query Tokenization Strategy | Replace phrase wrapping (`search.rs:44`) with AND-term matching. Sanitize FTS5 operators, split on whitespace, filter empty tokens and reserved words, join with AND. | Unanimous. Phrase-only kills recall — confirmed by DB data (all scores 0.47-0.50 = vector-only). AND matches industry default (Lucene, Elasticsearch). CJK unicode61 limitation is separate issue, deferred. | high |
| 3 | Post-Fix Validation Protocol | (0) Pre-fix baseline: 5 benchmark queries logged today. (1) Add `mcp__mengdie__*` to settings.json allow list. (2) Apply code fixes. (3) Run `mengdie dream`. (4) 2-week forced-use: manual scorecard, 5 human-written benchmark prompts, 1-2 counterfactual. Pass: ≥60% useful top-3, ≥50% influenced work, ≥1 non-Mengdie useful retrieval, ≥1 counterfactual improvement. | MCP permissions blocker confirmed (not in allow list). Circular validation broken by requiring non-Mengdie project searches. Pre-fix baseline enables before/after attribution. | high |

## Doodlestein Review

| Agent | Challenge | Resolution |
|-------|-----------|------------|
| Strategic | Missing pre-fix baseline measurement — fixes applied simultaneously prevent attribution | **Accepted**: added "pre-fix baseline" as Step 0 in validation protocol |
| Adversarial | `cli.rs:34` uses string literal "0.65", not `DEFAULT_MIN_RELEVANCE` — silent divergence after fix | **Accepted**: implementation must use `default_value_t = DEFAULT_MIN_RELEVANCE` to link values |
| Adversarial | Terms like `"***"` strip to empty string → invalid FTS5 query `rust AND AND memory` | **Accepted**: filter zero-length tokens after stripping, before joining with AND |
| Adversarial | Pass criteria don't enforce non-Mengdie searches — circularity not actually prevented | **Accepted**: added "≥1 non-Mengdie useful retrieval" to pass criteria |
| Regret | Topic 2 most likely reversed — AND may return zero results for long queries without OR fallback | **Noted**: monitor FTS zero-result frequency during validation. OR fallback is the escape hatch if AND proves too restrictive. Not changing decision — AND is correct starting point. |

## Spawned Discussions
None.

## Deferred Resolutions
None — zero deferred items.

## Implementation Notes (from Doodlestein)

**Topic 1 — two locations to change:**
- `src/core/dreaming.rs:11`: `pub const DEFAULT_MIN_RELEVANCE: f64 = 0.45;`
- `src/bin/cli.rs:33-34`: change to `default_value_t = crate::core::dreaming::DEFAULT_MIN_RELEVANCE`
- Same pattern for `DEFAULT_MIN_RECALL` and `DEFAULT_WINDOW_DAYS` if they also use string literals

**Topic 2 — sanitization pipeline:**
1. Strip `"`, `*`, `-`, `(`, `)` from each character in query terms
2. Split on whitespace
3. Filter: drop empty tokens, drop reserved words (`AND`, `OR`, `NOT`, `NEAR`, case-insensitive)
4. Join remaining tokens with ` AND `
5. Empty result → return empty Vec (no FTS query executed)

**Topic 3 — validation sequence:**
0. Pre-fix baseline: run 5 benchmark prompts, log current results
1. Add `mcp__mengdie__memory_search`, `mcp__mengdie__memory_ingest`, `mcp__mengdie__memory_invalidate` to `~/.claude/settings.json` allow list
2. Apply dreaming.rs + cli.rs threshold change
3. Apply search.rs FTS5 tokenization change
4. Run `mengdie dream` — expect 11 promotions
5. Post-fix regression: same 5 benchmark prompts, compare
6. 2-week forced-use with scorecard

## Backlog Items Generated
- Rank-based Dreaming (decouple promotion signal from RRF normalization) — trigger: threshold 0.45 proves too permissive during validation
- CJK tokenizer support (custom FTS5 tokenizer or pre-processing) — trigger: Chinese queries fail during real use
- OR fallback for AND-term matching — trigger: FTS zero-result rate > 30% during validation
- Snippet length increase (200 chars → 500 tokens) — trigger: agent fails to use relevant memory due to insufficient snippet
- Result metadata exposure (matched_terms, rankers_matched) — trigger: debugging search quality becomes painful

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| team-lead | TL (moderator) | Claude Opus | Start |
| architect | Solution design | Claude | Start |
| code-researcher | Code evidence | Claude | Start |
| codex-proxy | Cross-family DX | Codex | Start |
| doodlestein-strategic | Strategic review | Claude | Doodlestein |
| doodlestein-adversarial | Adversarial review | Claude | Doodlestein |
| doodlestein-regret | Regret prediction | Claude | Doodlestein |

## Process Metadata
- Discussion rounds: 2 (+ UAG + Doodlestein)
- Topics: 3 total (3 converged, 0 spawned, 0 deferred)
- Autonomous decisions: 3
- User escalations: 0
- Doodlestein challenges: 5 raised, 4 accepted as refinements, 1 noted for monitoring
- Deferred resolved in Sweep: 0

## Next Steps
→ `/ae:plan` for implementation (2 code fixes + validation setup)
→ Also closes discussion 005 (hybrid search) — FTS5 tokenization decision applies there
