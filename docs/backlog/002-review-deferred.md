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

### BL-002-4: FTS5 syntax abuse not sanitized
- **Source**: Codex (Steps 3-4 review)
- **Issue**: User query passed directly to FTS5 MATCH — FTS5 operators (OR, NOT, NEAR, wildcards) can alter semantics or trigger expensive queries.
- **Current rationale**: Queries come from trusted AI agents (ae:analyze), not end users.
- **Revisit trigger**: If Second Brain is exposed to user-facing queries or untrusted input.
