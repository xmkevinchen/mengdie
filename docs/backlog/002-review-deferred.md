---
id: "002"
title: "Deferred review findings"
status: open
created: 2026-04-05
tags: [review, deferred, performance, architecture]
---

# Deferred Review Findings

## From Step 1 Review

### BL-002-1: std::sync::Mutex blocks async executor
- **Source**: Doodlestein + Codex (Step 1 + Step 2 reviews)
- **Issue**: `Arc<std::sync::Mutex<Connection>>` blocks tokio executor threads during DB operations. At scale, concurrent MCP requests will serialize on the lock.
- **Fix**: Fetch rows into local Vec, drop lock, then compute outside. Or switch to `tokio::sync::Mutex` / connection pool.
- **Trigger**: When MCP server handles concurrent requests (Phase 2 daemon mode).

## From Step 2 Review

### BL-002-2: Metadata-in-chunk query asymmetry
- **Source**: Doodlestein (Step 2 review)
- **Issue**: Documents get metadata prepended before embedding (`[decisional] [entities: auth]...`) but queries do not. This creates an asymmetry in the embedding space — doc embeddings are "colored" by metadata while query embeddings are plain text.
- **Current rationale**: By design — queries match semantic content; docs are enriched. qmd uses the same pattern.
- **Revisit trigger**: If search quality tests show metadata-enriched docs rank lower than expected for plain-text queries, consider also prepending metadata to query embeddings.

### BL-002-3: No score threshold in vector search / hardcoded limit
- **Source**: Doodlestein (Step 2 review)
- **Issue**: `search_vector` returns all results sorted by similarity, including low-relevance noise (e.g., score 0.1).
- **Fix**: Add `min_score: Option<f32>` parameter to filter results below threshold. Also add `limit` parameter to MCP tool (currently hardcoded to 10).
- **Trigger**: When search results include clearly irrelevant entries in practice.

## From Steps 3+4 Review

### BL-002-7: Contradiction full table scan at scale
- **Source**: Code-reviewer (Step 5b review)
- **Issue**: `check_contradictions` scans all valid memories per project with no index on entity overlap. O(n) per ingest.
- **Current**: Acceptable for MVP (1K-10K memories).
- **Fix**: Add entity index or pre-filter query when scan exceeds ~50ms.
- **Trigger**: Ingestion latency exceeds 100ms per file.

### BL-002-6: Contradiction detection magic numbers
- **Source**: Doodlestein (Step 5b review)
- **Issue**: Cosine thresholds (0.7 evolution, 0.4 recent) and 30-day conflict window are hardcoded. Active projects with common entity tags (auth, api, database) may see false positives.
- **Fix**: Make thresholds configurable via Db config or pipeline.yml.
- **Trigger**: When users report noise in contradiction flags.

### BL-002-5: Manual debounce in watcher should use notify_debouncer_mini
- **Source**: Doodlestein + Code-reviewer (Step 5a review)
- **Issue**: Hand-rolled debounce in `watch_loop` has subtle timing bugs (500ms window resets per-batch, not per-path). `notify_debouncer_mini` is already a dependency but unused.
- **Fix**: Replace manual debounce with `notify_debouncer_mini::new_debouncer()`.
- **Trigger**: When watcher is integrated into MCP server or CLI daemon (Step 6+).

### BL-002-4: FTS5 syntax abuse not sanitized
- **Source**: Codex (Steps 3-4 review)
- **Issue**: User query passed directly to FTS5 MATCH — FTS5 operators (OR, NOT, NEAR, wildcards) can alter semantics or trigger expensive queries.
- **Current rationale**: Queries come from trusted AI agents (ae:analyze), not end users.
- **Revisit trigger**: If Second Brain is exposed to user-facing queries or untrusted input.
