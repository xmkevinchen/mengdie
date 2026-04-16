---
id: "001"
title: "Second Brain MVP Phase 1"
type: plan
created: 2026-04-04
status: done
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

- [x] fastembed-rs integration: Embedder struct (all-MiniLM-L6-v2, 384d) (a31b997)
- [x] `embed_text` / `embed_with_context` (metadata-in-chunk encoding from qmd)
- [x] Float32 blob storage: IEEE 754 LE, embedding_dim validation
- [x] Cosine similarity: brute-force scan with project_id filter + expired exclusion
- [x] sqlite-vec deferred (cosine primary path sufficient for MVP)
- [x] 11 new unit tests (blob roundtrip, cosine math, vector search, project filter, expiry)

Expected files: `src/core/embeddings.rs`, `src/core/vector.rs`

### Step 3: Hybrid search + RRF (AC5, AC6)

- [x] FTS5 search + vector search + RRF merge (k=60) + score normalization (83b6468)
- [x] memory_search: hybrid FTS5+vector+RRF, project filter, global scope, recall tracking
- [x] Expired entries filtered (valid_until > now), recall stats updated per hit

Expected files: `src/core/search.rs`

### Step 4: MCP server — 3 tools (AC7, AC8)

- [x] rmcp stdio server + ServerHandler + tool_router macro (1541089)
- [x] memory_search: hybrid FTS5+vector+RRF, spawn_blocking for embedding
- [x] memory_ingest: metadata-in-chunk encoding, conflict placeholder for Step 5b
- [x] memory_invalidate: valid_until + superseded_by
- [x] tracing → stderr, graceful error handling (no panics)

Expected files: `src/bin/mcp_server.rs`, `src/core/mcp_tools.rs`

### Step 5a: Ingestion pipeline + file watcher (AC9, AC10)

- [x] File parser: YAML frontmatter + body, entity extraction from tags, knowledge_type inference (c7b3f09)
- [x] AE file watcher (notify crate), is_ingestable pattern matching (excludes swap files)
- [x] Ingest pipeline: parse → embed (metadata-in-chunk) → store
- [x] UNIQUE(project_id, source_file) prevents duplicate ingestion

Expected files: `src/core/parser.rs`, `src/core/watcher.rs`, `src/core/ingest.rs`

### Step 5b: Contradiction detection (AC11)

- [x] Contradiction detection: entity overlap + cosine checks, runs before insert (998633d)
- [x] EvolutionCandidate (cosine > 0.7 + both decisional) + RecentConflict (<30d + cosine > 0.4)
- [x] Conflicts returned in memory_ingest response; invalidate_memory links superseded_by chain

Expected files: `src/core/contradiction.rs`

### Step 6: Dreaming + CLI (AC12, AC13)

- [x] Dreaming: recall_count >= 3, avg_relevance >= 0.65, 14-day window → is_longterm (67cc2b8)
- [x] CLI: dream, import --dir (project_id from dir), search, stats, --db-path override
- [x] Import: recursive walk, duplicate detection via rusqlite error type
- [x] launchd plist template (resources/com.second-brain.dream.plist)

Expected files: `src/bin/cli.rs`, `src/core/dreaming.rs`, `resources/com.second-brain.dream.plist`

### Step 7: ae:analyze integration + observability (AC14, AC15)

- [x] AE integration PRD: docs/prd-ae-integration.md (why, what, how, interface spec) (b5cd944)
- [x] 4 metrics tracked: search_count, search_nonempty, ingest_count, conflict_count
- [x] Metric counters wired into MCP search + ingest paths
- [x] list_metrics method for dashboard enumeration
- [x] E2e test: ingest → search → recall → dream → contradiction (full pipeline)

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
