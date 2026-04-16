---
id: "005"
title: "Phase 2 Roadmap — LLM Intelligence Layer"
status: open
created: 2026-04-16
source: "Discussion 016 (Dreaming Evolution)"
---

# Phase 2 Roadmap — LLM Intelligence Layer

Source: `docs/discussions/016-dreaming-evolution/conclusion.md`
Reference implementations: SmartPal, OpenClaw, Hermes-Agent

## Phase 2.1: LLM Provider + Dream Synthesis

### BL-005: LLM Provider Trait + Claude CLI Implementation
- **What**: `LlmProvider` trait + `ClaudeCliProvider` impl (tokio::process::Command, `claude -p --output-format text`)
- **Reference**: SmartPal `backend/app/core/llm/` (simplest), OpenClaw `~/.claude/credentials` migration, Hermes error classification
- **Auth**: read `~/.claude/credentials` (zero config), fallback to env `ANTHROPIC_API_KEY`
- **Config**: `~/.mengdie/config.toml` `[llm]` section
- **Trigger**: first Phase 2 item, everything depends on this
- **Scope**: ~200-300 LOC. `src/core/llm.rs` (new)

### BL-006: Embedding Clustering
- **What**: `cluster_memories()` — greedy cosine clustering on existing fastembed embeddings, threshold ~0.75, min cluster size 3
- **Trigger**: needed for dream synthesis
- **Scope**: ~100-150 LOC. `src/core/clustering.rs` (new)

### BL-007: Dream Synthesis (LLM-powered)
- **What**: `mengdie dream` second stage — cluster → LLM synthesize → store as `source_type = "synthesis"`
- **Schema**: add `memory_synthesis_links` table (source → synthesis, many-to-many). Do NOT reuse `superseded_by`.
- **Prompt**: enforce JSON output `{"title", "content", "entities"}`, validate with serde
- **Depends on**: BL-005 (LLM provider) + BL-006 (clustering)
- **Trigger**: after BL-005 + BL-006 complete
- **Scope**: ~300-500 LOC. `src/core/dreaming.rs`, `src/core/schema.rs`, `src/bin/cli.rs`

### BL-008: Power-Law Decay (Computed, Not Stored)
- **What**: effective_relevance = avg_relevance × 0.95^days at promotion/demotion time. NEVER mutate stored avg_relevance.
- **Demotion**: effective < 0.01 → clear is_longterm
- **Independent of LLM**: can ship alongside or before BL-005
- **Trigger**: any time (no dependency)
- **Scope**: ~50-100 LOC in `src/core/dreaming.rs`

### BL-009: MCP Dream Tool (Session-Based)
- **What**: `memory_dream` MCP tool — runs decay + promote + cluster, returns clusters to Claude. Claude synthesizes inline and calls `memory_ingest`.
- **Why**: in Claude session, Claude IS the LLM — no need to shell out
- **Depends on**: BL-006 (clustering), BL-008 (decay)
- **Trigger**: after BL-006 + BL-008
- **Scope**: ~100-150 LOC in `src/core/mcp_tools.rs`

## Phase 2.2: Daemon + Async Queue

### BL-010: Daemon Process + Job Queue
- **What**: persistent daemon (launchd), SQLite `pending_jobs` table, polls for work
- **Jobs**: entity extraction, synthesis, future tasks
- **IPC**: SQLite as communication channel (MCP writes job row, daemon polls)
- **Depends on**: Phase 2.1 complete
- **Trigger**: when real-time enrichment becomes needed

### BL-011: Async Entity Extraction on Ingest
- **What**: ingest stores immediately, enqueues extraction job. Daemon extracts entities from content via LLM.
- **Fixes**: silent contradiction-detection bypass when tags missing
- **Depends on**: BL-010 (daemon), BL-005 (LLM provider)
- **Trigger**: after daemon exists

## Phase 2.3: RAG Search

### BL-012: LLM-Based RAG Search
- **What**: `memory_query` MCP tool — retrieve top-N → send to LLM with context → return synthesized answer with source citations
- **Depends on**: BL-005 (LLM provider)
- **Trigger**: when search-only results feel insufficient

## Phase 2.4: Knowledge Graph

### BL-013: Knowledge Graph Schema + Typed Edges
- **What**: `memory_relationships` table (source_id, dest_id, rel_type, strength). Types: supersedes, contradicts, extends, relates-to.
- **Reference**: Graphiti (temporal edges), Gemini-proxy analysis (adjacency list pattern)
- **Migration**: non-breaking additive table, populate edges from entity overlap + cosine
- **Depends on**: BL-011 (extracted entities) for meaningful edges
- **Trigger**: 500+ memories or repeated search quality complaints

## Phase 2.5: Feedback Loop

### BL-014: Feedback Signal + RL-Like Tuning
- **What**: `memory_feedback(entry_id, helpful: bool)` MCP tool. Dreaming uses this signal to adjust knowledge retention.
- **Depends on**: Phase 2.1-2.4 producing enough data
- **Trigger**: after 3+ months of active use with LLM integration
