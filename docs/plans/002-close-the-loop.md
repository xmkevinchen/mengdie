---
id: "002"
title: "Close the Knowledge Loop — Mengdie + AE Integration"
type: plan
created: 2026-04-05
status: reviewed
discussion: ""
---

# Feature: Close the Knowledge Loop — Mengdie + AE Integration

## Goal

Close the knowledge spiral: AE skills produce knowledge → automatically ingested into Mengdie → surfaced as prior context in future AE sessions. Currently the write side is completely disconnected and the DB is empty. This plan patches Mengdie prerequisites, wires AE read+write integration, and validates the loop end-to-end.

## Cross-Repo Scope

- **mengdie** (`/Users/ckai/Workspace/Projects/mengdie`): Steps 1-2 (patches)
- **agentic-engineering-mengdie** (`/Users/ckai/Workspace/Projects/agentic-engineering-mengdie`): Steps 3-5 (AE integration)
- **Both**: Step 6 (validation)

## Source Decisions

- [004 MVP Assessment](../discussions/004-mvp-assessment/analysis.md) — L1 (loop not closed), L3 (source_file fragile), L4 (no list/dry-run)
- [AE PRD](agentic-engineering-mengdie: docs/prd/mengdie-integration.md) — phased integration map, extraction rules, graceful degradation
- [Backlog 002](../backlog/002-review-deferred.md) — 002-12 (score normalization), 002-13 (check+insert not transactional)

## Review Notes

Plan reviewed by: architect, dependency-analyst, codex-proxy (Codex), gemini-proxy (Gemini).

Key changes from review:
- Step 1: explicit version-2 schema migration (ADD COLUMN, DROP old index, CREATE new index), use `INSERT ... ON CONFLICT(project_id, content_hash) DO UPDATE` for atomic dedup
- Step 1: migration must backfill `content_hash = sha256(content)` for any existing rows (safety net even if DB expected empty)
- Step 1: `source_file` uses `#[serde(default)]` on `String` (not `Option<String>`) to avoid schemars nullable type issues; column stays `NOT NULL` with empty string default
- Step 1: normalize `SearchResult.score` to 0.0-1.0 in returned results (resolves backlog 002-12, enables `min_score` and quality gates)
- Steps 1→2 must be sequential (both touch `db.rs`); Steps 4+5 can be parallel (different SKILL.md files)
- Step 3: explicit binary rebuild prerequisite added
- Steps 4-5: extraction heuristics reference shared Knowledge Capture Protocol (avoid duplication across skills)
- Step 6: expanded to include Phase A gate (5+ analyze sessions) and conflict detection test; validation artifact added
- AC8: clarified output contract for both available and unavailable cases
- AC9: specified conflict logging format per PRD
- AC10: quantified gate condition (3/5 sessions must reference prior findings)

Doodlestein challenges (all accepted):
- Strategic: normalize search scores to 0-1 in returned results — one-line fix enabling quality gates across Steps 3-6
- Adversarial: migration must backfill content_hash; use `#[serde(default)]` instead of `Option<String>` for source_file; verify FTS5 trigger behavior with UPSERT
- Regret: extraction-on-caller-side will likely consolidate into shared reference — mitigate now by creating a Knowledge Capture Protocol reference doc instead of inline heuristics in each SKILL.md

## Execution Order

Steps 1→2 sequential (shared `db.rs`). Steps 4+5 parallel (different files). Full chain: **1 → 2 → 3 → {4, 5} → 6**

---

## Steps

### Step 1: Mengdie prerequisites — content hash dedup + source_file optional + score normalization (AC1, AC2)

- [x] Version-2 schema migration in `schema.rs`: `ALTER TABLE memory_entries ADD COLUMN content_hash TEXT` (299b4e6)
- [x] In migration: `DROP INDEX IF EXISTS idx_memory_source` then `CREATE UNIQUE INDEX idx_memory_content_hash ON memory_entries(project_id, content_hash)` (299b4e6)
- [x] In migration: backfill existing rows via Rust loop with `compute_content_hash()` (299b4e6)
- [x] Compute SHA-256 of `content` at insert time, store in `content_hash` (299b4e6)
- [x] Replace `insert_memory` dedup logic: `INSERT ... ON CONFLICT(project_id, content_hash) DO UPDATE ... RETURNING id` (299b4e6)
- [x] Make `source_file` optional in `IngestParams`: `#[serde(default)] pub source_file: String` (299b4e6)
- [x] Normalize `SearchResult.score` to 0.0-1.0 in `memory_search` return values (resolves backlog 002-12) (299b4e6)
- [x] Update existing tests to reflect new dedup behavior + 5 new tests (299b4e6)
- [x] Verify FTS5 triggers fire correctly with upsert path: `test_content_hash_upsert_fts5_sync` (299b4e6)

Expected files: `src/core/schema.rs`, `src/core/db.rs`, `src/core/search.rs`, `src/core/mcp_tools.rs`, `src/core/ingest.rs`, `tests/e2e.rs`

### Step 2: Mengdie CLI improvements — list + dry-run (AC3)

Prerequisite: Step 1 complete (shared `db.rs` changes settled).

- [x] Add `list_memories` query to `db.rs`: return all entries for a project (or all projects with `--global`) (3b1690e)
- [x] Add `mengdie list` subcommand: show id, title, knowledge_type, source_file, recall_count, is_longterm (3b1690e)
- [x] Add `--global` flag to `list` for cross-project view (3b1690e)
- [x] Add `--dry-run` flag to `mengdie import`: scan and print what would be imported/skipped without writing (3b1690e)
- [x] Add `--format` flag to `list` with `table` (default) and `json` options (3b1690e)

Expected files: `src/bin/cli.rs`, `src/core/db.rs` (add `list_memories` query)

### Step 3: Register Mengdie MCP in AE + validate search quality (AC4, AC5)

Prerequisite: Steps 1-2 complete. **Rebuild binary**: `cargo build --release` in mengdie repo and ensure `mengdie-mcp` is on PATH (via `cargo install --path .` or absolute path in `.mcp.json`).

- [x] Mengdie MCP already registered in `~/.claude/.mcp.json` (global); AE subagents inherit — no `.mcp.json` change needed
- [x] Imported 26 memories: 2 from mengdie discussions + 24 from AE `.ae/discussions/`
- [x] 5 search queries validated: 5/5 correct top-1, all scores 0.48-0.50 (normalized)
- [x] Results documented inline (search quality log in commit message)

Expected files (AE repo): `plugins/ae/.mcp.json`

### Step 4: ae:analyze write integration — Knowledge Capture step (AC6, AC7)

Prerequisite: Step 3 complete (MCP registered, search quality validated).

- [x] Create shared Knowledge Capture Protocol reference at `plugins/ae/docs/knowledge-capture-protocol.md` (1d58856)
- [x] Add Step 4.5 "Knowledge Capture (to Mengdie)" after TL synthesis in ae:analyze SKILL.md (1d58856)
- [x] Skill-specific extraction rule: one item per key finding, skip prior art restatements, factual type, entities from tags (1d58856)

Expected files (AE repo): `plugins/ae/docs/knowledge-capture-protocol.md`, `plugins/ae/skills/analyze/SKILL.md`

### Step 5: ae:discuss read + write integration (AC8, AC9)

Prerequisite: Step 3 complete. Can run in parallel with Step 4 (different SKILL.md file).

**Read (Prior Context):**
- [x] Add Step 1.5 Prior Context after setup, before team spawn — same pattern as ae:analyze Step 3.5 (1d58856)
- [x] Provenance display: title, source_file, knowledge_type, valid_from, snippet (1d58856)
- [x] Graceful degradation: "Prior context: unavailable" on failure/no results (1d58856)

**Write (Knowledge Capture):**
- [x] Add Step 9.5 Knowledge Capture after conclusion, before team shutdown — references shared protocol (1d58856)
- [x] Skill-specific: one item per resolved decision, include rationale, decisional type, entities from conclusion (1d58856)

Expected files (AE repo): `plugins/ae/skills/discuss/SKILL.md`

### Step 6: End-to-end loop validation (AC10)

Prerequisite: Steps 4-5 complete.

**Phase A gate (analyze write→read loop):**
- [x] Run 5+ ae:analyze sessions with write enabled on different topics (006-010: SQLite concurrency, embedding models, contradiction detection, dreaming/promotion, cross-project sharing)
- [x] For each subsequent session, verify Step 3.5 surfaces findings from prior sessions (sessions 2-5 all surfaced prior findings; session 5 surfaced 5 results from 4 prior analyses)
- [x] Gate: at least 3/5 sessions must show AI referencing prior findings in synthesis (4/5 sessions referenced prior findings — PASS)
- [x] If gate fails: stop, reassess extraction quality and search relevance before Phase B (gate passed; 14 memories ingested, recall_count up to 5 on earliest finding)

**Phase B validation (discuss→analyze cross-loop):**
- [ ] Run ae:discuss on a real topic → verify resolved decisions ingested into Mengdie (check via `mengdie list`)
- [ ] Run ae:analyze on a related topic → verify prior discussion decisions surface in Step 3.5
- [ ] Confirm the loop: discuss writes → analyze reads → knowledge spiral demonstrated

**Conflict detection test:**
- [ ] Seed a memory with overlapping content, run ae:discuss that produces a conflicting decision
- [ ] Verify `memory_ingest` returns conflicts and they appear in skill output footer

**Record validation results** in `docs/discussions/005-loop-validation/validation.md`:
- Per-session: what was ingested, what was surfaced, AI utilization (cited/ignored)
- Aggregate: pass rate, search quality, conflict detection behavior

Expected files: `docs/discussions/005-loop-validation/validation.md`

---

## Acceptance Criteria

### AC1: Content Hash Dedup
Insert memory A with content "foo". Insert memory B with same content "foo" but different `source_file`. B upserts over A (same `content_hash`). Insert memory C with different content → creates new row. Dedup uses `INSERT ... ON CONFLICT DO UPDATE` (atomic, no TOCTOU). `cargo test` passes.

### AC2: Source File Optional
Call `memory_ingest` MCP tool without `source_file` field. Ingestion succeeds. Entry stored with empty string source_file. No error, no panic. Content hash dedup still works without source_file.

### AC3: CLI List + Dry-Run
`mengdie list` shows all memories for current project with id, title, type, source, recall count. `mengdie import --dir <path> --dry-run` prints files that would be imported and files that would be skipped, without modifying the database. DB state unchanged after dry-run (verify with `mengdie stats`).

### AC4: MCP Registration
ae:analyze Step 3.5 successfully calls `memory_search` when Mengdie DB has data. Results appear under "Prior Art from Project Knowledge Base" with provenance. When mengdie-mcp binary is not available, skill continues with "Prior context: unavailable".

### AC5: Search Quality
Import 10+ real AE conclusions. Run 5 search queries on known decision topics. At least 4/5 queries return the correct conclusion in top-3 results. Document: query, expected result, actual top-3, pass/fail.

### AC6: ae:analyze Writes to Mengdie
After ae:analyze completes, 1-3 new memories exist in Mengdie DB matching the analysis findings. Each memory has: title format `[analyze]: [finding]`, `source_type=conclusion`, `knowledge_type=factual`, non-empty entities. No prior-art restatements ingested. Verify via `mengdie list`.

### AC7: Write Graceful Degradation
Disconnect Mengdie MCP (remove from .mcp.json). Run ae:analyze. Skill completes normally with no error. Output footer shows "Knowledge capture: skipped (Mengdie unavailable)".

### AC8: ae:discuss Reads Prior Decisions
- **Mengdie available + results exist**: `## Prior Art from Project Knowledge Base` section appears before team spawns with provenance (title, source_file, knowledge_type, valid_from, snippet).
- **Mengdie unavailable / no results**: Output `Prior context: unavailable (tool not registered / no relevant results)` and continue normally. No error, no blocked execution.

### AC9: ae:discuss Writes Decisions
After ae:discuss conclusion written, each resolved decision is ingested as a separate memory entry with `knowledge_type=decisional`. Entities match conclusion's entities field. Rationale included in content (not just the decision text). If `memory_ingest` returns non-empty conflicts: skill output footer contains `⚠ Conflicts detected with: [comma-separated entry IDs]`.

### AC10: Full Loop Closure
- **Phase A gate**: Run 5+ ae:analyze sessions with write enabled. At least 3/5 subsequent sessions surface prior findings in Step 3.5 and AI references them in synthesis.
- **Phase B**: ae:discuss produces decision → ingested to Mengdie → ae:analyze on related topic surfaces that decision → AI references it. The knowledge spiral is closed.
- **Conflict detection**: Overlapping content triggers conflict detection, logged in output footer.
- Results documented in `docs/discussions/005-loop-validation/validation.md`.
