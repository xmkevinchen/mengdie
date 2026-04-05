---
id: "001"
title: "Second Brain MVP Phase 1"
type: plan
created: 2026-04-04
status: reviewed
discussion: "docs/discussions/002-mvp-phase1/conclusion.md"
---

# Feature: Second Brain MVP Phase 1

## Goal

Build an AI-native knowledge management MCP server in Rust that ingests AE pipeline output, stores memories with hybrid search (FTS5 + vector + RRF), filters via Dreaming promotion, detects contradictions, and feeds context back into ae:analyze — completing the knowledge spiral loop.

## Source Decisions

- [002 MVP Phase 1 Conclusion](../discussions/002-mvp-phase1/conclusion.md) — scope, architecture, schema
- [003 Tech Stack Conclusion](../discussions/003-tech-stack/conclusion.md) — Rust, rusqlite, rmcp, fastembed-rs
- [001 qmd Learnings](../backlog/001-qmd-learnings.md) — RRF, score normalization, metadata-in-chunk

## Review Notes

Plan reviewed by: architect, dependency-analyst, codex-proxy, gemini-proxy.

Key changes from review:
- Contradiction detection returns conflict flags in `memory_ingest` response (not interactive prompts)
- sqlite-vec is optional/opportunistic; app-level cosine is primary vector path
- Entity extraction simplified to YAML `tags` only (skip table parsing)
- Added `load_extension` + rmcp feature flags to Cargo.toml
- fastembed inference wrapped in `spawn_blocking` (non-blocking async)
- DB connection sharing via `Arc<Mutex<Connection>>`
- AC1 clarified: model downloaded on first run (~90MB)
- Removed `axum` from Phase 1 deps (Phase 2)
- Added AE conclusion.md template `entities` field change
- Step 5 split into 5a (parser + watcher) and 5b (contradiction detection)

---

## Steps

### Step 1: Project scaffold + SQLite core (AC1, AC2)

- [x] Initialize Cargo project with `Cargo.toml` (78c59fd)
- [x] Create directory structure: `src/core/`, `src/bin/mcp_server.rs`, `src/bin/cli.rs`
- [x] SQLite schema with migration (memory_entries + FTS5 + metrics)
- [x] WAL mode + `busy_timeout(5000)`
- [x] DB connection wrapper: `Arc<Mutex<Connection>>`
- [x] Core DB module: open/create, migrations, CRUD, recall tracking, promotion
- [x] `project_id` inference: git remote (fallback: FNV-1a path hash)
- [x] tracing subscriber → stderr

Expected files: `Cargo.toml`, `src/lib.rs`, `src/core/mod.rs`, `src/core/db.rs`, `src/core/schema.rs`, `src/core/project.rs`, `src/bin/mcp_server.rs` (stub), `src/bin/cli.rs` (stub)

### Step 2: Embedding + vector search (AC3, AC4)

- [ ] fastembed-rs integration: model loading (all-MiniLM-L6-v2, 384 dimensions, ~90MB downloaded on first run)
- [ ] `text_to_embedding(text) -> Vec<f32>` — wrapped in `tokio::task::spawn_blocking` (fastembed is sync/blocking)
- [ ] Metadata-in-chunk encoding at embedding time: prepend `[{knowledge_type}] [entities: {entities}] [project: {project_id}]\nTitle: {title}\n---\n` to content before embedding (from qmd learnings)
- [ ] Float32 blob storage: IEEE 754 little-endian, dimension count stored in `embedding_dim` column
- [ ] Primary vector search: app-level cosine similarity over float32 blobs (brute-force scan with project_id filter)
- [ ] Optional sqlite-vec: attempt `load_extension` at startup; if available, use vec0 virtual table for indexed search; if unavailable, log warning and use cosine fallback silently
- [ ] Validate `embedding_dim` matches model dimension on upsert
- [ ] Unit tests: embed → store → retrieve by similarity → top result is semantically closest

Expected files: `src/core/embeddings.rs`, `src/core/vector.rs`

### Step 3: Hybrid search + RRF (AC5, AC6)

- [ ] FTS5 search: `search_fts(query, project_id, limit) -> Vec<(id, bm25_score)>`
- [ ] Vector search: `search_vector(query_embedding, project_id, limit) -> Vec<(id, cosine_score)>`
- [ ] Score normalization: BM25 → 0-1 (min-max within result set), cosine → 0-1 via `1/(1+distance)`
- [ ] Reciprocal Rank Fusion: `rrf_merge(fts_results, vec_results, k=60) -> Vec<(id, rrf_score)>`
- [ ] `memory_search(query, scope?) -> Vec<SearchResult>` — combines FTS5 + vector + RRF
- [ ] Default scope = current project; `scope: "global"` parameter accepted but searches all projects
- [ ] On each search hit: increment `recall_count`, update `avg_relevance` (running average with new score), update `last_recalled`
- [ ] Filter out memories with `valid_until < now` (expired) from results

Expected files: `src/core/search.rs`

### Step 4: MCP server — 3 tools (AC7, AC8)

- [ ] rmcp stdio server setup with tokio runtime (`#[tool_router]` macro)
- [ ] `memory_search` tool: input `{query: string, scope?: string}`, returns `Vec<{title, source_file, knowledge_type, entities, score, valid_from, snippet}>`
- [ ] `memory_ingest` tool: input `{title, content, source_file, source_type, knowledge_type, entities}`, generates embedding (spawn_blocking), stores, runs contradiction check, returns `{entry_id, conflicts: Vec<{id, title, reason}>}` — caller decides what to do with conflicts (no interactive prompt)
- [ ] `memory_invalidate` tool: input `{entry_id: string, reason: string, superseded_by?: string}`, sets `valid_until = now` + `superseded_by`
- [ ] All logging via tracing → stderr (zero stdout pollution)
- [ ] Structured MCP error responses, never panic
- [ ] Smoke test: start binary, send JSON-RPC via stdin, verify response on stdout

Expected files: `src/bin/mcp_server.rs`, `src/core/mcp_tools.rs`

### Step 5a: Ingestion pipeline + file watcher (AC9, AC10)

- [ ] File parser: extract YAML frontmatter (serde_yaml) + body content from AE output files
- [ ] Entity extraction: from frontmatter `tags` field only (skip table parsing for MVP)
- [ ] Knowledge type: infer from source_type (conclusion/plan → decisional, review/retrospect → experiential)
- [ ] AE file watcher (notify crate): watch configured directories for `conclusion.md`, `review.md`, `plan.md`, `retrospect.md` patterns
- [ ] Debounce: 500ms after last file event before processing (handles rapid saves)
- [ ] Watcher → parser → embedding → store pipeline (on file create/modify)
- [ ] Integration test: write a file → watcher triggers → memory stored → searchable

Expected files: `src/core/parser.rs`, `src/core/watcher.rs`, `src/core/ingest.rs`

### Step 5b: Contradiction detection (AC11)

- [ ] At ingestion time: query existing memories with overlapping entities + same knowledge_type
- [ ] Entity overlap: ≥1 shared entity tag between new and existing memory
- [ ] Semantic similarity: cosine similarity > 0.7 between embeddings
- [ ] If entity overlap + high similarity + knowledge_type == "decisional": flag as "evolution candidate"
- [ ] If entity overlap + time gap <30 days: flag as "conflict"
- [ ] Return conflict info in `memory_ingest` response (not interactive — caller decides)
- [ ] When `memory_invalidate` is called with `superseded_by`, link the chain

Expected files: `src/core/contradiction.rs`

### Step 6: Dreaming + CLI (AC12, AC13)

- [ ] Simplified Dreaming: query memories where `recall_count >= 3 AND avg_relevance >= 0.65` and `last_recalled` within 14-day window → set `is_longterm = true` (boosted weight in search)
- [ ] CLI entry point (`src/bin/cli.rs`) with clap subcommands:
  - `second-brain dream` — run Dreaming pass, print promoted/unchanged counts
  - `second-brain import --dir <path>` — batch import (scan for conclusion.md + review.md, parse, embed, store with `recall_count = 0`)
  - `second-brain search <query>` — run memory_search, print results (debugging)
  - `second-brain stats` — print observability metrics
- [ ] CLI accepts `--db-path` override (default: `~/.second-brain/db.sqlite`)
- [ ] macOS launchd plist template for daily Dreaming cron

Expected files: `src/bin/cli.rs`, `src/core/dreaming.rs`, `resources/com.second-brain.dream.plist`

### Step 7: ae:analyze integration + observability (AC14, AC15)

- [ ] Modify ae:analyze SKILL.md: add Step 3.5 "Prior Context (from Second Brain)" — after research phase, before synthesis, call `memory_search` with research topic, present as "Round 0: Prior Decisions" with provenance
- [ ] Modify AE conclusion.md template: add `entities: [...]` frontmatter field (002 Topic 3 requirement)
- [ ] Locate ae:analyze SKILL.md path (in AE plugin directory)
- [ ] 4 observability metrics in SQLite `metrics` table:
  - `context_injection_rate`: % of memory_search returning non-empty (target: >60%)
  - `stale_citation_rate`: % of cited memories with expired valid_until (target: <10%)
  - `conflict_detection_rate`: % of ingestions triggering conflict flags (target: 1-15%)
  - `memory_age_at_retrieval`: average age of cited memories
- [ ] Increment metric counters in search/ingest code paths
- [ ] End-to-end smoke test: create test conclusion.md → watcher ingests → search returns it → Dreaming processes it

Expected files: `src/core/metrics.rs`, `tests/e2e.rs`, ae:analyze SKILL.md (external), AE conclusion.md template (external)

---

## Acceptance Criteria

### AC1: Project Builds and Runs
`cargo build --release` succeeds. Binary starts in <50ms. `cargo test` passes. Single binary; embedding model (~90MB) downloaded on first run and cached at `~/.cache/fastembed/`.

### AC2: SQLite Schema Correct
DB created at `~/.second-brain/db.sqlite`. All columns present in memory_entries including `is_longterm` and `embedding_dim`. FTS5 index on title+content+entities. WAL mode active. `project_id` correctly inferred from git remote.

### AC3: Embeddings Generate Correctly
`text_to_embedding("test query")` returns Vec<f32> of dimension 384. Embedding stored as IEEE 754 LE float32 blob with `embedding_dim = 384`. Round-trip: embed → store → load → cosine similarity with self ≈ 1.0.

### AC4: Vector Search Works
App-level cosine search returns correct results. Upsert 10 memories, query with related text, top result is semantically closest. If sqlite-vec extension is available, it produces equivalent results. If unavailable, cosine fallback activates with a log warning (no error/panic).

### AC5: Hybrid Search Returns Relevant Results
Insert 5 test memories with known content. FTS5 search for exact keyword returns correct memory. Vector search for semantically similar query returns correct memory. RRF merge produces single ranked list with scores in 0-1 range.

### AC6: RRF Improves Over Single Ranker
Test: memory A matches keyword only (high BM25, low vector), memory B matches meaning only (low BM25, high vector). RRF list includes both in top-3. FTS-only misses B; vector-only misses A.

### AC7: MCP Server Responds to All 3 Tools
Start MCP server via stdio. `memory_search` returns results. `memory_ingest` returns entry_id + conflict list (empty if no conflicts). `memory_invalidate` sets valid_until. Zero non-JSON-RPC bytes on stdout.

### AC8: MCP Server Works in Claude Code
Config `{"command": "/path/to/second-brain", "args": ["mcp"]}` in settings.json. Claude Code connects, lists 3 tools, executes a search query successfully.

### AC9: File Watcher Detects AE Output
Create test `conclusion.md` in watched directory. Watcher detects within 2 seconds. Parser extracts frontmatter tags as entities, infers knowledge_type. Memory stored in DB with correct fields.

### AC10: Ingestion Pipeline End-to-End
Write conclusion.md with known tags. Watcher triggers → parser extracts entities from tags → embedding generated (metadata-in-chunk encoded) → stored in DB → searchable via memory_search.

### AC11: Contradiction Detection Flags Conflicts
Insert memory A (decisional, entities: [auth]). Insert memory B (decisional, entities: [auth], different content, <30 days). `memory_ingest` response for B includes conflict flag referencing A. After `memory_invalidate(A, superseded_by=B)`, A has valid_until set.

### AC12: Dreaming Promotes Memories
Insert memory with recall_count=5, avg_relevance=0.8, last_recalled within 14 days. Run `second-brain dream`. Memory has `is_longterm = true`. Insert memory with recall_count=1, avg_relevance=0.3. Dream does NOT promote it.

### AC13: Batch Import Works
Create directory with 3 test conclusion.md files. Run `second-brain import --dir <path>`. All 3 imported with recall_count=0. Searchable via memory_search.

### AC14: ae:analyze Injection Works
ae:analyze SKILL.md modified. MCP server registered. Run ae:analyze — output includes "Round 0: Prior Decisions" section with memories and provenance (source_file, knowledge_type).

### AC15: Observability Metrics Tracked
After searches and ingestions, `second-brain stats` reports all 4 metrics. context_injection_rate calculable from data.
