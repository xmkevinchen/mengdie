---
id: "003"
title: "Phase 1.1 — API Contract Correctness + Knowledge Capture Completeness"
type: plan
created: 2026-04-09
status: done
discussion: "docs/discussions/012-phase-1.1-scope/"
---

# Feature: Phase 1.1 — API Contract Correctness + Knowledge Capture Completeness

## Goal

Make the MCP API contract reliable (enums, descriptions, server instructions) and complete the knowledge capture integration across all AE pipeline skills, so every pipeline stage that produces durable knowledge writes to Mengdie and every stage that benefits from prior context reads from it.

## Cross-Repo Scope

- **mengdie** (`/Users/ckai/Workspace/Projects/mengdie`): Steps 1-2 (API contract fixes)
- **agentic-engineering-mengdie** (`/Users/ckai/Workspace/Projects/agentic-engineering-mengdie`): Steps 3-6 (AE skill wiring)

## Source Decisions

- [012 Phase 1.1 Scope Conclusion](../discussions/012-phase-1.1-scope/conclusion.md) — scope, execution approach, acceptance criteria
- [AE PRD Phase C](agentic-engineering-mengdie: docs/prd/mengdie-integration.md) — skill-level integration map, extraction heuristics
- [Knowledge Capture Protocol](agentic-engineering-mengdie: plugins/ae/docs/knowledge-capture-protocol.md) — shared extraction rules

## Review Notes

Plan reviewed by: architect, dependency-analyst, simplicity-reviewer.

Key changes from review:
- Merged Steps 2+3 (both text edits in mcp_tools.rs) into single Step 2
- Moved Step 0 (Phase B gate check) to Precondition
- Moved Steps 9-10 (verification-only) to AC verification instructions
- Added `src/core/parser.rs` to Step 1 Expected Files (validate_* functions live there)
- Simplified mid-execution gate to boundary note
- Fixed step numbering (was 0,1,2,3,gate,5,6,7,8,9,10 → now 1,2,boundary,3,4,5,6)
- Steps 3-6 can be parallel (different SKILL.md files, no overlap)

## Precondition

Before starting, verify the knowledge loop is functional:
- Call `memory_search` with query "conflict resolution MCP tool" in current session
- Verify the discuss 008 decision appears in results
- If not found: rebuild binary (`cargo build --release`), restart MCP server, retry

## Steps

### Step 1: source_type/knowledge_type → Rust enums (AC1)

- [x] Create `SourceType` enum (`Conclusion`, `Review`, `Plan`, `Retrospect`) with `Deserialize + schemars::JsonSchema` derives (e773081)
- [x] Create `KnowledgeType` enum (`Decisional`, `Experiential`, `Factual`) with same derives (e773081)
- [x] Replace `source_type: String` and `knowledge_type: String` in `IngestParams` with enum types (e773081)
- [x] Remove silent normalization logic (`validate_source_type`, `validate_knowledge_type` functions) (e773081)
- [x] Enum variants serialize as lowercase (`#[serde(rename_all = "lowercase")]`) to match existing stored values (e773081)
- [x] Update `NewMemory` struct to accept enum `.to_string()` — DB stores as TEXT, no schema change needed (e773081)
- [x] Add test: ingest with `source_type="decision"` (invalid) returns error, not silent normalization (766cf8a)
- [x] Add test: ingest with `source_type="conclusion"` (valid) succeeds (766cf8a)
- [x] `cargo test` passes, `cargo clippy` clean (e773081)

Expected files: `src/core/mcp_tools.rs`, `src/core/db.rs`, `src/core/ingest.rs`, `src/core/parser.rs`

### Step 2: Tool descriptions + server instructions rewrite (AC2, AC3)

- [x] Expand `memory_search` description to 3-4 sentences: purpose, what's returned (200-char snippets, not full content), hybrid FTS5+vector ranking, guidance on `min_score` and `scope` params (36838c1)
- [x] Move conflict resolution workflow logic from `memory_ingest` description to `ServerHandler::get_info()` instructions (36838c1)
- [x] Server instructions explain: (a) search for context, (b) ingest to store, (c) two-call resolution with `superseded_by`, (d) `resolves` param for atomic resolution (36838c1)
- [x] `memory_ingest` description reduced to: what it does, what it returns, mention of `resolves` param — no branching decision tree (36838c1)
- [x] `cargo build` succeeds (36838c1)

Expected files: `src/core/mcp_tools.rs`

### --- Boundary: rebuild + restart ---

Rebuild mengdie-mcp binary (`cargo build --release`), restart MCP server, verify `memory_search` and `memory_ingest` work via MCP before proceeding to AE skill changes.

### Step 3: ae:think — read integration (AC7)

- [x] Add Step 1.5 Prior Context after Frame (Step 1), before Agent Teams Investigation (Step 2) (ba5fe5d)
- [x] Query: use $ARGUMENTS problem statement (ba5fe5d)
- [x] Graceful degradation: "Prior context: unavailable" on failure/no results (ba5fe5d)
- [x] No write step (output is ephemeral reasoning per PRD) (ba5fe5d)

Expected files (AE repo): `plugins/ae/skills/think/SKILL.md`

### Step 4: ae:plan — read + write integration (AC4)

- [x] Add Step 1.5 Prior Context after Research (Step 1), before Write Plan (Step 2) (22ee40c)
- [x] Query: feature description from $ARGUMENTS or referenced discussion's problem statement (22ee40c)
- [x] Add Knowledge Capture step after Doodlestein (Step 4), before Confirm (Step 5) (22ee40c)
- [x] Gate: only capture if plan `status: reviewed` (skip for draft plans) (22ee40c)
- [x] Extraction: overall approach rationale + non-obvious technical choices, max 3, source_type `plan`, knowledge_type `decisional` (22ee40c)
- [x] Entities: compound tags per decision (per knowledge-capture-protocol.md rule 4) (22ee40c)
- [x] Add conflict summary to Confirm step output (22ee40c)
- [x] Reference knowledge-capture-protocol.md for common rules (22ee40c)

Expected files (AE repo): `plugins/ae/skills/plan/SKILL.md`

### Step 5: ae:review — read + write integration (AC5)

- [x] Add Prior Context step before Step 1 (Create Team), after Pre-checks (7431143)
- [x] Query: feature name from $ARGUMENTS or plan title (7431143)
- [x] Include prior art in reviewer prompts as context (add prior art to Step 3 prompt templates) (7431143)
- [x] Add Knowledge Capture step after Output (review file written), before "Prompt user to create PR" (7431143)
- [x] Extraction: reusable patterns (P2+ findings that apply beyond this code), max 3, source_type `review`, knowledge_type `experiential` (7431143)
- [x] Add conflict summary to output (7431143)
- [x] Reference knowledge-capture-protocol.md (7431143)

Expected files (AE repo): `plugins/ae/skills/review/SKILL.md`

### Step 6: ae:retrospect — read + write integration (AC6)

- [x] Add Step 0.5 Prior Context between Pre-check and Step 1 (Collect Outcome Statistics) (3e33a57)
- [x] Query: "retrospective insights" or $ARGUMENTS filter (3e33a57)
- [x] Add Knowledge Capture step after Step 4 (Output written), before Next Steps (3e33a57)
- [x] Gate: skip Knowledge Capture in `--compare` mode (comparison, not new insights) (3e33a57)
- [x] Extraction: actionable trend conclusions, skip raw statistics, max 3, source_type `retrospect`, knowledge_type `experiential` (3e33a57)
- [x] Add conflict summary to output (3e33a57)
- [x] Reference knowledge-capture-protocol.md (3e33a57)

Expected files (AE repo): `plugins/ae/skills/retrospect/SKILL.md`

## Acceptance Criteria

### AC1: Enum Validation — source_type/knowledge_type reject unknown values
memory_ingest with `source_type="decision"` (typo) returns a structured error, not silent normalization. Valid values (`conclusion`, `review`, `plan`, `retrospect` for source_type; `decisional`, `experiential`, `factual` for knowledge_type) succeed. Verified via `cargo test`.

### AC2: Search Description — 3+ sentences with limitation disclosure
memory_search tool description is 3-4 sentences. Mentions: purpose, 200-char snippet limit, guidance on min_score and scope. Verified by reading `mcp_tools.rs`.

### AC3: Server Instructions — workflow guidance moved
memory_ingest description contains no branching workflow logic ("if reason contains..."). Server instructions (`get_info()`) explain the resolution workflow. Verified by reading `mcp_tools.rs`.

### AC4: ae:plan — read + write integration
ae:plan surfaces prior context before designing steps AND writes knowledge items after plan is reviewed. Verified: run once, check `mengdie list` for new entries with source_type `plan`.

### AC5: ae:review — read + write integration
ae:review surfaces prior context before reviewers start AND writes reusable patterns after report. Verified: run once, check `mengdie list` for new entries with source_type `review`.

### AC6: ae:retrospect — read + write integration
ae:retrospect surfaces prior context before analysis AND writes trend conclusions after output. Verified: run once, check `mengdie list` for new entries with source_type `retrospect`.

### AC7: ae:think — read integration
ae:think surfaces prior context before investigation. No write step. Verified: run once, check output includes Prior Art section.

### AC8: Graceful Degradation — all skills work without Mengdie
With Mengdie MCP disconnected, ae:think/plan/review/retrospect all continue normally. Prior Context step emits "unavailable" and skill proceeds unchanged. No errors, no blocking.
Verification: disconnect MCP, run ae:think on a topic, confirm it works. Reconnect.

### AC9: Workflow Marker — ae:plan surfaces prior decision unprompted
Prerequisite: Steps 3-6 all complete, AE repo committed, plugin reloaded.
In a fresh Claude Code session, run ae:plan on a real feature that relates to prior discuss/analyze decisions. Verify Step 1.5 Prior Context surfaces at least 1 prior decision from Mengdie. The agent references it in the plan without being told to look for it. This is the "part of workflow" signal.
