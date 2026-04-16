---
id: "006"
title: "Phase 2.1 — Dream MVP: LLM Synthesis + Decay"
type: plan
created: 2026-04-16
status: cancelled
discussion: "docs/discussions/016-dreaming-evolution/"
---

# Feature: Phase 2.1 — Dream MVP

## Goal

Make mengdie "think" — `mengdie dream` clusters related memories by embedding similarity, calls an LLM to synthesize each cluster into a consolidated insight, and applies power-law decay to stale memories. First visible intelligence in the system.

## Steps

### Step 1: LLM provider trait + Claude CLI implementation (AC1)
- [ ] Define `LlmProvider` trait: `async fn complete(&self, system: &str, prompt: &str) -> Result<String>`
- [ ] Implement `ClaudeCliProvider`: uses `tokio::process::Command` to call `claude -p --tools "" --no-session-persistence --output-format text`
- [ ] Error handling: distinguish NotFound / timeout / non-zero exit / empty stdout / invalid UTF-8
- [ ] Config: read `~/.mengdie/config.toml` for `[llm]` section (provider, model, cli_path). Fallback to env vars (`MENGDIE_LLM_PROVIDER`, `CLAUDE_CLI_PATH`).
- [ ] Unit test: argument building produces correct command line
- [ ] Integration test: if `claude` binary exists, call with trivial prompt and verify non-empty response
Expected files: `src/core/llm.rs` (new), `src/core/mod.rs`

### Step 2: Embedding clustering (AC2)
- [ ] `cluster_memories(db: &Db, project_id: Option<&str>, threshold: f64) -> Vec<Vec<MemoryEntry>>` — load all valid memory embeddings, greedy cosine clustering (threshold ~0.75), return clusters with ≥3 members
- [ ] Reuse existing `cosine_similarity` from `vector.rs`
- [ ] Pass project_id through for scoping (don't cluster across projects)
- [ ] Unit test: 6 memories with known embeddings (3 similar + 3 different) → expect 1 cluster of 3
- [ ] Unit test: no memories above threshold → empty clusters
Expected files: `src/core/clustering.rs` (new), `src/core/mod.rs`

### Step 3: Schema + synthesis in dream command (AC3)
- [ ] Add migration v4: `memory_synthesis_links` table (`source_memory_id TEXT, synthesis_memory_id TEXT, created_at TEXT, PRIMARY KEY(source_memory_id, synthesis_memory_id)`)
- [ ] Add `Synthesis` variant to `SourceType` enum in `mcp_tools.rs`
- [ ] After existing promotion pass in `cmd_dream`, add synthesis stage:
  1. Run clustering on valid memories
  2. For each cluster: build prompt with titles + content (truncated at ~8000 chars ≈ 2000 tokens)
  3. System prompt enforces JSON output: `{"title": "...", "content": "...", "entities": ["..."]}`
  4. Call LlmProvider, parse response with `serde_json::from_str` + validation
  5. Store as new memory with `source_type = "synthesis"`, `knowledge_type = "factual"`
  6. Insert `memory_synthesis_links` rows (source → synthesis) — originals stay valid
- [ ] CLI flags: `--synthesize` (default on), `--no-synthesize` (skip), `--dry-run` (show clusters, don't call LLM)
- [ ] Print: cluster count, memories per cluster, synthesis titles created
Expected files: `src/core/schema.rs`, `src/core/dreaming.rs`, `src/core/mcp_tools.rs`, `src/bin/cli.rs`

### Step 4: Power-law decay — computed, not stored (AC4)
- [ ] In `run_dreaming_with_config`, compute effective relevance at promotion time: `effective = avg_relevance * 0.95_f64.powf(days_since_last_recall)`. Use this for threshold comparison instead of raw `avg_relevance`.
- [ ] Demotion: memories where effective relevance < 0.01 AND is_longterm → set `is_longterm = 0`
- [ ] Do NOT modify stored `avg_relevance` — it stays as the true recall signal
- [ ] Run decay-aware promotion before synthesis in `cmd_dream`
- [ ] CLI flag: `--decay-rate` (default 0.95)
- [ ] Unit test: memory with last_recalled 30 days ago, effective relevance = original * 0.95^30 ≈ 0.21x
- [ ] Unit test: demoted memory loses is_longterm flag
Expected files: `src/core/dreaming.rs`, `src/bin/cli.rs`

### Step 5: End-to-end verification (AC5)
- [ ] Update `resources/com.mengdie.dream.plist`: correct binary path (`~/.cargo/bin/mengdie`)
- [ ] Run `mengdie dream` on real DB — verify clusters form, synthesis memories created, decay-aware promotion works
- [ ] Verify: `mengdie search "topic"` returns synthesis memories alongside originals
- [ ] Verify: `mengdie stats` shows updated counts
Expected files: `resources/com.mengdie.dream.plist`

Note: Steps 1 and 2 are parallel-safe. Step 3 depends on both 1 and 2. Step 4 is independent (parallel with 1-2). Step 5 depends on all.

## Acceptance Criteria

### AC1: LLM Provider Works
- `ClaudeCliProvider::complete()` returns non-empty response from Claude CLI via `tokio::process::Command`
- Config from `~/.mengdie/config.toml` or env vars
- Trait allows future providers without changing callers

### AC2: Clustering Produces Coherent Groups
- Memories with cosine similarity ≥ 0.75 grouped together
- Clusters with < 3 members excluded
- Clustering completes in < 1 second for 200 memories
- Project-scoped (no cross-project clustering)

### AC3: Dream Synthesis Creates New Knowledge
- `mengdie dream` produces `source_type = "synthesis"` memories in the DB
- Each synthesis linked to sources via `memory_synthesis_links` (originals NOT invalidated)
- LLM output validated: JSON with title, content, entities
- Synthesis memories appear in `memory_search` results
- `--dry-run` shows clusters without calling LLM

### AC4: Decay-Aware Promotion
- Effective relevance computed at promotion time: `avg_relevance * 0.95^days`
- Stored `avg_relevance` is NEVER modified by decay
- Memories with effective relevance < 0.01 lose is_longterm (demotion)
- Freshly recalled memories unaffected

### AC5: End-to-End Dream Cycle
- `mengdie dream` runs: decay-aware promote → cluster → synthesize
- At least 1 synthesis memory created from 194-memory corpus
- launchd plist updated with correct path
