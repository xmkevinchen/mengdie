---
id: BL-048
title: "HyDE query mode — Hypothetical Document Embeddings for abstract AE-skill queries"
status: open
created: 2026-05-18
origin: "v0.0.2 positioning discussion 2026-05-18 (QMD borrow #2 / Tier C — highest-ROI search improvement): QMD has `hyde` query type. Pattern: LLM generates a hypothetical answer to the user query, embeds the hypothetical answer, searches with that vector instead of the query's. Effective on abstract queries like 'what should we do about X' — exactly the shape AE skills send."
size: M
depends_on: []
v_target: "v0.0.2 — Tier C (QMD borrow); highest-ROI search improvement"
---

# BL-048 — HyDE query mode — Hypothetical Document Embeddings

## Origin

Mengdie audit data (5 days, 21 searches) shows AE-skill queries are typically abstract: "synthesis structured output", "plan-review provider-specific schema probe", "AE plugin output standards". These are concept-level queries; direct embedding of the *question* often retrieves docs that *mention* the question's terms, not docs that *answer* the question.

HyDE: ask the LLM to write a 1-2 paragraph hypothetical answer first, embed that paragraph, search with the paragraph's vector. The hypothetical answer is in the **same vocabulary domain** as the corpus (since both are domain content, not query text), so cosine similarity is more meaningful.

LlmProvider trait + ClaudeCliProvider are already in place — zero new infra needed beyond an extra `complete()` call before vector search.

## Scope

### Pipeline

```
SearchParams { query: "what should we do about X" }
  └→ if mode == Hyde:
       └→ llm.complete(hyde_prompt(query)) → hypothetical_answer
       └→ embed(hypothetical_answer) → hyde_vector
       └→ search_vector(hyde_vector, ...)  // instead of embed(query)
       └→ optional: also run search_fts5(query) and merge via RRF — same as hybrid
  └→ else: existing path
```

### Prompt template (initial draft)

```
You are answering a question by writing what a relevant memory entry would say.

Question: {{query}}

Write 1-2 paragraphs that would directly answer this question, in the same
tone as a project conclusion/review/decision document. Be concrete and
specific. Do not hedge. Do not say "I don't know" — write the most likely
answer. This text will be embedded and used to find similar real entries
in a fact database.
```

(Refine based on dogfood; spec is the wire, not the prompt text.)

### Config

- `llm.hyde_max_tokens: u32 = 400` — bound LLM cost
- `llm.hyde_temperature: f32 = 0.3` — slight variation for query diversity, but not too creative
- Cache hyde-answer by `(query, project_id)` for a session-bounded TTL (e.g., 5 minutes) to avoid re-LLM-ing identical queries

## Acceptance criteria

1. `memory_search` with `mode: "hyde"` calls LlmProvider once per unique query, embeds the response, runs vector search
2. End-to-end latency: ≤ 5s p95 on dogfood corpus (LLM call dominates)
3. Empty / error LLM response → graceful degrade to `Hybrid` mode with `degraded: "hyde_llm_failed"` annotation
4. Benchmark on dogfood queries: HyDE precision@5 ≥ Hybrid precision@5 on at least 5 of 10 abstract queries (subjective operator eval acceptable — formal eval would require labeled data; defer)
5. Cache hits across rapid repeat queries (operator dogfood pattern)
6. Audit row records `mode: hyde` AND the hypothetical-answer text (for debug) — new column or extend `scope`

## Trigger

**Conditional on operator dogfood.** Build when:
- Operator notes ≥ 3 instances where hybrid search misses an obviously-relevant fact for an abstract query, **OR**
- Audit data shows recall_count distribution is highly concentrated (top 10% of facts collect >50% of recalls — suggesting unrecalled facts are findability-blocked, not relevance-blocked)

Until trigger: capture but don't promote. BL-048 is the only Tier C BL that needs real dogfood signal first (others are XS quality-of-life).

## Non-goals

- Multi-step HyDE chains (multi-hop hypothetical reasoning) — single round only
- Tuning per-AE-skill (different prompt per `/ae:analyze` vs `/ae:think`) — single template
- Replacing query embedding entirely — HyDE is an opt-in mode, not the default

## Cost note

Operator on flat-fee Claude Code Pro — LLM-per-query cost is amortized. If LlmProvider switches to metered (different ClaudeCliProvider impl or oMLX local model), revisit cost-benefit.
