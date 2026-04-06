---
id: "010"
title: "Analysis: Cross-Project Knowledge Sharing"
type: analysis
created: 2026-04-05
tags: [cross-project, project-id, scoping, privacy, global-search, fts5]
---

# Analysis: Cross-Project Knowledge Sharing

## Question

How does mengdie handle cross-project knowledge? Is the project_id scoping model correct, and what patterns enable useful knowledge transfer without noise?

## Findings

### Prior Art from Project Knowledge Base

- **[analyze]: Mengdie hybrid search (RRF k=60) correctly implemented** (factual, `docs/discussions/005-hybrid-search-analysis/analysis.md`): RRF merges FTS5 + vector by rank. Relevant because cross-project search relies on RRF to rank results from different projects without weighting by source.
- **[analyze]: Entity-tag overlap is a noisy proxy for contradiction** (factual, `docs/discussions/008-contradiction-detection/analysis.md`): Broad tags cause false positives. Cross-project contradiction detection inherits this problem — tags like "auth" span multiple projects.
- **[analyze]: is_longterm has zero effect on search** (factual, `docs/discussions/009-dreaming-promotion/analysis.md`): Dreaming promotion is disconnected from retrieval. If cross-project search is added, promoted memories should carry weight regardless of source project.
- **[analyze]: all-MiniLM-L6-v2 is weakest candidate but adequate** (factual, `docs/discussions/007-embedding-model-tradeoffs/analysis.md`): Embedding quality affects cross-project semantic search — weaker model means less reliable cross-project paraphrase matching.
- **[analyze]: Contradiction detection is operationally incomplete** (factual, `docs/discussions/008-contradiction-detection/analysis.md`): Resolution workflow missing. Cross-project contradictions (same entity, different decisions per project) are invisible — contradiction detection is hard-scoped by project_id.

### Relevant Code

- **`src/core/project.rs:8`**: `infer_project_id()` — runs `git remote get-url origin`, normalizes SSH/HTTPS URLs to identical hash, FNV-1a → `proj_<16hex>`. Falls back to absolute path if no remote.
- **`src/core/search.rs:27,47-54`**: FTS search — `WHERE memory_fts MATCH ?1 AND me.project_id = ?3`. FTS5 virtual table has no `project_id` column — BM25 IDF computed over entire corpus before project filter.
- **`src/core/vector.rs:43`**: Vector search — `WHERE embedding IS NOT NULL AND embedding_dim = ?2` with optional `AND project_id = ?3`.
- **`src/core/mcp_tools.rs:131-138`**: Scope logic — `scope: "global"` passes `project_id = None` to search (no filter). Default uses `self.default_project_id`.
- **`src/bin/mcp_server.rs:32-37`**: `default_project_id` frozen at cwd on MCP server startup. No per-request override mechanism.
- **`src/core/schema.rs:26,43`**: `project_id TEXT NOT NULL` with index `idx_memory_project ON memory_entries(project_id)`.
- **`src/core/schema.rs:105-106`**: Dedup unique constraint: `(project_id, content_hash)` — same content in two projects creates two entries.
- **`src/core/contradiction.rs:48-73`**: Contradiction check hard-scoped: `WHERE project_id = ?1`. Cross-project contradictions invisible.
- **Tests**: `test_fts5_search_respects_project`, `test_memory_search_global_scope`, `test_vector_search_respects_project_filter`, `test_cross_project_no_conflict`.

### Architecture & Patterns

**Current model: binary scope — project-local or fully global.**

The design (CLAUDE.md: "Global storage, per-project default search") works as specified. The implementation is clean: `project_id = Some(pid)` for project-scoped, `None` for global. All search paths respect this consistently.

**What works well:**
- Unified SQLite DB with project_id column — correct foundation (matches enterprise multi-tenant pattern)
- Project-scoped search as default — aligned with industry consensus (Notion, Linear, Slack, Cursor, Copilot all default to workspace-first)
- Global search opt-in via `scope: "global"` — explicit, not automatic
- Project_id index exists — efficient filtering at scale

**What's missing:**

1. **No global knowledge tier.** There's no way to mark a memory as "applies to all projects" (e.g., "always prefer rustls over native-tls"). Such knowledge lives in a specific project and is invisible in other contexts unless global search is explicitly used. Industry pattern: three tiers (global → project → session), not two (project or everything).

2. **No cross-project weighting.** Global search treats all results equally regardless of source project. Industry standard: prefer current-project results at higher rank, include cross-project at a relevance penalty. RRF has no project-affinity signal.

3. **FTS5 IDF contamination.** BM25 IDF is computed over the entire FTS5 virtual table (all projects), then results are post-filtered by project_id. If project A has 100 memories about "authentication" and project B has 2, the term "authentication" gets a dampened IDF globally. Project B's BM25 scores for "authentication" are degraded by project A's corpus. This is a correctness issue that worsens as projects accumulate.

4. **MCP startup lock-in.** `default_project_id` is frozen at server startup. Multi-project sessions in Claude Code get silently wrong default scope. The `project_id` MCP parameter allows override, but Claude Code doesn't use it dynamically.

5. **No scope field in schema.** Can't distinguish "transferable general knowledge" from "project-specific decisions" at the data level. Without this, cross-project search is unprincipled — it surfaces everything or nothing.

### Industry Practice Comparison

**Unanimous industry pattern: workspace-first default, explicit opt-in for broader scope.**

| Tool | Default scope | Cross-scope mechanism |
|---|---|---|
| Notion | Workspace-only | Workspace switcher, "All sources" opt-in |
| Linear | Workspace-only | Workspace switcher |
| GitHub | Global (outlier) | `repo:`, `org:` qualifiers to narrow |
| Slack | Workspace-scoped | `in:`, `from:` filters; Enterprise global |
| Cursor | Current workspace | `@Files/@Folders` for cross-project; project rules scoped |
| Copilot | Current workspace | Copilot Spaces for curation |
| Claude Code | Per-project memory | `~/.claude/projects/<project>/memory/MEMORY.md` isolation |

Mengdie's default (project-scoped) matches 6 of 7 tools. The global opt-in (`scope: "global"`) matches the pattern.

**What mengdie is missing that others have:**
- **Global/org tier** (Cursor's `alwaysApply` rules, Claude Code's global `~/.claude/CLAUDE.md`)
- **Provenance labels** ("Found in [project-name]") in cross-project results
- **Curated cross-project sets** (Copilot Spaces, Obsidian "vaults")

**Security considerations:**
- Pre-retrieval authorization (filter by project_id before ranking) — mengdie does this correctly
- Cross-project search should remain opt-in to prevent sensitive data leakage
- No credential scrubbing at ingestion — a risk if API keys or secrets appear in ingested documents

### Challenges & Disagreements

**Challenger's core thesis: cross-project sharing adds friction, not value, for a single developer.**

Key challenges:
1. **FTS5 IDF contamination** — cross-project data actively degrades within-project search quality. BM25 IDF over the global corpus dampens project-specific term frequencies.
2. **project_id instability** — forks get upstream's ID, local-only repos use absolute path (breaks on move), monorepos get one ID for N logical projects.
3. **No "scope" field** — can't distinguish transferable knowledge from project-local decisions.
4. **Recall stat corruption** — global search hits inflate recall_count for memories in other projects, corrupting Dreaming's promotion signal.

**Standards-expert's counter:** The value of cross-project knowledge isn't team sharing — it's **the developer not having to rediscover their own decisions in each new project.** Personal conventions (toolchain choices, architecture preferences, recurring patterns) are exactly what should transfer. The binary model (project-only or everything) is the gap, not cross-project itself.

**Standards-expert's recommended fix:** Three-tier model (global → project → session). Global tier for universal knowledge, project tier for project-specific decisions. Memories marked as "global" at ingestion always surface. Cross-project search remains opt-in for non-global content.

**Cross-family (Codex):** Confirmed all major AI coding tools (Cursor, Copilot, Windsurf) default to workspace-first with opt-in expansion. Recommended hybrid model: current project default, explicit one-action opt-in to broaden. MCP protocol has no built-in "project scope" — it's a client convention.

**Consensus:** Project-scoped default is correct. The missing piece is a global knowledge tier for personal conventions, plus provenance labels for cross-project results.

## Summary

**Mengdie's project scoping is architecturally sound but lacks a global knowledge tier and has a BM25 quality issue.**

The unified DB with project_id filtering matches industry best practices for multi-tenant search. Project-scoped default with opt-in global search aligns with 6 of 7 major developer tools surveyed.

**Three structural gaps:**

| Gap | Impact | Fix |
|---|---|---|
| No global knowledge tier | Personal conventions invisible across projects | Add `scope` field or `project_id = "global"` convention; always include global memories in search |
| FTS5 IDF contamination | Cross-project corpus degrades per-project BM25 quality | Long-term: per-project FTS5 tables or include project_id in FTS5. Short-term: accept and rely on RRF vector leg to compensate |
| project_id instability (forks, moves, monorepos) | Knowledge orphaning on repo reconfiguration | Add explicit `--project-name` override; store human-readable name alongside hash |

**Two concrete items for Phase 2:**

| Item | Effort | Value |
|---|---|---|
| Global scope at ingestion (`scope: "global"` on memory_ingest) | Small — schema migration + search logic tweak | Enables the most valuable cross-project use case (personal conventions) |
| Provenance labels in search results | Small — add `project_id` or project name to SearchResult | Users know where cross-project results came from |

**Backlog items:**

| Item | Trigger | Action |
|---|---|---|
| Cross-project weighting in RRF | Multiple active projects with overlapping vocabulary | Add project-affinity score as third RRF input |
| Per-project FTS5 tables | FTS5 IDF contamination becomes measurable (5+ projects) | Partition FTS index by project_id |
| Explicit project name/alias | Multi-machine or monorepo use | Store human-readable project name; support `--project-name` override |
| Cross-project contradiction detection | Cross-project search is actively used | Extend contradiction scope to match search scope |

## Possible Next Steps

- If global knowledge tier is prioritized → `/ae:discuss` the schema change and ingestion UX
- If FTS5 IDF contamination needs quantification → run A/B test with multi-project corpus
- Otherwise → backlog and proceed with closing the loop validation
