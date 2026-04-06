---
id: "008"
title: "Analysis: Contradiction Detection Strategies"
type: analysis
created: 2026-04-05
tags: [contradiction, entity-tag, temporal-validity, knowledge-evolution, supersession]
---

# Analysis: Contradiction Detection Strategies

## Question

How effective is mengdie's entity-tag directed comparison + temporal validity approach to contradiction detection? What are the gaps, and what's the minimal viable improvement path?

## Findings

### Prior Art from Project Knowledge Base

- **[analyze]: No embedding model provenance in DB schema** (factual, `docs/discussions/007-embedding-model-tradeoffs/analysis.md`): Schema stores `embedding_dim` but not model name. Relevant because contradiction detection's cosine thresholds depend on embedding model consistency.
- **[analyze]: Mengdie hybrid search (RRF k=60) correctly implemented** (factual, `docs/discussions/005-hybrid-search-analysis/analysis.md`): RRF merges by rank, not score. Cosine similarity thresholds in contradiction detection operate on the same embedding space.
- **MVP Phase 1 Conclusion** (decisional, `docs/discussions/002-mvp-phase1/conclusion.md`): Contradiction detection returns conflict flags in `memory_ingest` response (not interactive prompts).

### Relevant Code

- **`src/core/contradiction.rs:10-14`**: Hardcoded thresholds — `EVOLUTION_SIMILARITY_THRESHOLD = 0.7`, `RECENT_CONFLICT_SIMILARITY_FLOOR = 0.4`, `RECENT_CONFLICT_DAYS = 30`.
- **`src/core/contradiction.rs:64-81`**: Full table scan — `SELECT * FROM memory_entries WHERE project_id = ? AND valid_until IS NULL` with app-side entity matching.
- **`src/core/contradiction.rs:89-99`**: Entity overlap — comma-split, lowercase trim, `any()` for ≥1 shared tag.
- **`src/core/contradiction.rs:102-117`**: EvolutionCandidate — triple gate: both `decisional` AND entity overlap AND cosine > 0.7.
- **`src/core/contradiction.rs:121-144`**: RecentConflict — entity overlap AND `created_at` < 30 days AND cosine > 0.4.
- **`src/core/schema.rs:29-31`**: `valid_from TEXT NOT NULL`, `valid_until TEXT`, `superseded_by TEXT`.
- **`src/core/ingest.rs:39-44`**: Contradiction check before insert, errors degrade silently to empty vec.
- **`src/core/mcp_tools.rs:88-102`**: `IngestOutput` returns `conflicts: Vec<ConflictItem>` with id, title, reason.
- **Tests**: 6 unit tests + 1 ignored e2e test covering the main paths.

### Architecture & Patterns

**What's implemented:**
- Two detection strategies (EvolutionCandidate, RecentConflict) with entity overlap as candidate filter + cosine similarity as quality gate
- Non-blocking, advisory-only — ingestion always proceeds, conflicts returned to caller
- Invalidated entries (`valid_until IS NOT NULL`) excluded from checks
- `superseded_by` column exists but no workflow uses it

**What's NOT implemented (despite being in the design):**
- `valid_from` is stored but **never read** in any query or contradiction check — dead storage
- No supersession workflow — no MCP tool to resolve conflicts (confirm new supersedes old)
- No feedback loop — no way to record whether a flagged conflict was a true positive
- No entity index — full table scan per ingest

**Detection effectiveness assessment:**

The EvolutionCandidate triple gate (both `decisional` + entity overlap + cosine > 0.7) is extremely restrictive. At cosine > 0.7 with all-MiniLM-L6-v2 384d embeddings, the memories are near-identical — likely duplicates, not contradictions. The gate rarely fires, and when it does, it's probably catching dedup failures rather than genuine decision evolution.

RecentConflict (entity overlap + 30 days + cosine > 0.4) is broader but the entity overlap filter has a high false positive rate for common domain tags ("auth", "database", "api"). The 0.4 cosine floor is the only defense against noise from broad tags.

### Industry Practice Comparison

**Production AI memory systems:**
- **Zep/Graphiti**: Explicit temporal facts (`valid_at`, `invalid_at`) + invalidation (not deletion). Most directly comparable to mengdie's approach.
- **MemGPT/Letta**: "Last write wins" with version history. No explicit contradiction detection.
- **LangChain**: Memory stores with no built-in contradiction engine.
- **Wikidata/RDF**: Keep all statements with ranks and deprecation. Never delete — mark with `validFrom/validTo`.
- **Neo4j**: Uses BOTH temporal validity AND supersedes lineage.

**Key finding**: No mainstream AI tool (Mem, Notion AI, Obsidian) ships automated contradiction detection in 2025. Mengdie is ahead of the product curve — but also in uncharted territory with no reference implementations to benchmark against.

**Standard approach (knowledge graphs):** Same head entity + same relation + conflicting value = contradiction. Mengdie's entity-tag overlap is a weaker proxy — tags are not canonical entities, so "auth" matches too broadly while "PostgreSQL" vs "postgres" won't match at all.

**Recommended upgrade path** (industry consensus):
1. Similarity as candidate retrieval filter (current approach — correct)
2. NLI classifier for actual entailment/contradiction classification on candidates (not implemented)
3. Explicit supersession workflow for resolution (schema exists, workflow missing)

### Challenges & Disagreements

**Challenger's core thesis: contradiction detection is premature and delivers no actionable value at MVP scale.**

Key challenges:
1. **`valid_from` is dead code** — stored but never read. The temporal validity design described in docs is not implemented in code.
2. **EvolutionCandidate triple gate fires on duplicates, not contradictions** — cosine > 0.7 means near-identical content. If they're both `decisional` with the same entities and very similar text, they're likely the same decision from the same source, not an evolved stance.
3. **Entity-tag overlap is a noisy signal** — comma-separated strings, no index, no fuzzy matching. Correctness depends on tag naming discipline that AI-generated tags won't maintain consistently.
4. **Silent error degradation** — if embedding model unavailable (first run), contradiction check silently returns empty vec. User sees "no conflicts" when actually "conflicts not checked."
5. **Advisory-only with no resolution workflow** — conflict is flagged, then what? No `memory_resolve_conflict` tool, no supersession automation.

**Standards-expert's counter:**
- Advisory-only is the correct pattern (auto-resolution would be worse)
- The detection logic is directionally correct and aligned with industry patterns
- The real gap is the **resolution workflow**, not the detection logic
- A DB index on `(project_id, valid_until)` trivially fixes the scan cost
- "Theater" is too strong — the loop from detection to resolution isn't closed, but the detection itself is reasonable

**Cross-family (Codex):**
- Zep/Graphiti and Neo4j validate temporal validity + supersedes as a proven pattern
- Recommended normalizing to claim records `(subject, relation, object)` for more precise matching
- No open-source contradiction detection logic found in MemGPT/Zep/Graphiti — this remains proprietary
- Mengdie's transparent approach is a differentiator

**Consensus:** Detection logic is directionally correct but the value chain is incomplete — detection fires → user sees conflict → nothing happens. The fix is closing the resolution loop, not reworking the detection algorithm.

## Summary

**Mengdie's contradiction detection is architecturally sound but operationally incomplete.** The two-stage approach (entity filter → semantic similarity) matches industry patterns. The non-blocking, advisory-only design is correct. The schema has the right fields (`valid_until`, `superseded_by`).

**Three structural gaps:**

| Gap | Impact | Fix |
|---|---|---|
| No resolution workflow | Conflicts flagged but never acted on — incomplete value chain | Add `memory_resolve_conflict` MCP tool that sets `valid_until` on old + `superseded_by` on new |
| `valid_from` dead code | Design/implementation mismatch; temporal validity is unimplemented | Either use it in queries or remove it from the "temporal validity" narrative |
| Entity tag quality | Broad tags cause false positives; no canonical entity resolution | Short-term: tag specificity guidance for AE extractors. Long-term: structured entity extraction |

**Two concrete issues to fix now:**

1. **Silent degradation on embedding failure** — contradiction check returns empty vec when embedder unavailable. Should log clearly: "contradiction check skipped (embedder unavailable)" in the MCP response, not silently pass.
2. **DB index** — `CREATE INDEX idx_contradiction_scan ON memory_entries(project_id, valid_until)` — one-line hygiene fix.

**Backlog items:**

| Item | Trigger | Action |
|---|---|---|
| Calibrate thresholds empirically | 100+ ingested memories with real AE output | Measure FP rate, adjust EVOLUTION_SIMILARITY_THRESHOLD and RECENT_CONFLICT_SIMILARITY_FLOOR |
| NLI classifier for contradiction classification | FP rate > 30% after threshold calibration | Add cross-encoder/nli-deberta-v3-small as second stage after similarity filter |
| Claim normalization | If entity-tag approach hits quality ceiling | Normalize to `(subject, relation, object)` tuples per Codex recommendation |

## Possible Next Steps

- If resolution workflow is prioritized → `/ae:discuss` the `memory_resolve_conflict` tool design
- If detection quality needs validation → run contradiction detection on the growing corpus from these 5 analyses and measure FP rate
- Otherwise → backlog the gaps and proceed with other Phase 1.1 work
