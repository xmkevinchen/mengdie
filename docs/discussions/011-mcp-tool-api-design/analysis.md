---
id: "011"
title: "Analysis: MCP Tool API Design Patterns"
type: analysis
created: 2026-04-05
tags: [mcp, tool-api, minimalism, discoverability, descriptions]
---

# Analysis: MCP Tool API Design Patterns

## Question

How well does mengdie's 3-tool MCP surface balance minimalism vs discoverability? What are the design tradeoffs in parameter design, tool descriptions, and workflow guidance?

## Findings

### Prior Art from Project Knowledge Base

- **[discuss]: No new memory_resolve_conflict tool** (decisional, `docs/discussions/008-contradiction-detection/conclusion.md`, 2026-04-06): Decided to keep 3-tool surface. Four targeted fixes instead of adding a 4th tool. Rationale: resolution workflow is mechanically complete via existing ingest→invalidate pattern; gaps are in descriptions, output completeness, and a dropped field.
- **[analyze]: Contradiction detection is operationally incomplete** (factual, `docs/discussions/008-contradiction-detection/analysis.md`): Resolution workflow gap was the original finding that led to the discuss decision above.
- **MVP Phase 1 Conclusion** (decisional, `docs/discussions/002-mvp-phase1/conclusion.md`): Original design: contradiction detection returns conflict flags, not interactive prompts.

### Relevant Code

- **`src/core/mcp_tools.rs`**: All 3 tools defined here — `memory_search` (line 126), `memory_ingest` (line 223), `memory_invalidate` (line 360).
- **`src/core/mcp_tools.rs:404-408`**: `ServerHandler::get_info()` — minimal 1-sentence instructions.
- **`src/core/mcp_tools.rs:44-57`**: `IngestParams` — 9 fields including `resolves: Option<Vec<String>>` for atomic resolution.
- **`src/core/mcp_tools.rs:243-250`**: Silent normalization of `source_type`/`knowledge_type` — unknown values mapped to defaults with `tracing::warn`.
- **`src/core/db.rs:136`**: `get_memory(id)` exists but has no MCP tool surface.
- **`src/core/search.rs:200`**: Search results return 200-char snippets, not full content.

### Architecture & Patterns

**Tool surface: correct at 3.**

Research consensus: 2-5 tools for focused-domain MCP servers. Mengdie's 3 tools match the "macro, not endpoint" pattern — each tool represents a complete capability (search, write, invalidate), not an atomic REST operation.

The `memory_` prefix follows `{service}_{action}` naming convention — unambiguous in multi-server contexts.

**Description quality: needs improvement.**

A 2025 study of 856 tools across 103 MCP servers found 97.1% had quality problems. Mengdie's descriptions against the 6-component rubric:

| Component | memory_search | memory_ingest | memory_invalidate |
|---|---|---|---|
| Purpose | Partial | Good | Good |
| Guidelines | Missing | Overloaded (workflow logic) | Present |
| Limitations | Missing (no snippet disclosure) | Missing | N/A |
| Param explanation | Missing | Partial | Present |
| Length | 1 sentence (too short) | 3 sentences (good length, wrong content) | 2 sentences (good) |
| Examples | Missing | Missing | Missing |

**Key design tension: workflow guidance in descriptions vs server instructions.**

`memory_ingest`'s description contains a 3-branch decision tree for conflict resolution. This is workflow orchestration logic embedded in a tool docstring — the wrong layer. The description is parsed by the LLM every invocation. It competes with per-call reasoning and is fragile (string-matching on `reason` values).

Better approach: move cross-tool workflow guidance to `ServerHandler::get_info()` instructions. Individual tool descriptions focus on what the tool does; server instructions describe when and how to use the tools together.

**Parameter design: mostly good, one clear bug.**

`source_type` and `knowledge_type` are free strings with silent normalization. An agent passing "decision" instead of "decisional" gets silently mapped to a default with only a `tracing::warn` to stderr. Research shows agents are measurably more accurate when choices are enumerated in JSON Schema (not free-text). This should be enum types.

**Missing capability: memory_get.**

`get_memory(id)` exists in `db.rs:136` but has no MCP tool. Search returns 200-char snippets. An agent that needs full content (e.g., to compare memories before resolving a conflict) has no path to retrieve it. This is either an oversight or a design constraint that needs explicit documentation.

### Industry Practice Comparison

- **Tool count**: 3 tools matches GitHub MCP's recommendation (default minimal set). Production memory MCP servers range from 4 (OpenMemory) to 9 (Mem0) to 18 (Engram).
- **Description quality**: Mengdie's descriptions are better than average (most servers have 1-line descriptions) but below the research-recommended standard (3-4 sentences with guidelines + limitations).
- **Enums vs strings**: All well-designed MCP servers use schema-constrained enums for categorical params. Free strings with silent normalization is a known anti-pattern.
- **Server instructions**: Most production MCP servers use `get_info()` instructions for workflow context. Mengdie's is minimal (just tool names).

### Challenges & Disagreements

**Challenger's strongest findings:**

1. **memory_get is a real gap** — search without full retrieval is incomplete. Either add the tool or explicitly document that memories should be self-contained within 200 chars (contradicts the 100K char content limit).
2. **Workflow logic in description is the wrong layer** — fragile, truncation-prone, competes with per-call reasoning. Belongs in server instructions.
3. **source_type/knowledge_type as free strings is a silent failure mode** — agents don't know they passed an invalid value; the system silently degrades.
4. **The `resolves` two-phase protocol is encoded nowhere in the schema** — agents must infer from prose that they should call ingest twice.

**Standards-expert's assessment:** Tool count is correct. Description quality is the primary gap. The `source_type`/`knowledge_type` string issue is a confirmed API contract bug. `memory_get` is a product decision, not a clear violation.

**Cross-family (Codex):** Pending at time of synthesis.

## Summary

**Mengdie's 3-tool MCP surface is correctly sized but has description and parameter quality gaps.**

The tool count (3) matches industry best practice for focused-domain servers. The `memory_` naming convention is correct. The `resolves` param for atomic resolution is well-designed.

**Four issues to address:**

| # | Issue | Severity | Fix |
|---|---|---|---|
| 1 | `source_type`/`knowledge_type` are free strings | High | Change to Rust enums with `Deserialize + JsonSchema` |
| 2 | `memory_search` description too short, no snippet-truncation disclosure | High | Expand to 3-4 sentences, mention 200-char limit |
| 3 | Workflow logic in `memory_ingest` description | Medium | Move to `ServerHandler::get_info()` instructions |
| 4 | Server instructions are minimal/redundant | Medium | Expand with retrieval-augmented workflow pattern |

**One product decision:**

| Issue | Options | Recommendation |
|---|---|---|
| No `memory_get` tool (200-char snippets only) | (a) Add 4th tool `memory_get(id)`, (b) Add `full_content` param to search, (c) Accept truncation | Defer — current AE skill usage works with snippets. Revisit if agents need full content for conflict resolution or comparison. |

## Possible Next Steps

- Description improvements + enum types → direct code change, no plan needed
- If `memory_get` is prioritized → `/ae:discuss` the tradeoff (4th tool vs search param vs accept truncation)
- Otherwise → backlog and proceed with Phase B validation completion
