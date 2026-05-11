---
title: "spec: memory_search"
type: spec
created: 2026-05-08
as_of_commit: 1b48c92
stability: draft-until-v0.0.1     # draft-until-v0.0.1 / stable / deprecating
implementation_status: current     # current / planned / partial
audience: [operator, llm-at-mcp-tool-discovery]
---

# spec: `memory_search` (MCP tool)

Search Mengdie memories by query. Returns ranked results with provenance.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn search(&self, Parameters(params): Parameters<SearchParams>)
    -> Json<SearchOutput>
```

**MCP tool description** (verbatim from `#[tool(description=...)]`, 76 words / ~110 tokens):

> Search Mengdie memories by query. Returns ranked results with title,
> snippet (first 200 characters of content, not full text), score, and
> provenance. Results are ranked by hybrid FTS5 + vector similarity merged
> via Reciprocal Rank Fusion. Use `min_score` to filter low-relevance
> results. Use `scope='global'` to search across all projects (default:
> current project only).

(Under 200-token cap per F-004 plan AC3.)

## Params

| Field | Type | Required | Default | Constraint |
|---|---|---|---|---|
| `query` | `String` | yes | — | ≤ 10,000 chars; longer → degraded response |
| `scope` | `String?` | no | (current project) | `"global"` searches all projects; any other value treated as current-project |
| `project_id` | `String?` | no | inferred from cwd at server startup | overrides default project_id (e.g., when host running outside a project tree) |
| `limit` | `usize?` | no | `10` | maximum number of results |
| `min_score` | `f64?` | no | `0.0` | results scoring below this are filtered out (range 0.0-1.0) |

## Returns

```json
{
  "results": [
    {
      "id": "string",
      "title": "string",
      "source_file": "string",
      "source_type": "conclusion | review | plan | retrospect | synthesis",
      "knowledge_type": "decisional | experiential | factual",
      "entities": "comma,separated,tags",
      "score": 0.87,
      "valid_from": "YYYY-MM-DDTHH:MM:SSZ",
      "snippet": "first 200 chars of content"
    }
  ],
  "degraded": null
}
```

**Result item field semantics**:

- `id` — opaque memory entry ID (UUID-like)
- `title` — memory title (set at ingestion time)
- `source_file` — original file path (e.g., `docs/discussions/027/conclusion.md`); empty string if not applicable
- `source_type` — one of 5 enum values (mirrors ingestion `SourceType`)
- `knowledge_type` — one of 3 enum values (mirrors ingestion `KnowledgeType`)
- `entities` — comma-separated entity tags (no leading/trailing whitespace per item)
- `score` — hybrid relevance score after Reciprocal Rank Fusion (RRF). Higher = more relevant
- `valid_from` — UTC ISO 8601 timestamp when this memory became valid
- `snippet` — **first 200 characters of `content` field**, NOT the full text. Callers wanting full content must call a separate retrieval (currently no MCP tool for that — operator-only via SQLite)

**`degraded` field** — non-null indicates the search ran in a fallback mode:
- `"degraded: embedding unavailable, FTS-only"` — embedder failed; FTS5-only ranking
- `"query too long (max 10000 chars)"` — query rejected before search
- `"search temporarily unavailable"` — orchestrator failure (DB unreachable, etc.)

When `degraded` is non-null, callers SHOULD treat results as lower-confidence; ae:analyze annotates these as `(partial — [degraded reason])`.

## Errors

| Condition | Behavior |
|---|---|
| `query.len() > 10_000` | returns empty results + `degraded: "query too long ..."` (no MCP-level error; soft-fail) |
| Embedder failure | returns FTS-only results + `degraded: "embedding unavailable ..."` |
| DB orchestrator failure | returns empty results + `degraded: "search temporarily unavailable"` (logged at error level) |
| MCP transport failure | rmcp returns transport error to host (out of scope of this spec) |

**Note**: `memory_search` does NOT return MCP-protocol errors for soft failures — it always returns a valid `SearchOutput` (possibly empty + degraded). This decision is per F-002 audit-substrate invariant: every search call writes one audit row, even on failure.

## Examples

### Example 1 — Successful current-project search

Request:

```json
{
  "name": "memory_search",
  "arguments": {
    "query": "v0.0.1 thesis narrow OSS adoption",
    "limit": 3
  }
}
```

Response:

```json
{
  "results": [
    {
      "id": "01h9mz...",
      "title": "v0.0.1 thesis: narrow OSS-adoption (operator clarification 2026-05-05)",
      "source_file": "docs/discussions/026-rust-oss-survey/conclusion.md",
      "source_type": "conclusion",
      "knowledge_type": "decisional",
      "entities": "v0.0.1,oss-adoption,sqlite-vec,rig",
      "score": 0.94,
      "valid_from": "2026-05-05T00:00:00Z",
      "snippet": "Per 026 OSS-survey analysis verdicts (already settled at analyze time), v0.0.1 means narrow OSS-adoption scope, NOT a rip-out-and-replace rebuild..."
    }
  ],
  "degraded": null
}
```

### Example 2 — Degraded (embedder unavailable)

Request:

```json
{
  "name": "memory_search",
  "arguments": {
    "query": "test query"
  }
}
```

Response:

```json
{
  "results": [
    {
      "id": "01h9...",
      "title": "...",
      "score": 0.62,
      "...": "..."
    }
  ],
  "degraded": "degraded: embedding unavailable, FTS-only"
}
```

Note: results are still returned (FTS-only ranking); caller should display the degraded notice to the user.

## Notes

**Scope semantics**:
- Default scope = current project (inferred from cwd at server startup, recorded in `default_project_id`).
- `scope: "global"` searches across all projects.
- `project_id: "<id>"` overrides default for this call (useful when MCP host running outside a project tree).
- If both `scope` and `project_id` are passed: `scope: "global"` wins (project_id is ignored when scope=global).

**Ranking algorithm**:
- Hybrid FTS5 BM25 score + vector cosine similarity, merged via Reciprocal Rank Fusion (RRF) with `k=60`.
- Score field is the post-RRF score; ranges 0.0-1.0 typical but no hard upper bound (RRF accumulates).

**Audit substrate (F-002 invariant)**:
- Every `memory_search` call fires exactly one `audit_returned_facts` write hook (post-filter, post-min_score).
- Audit captures: query, returned IDs, took_ms, route (Hybrid | FtsOnly), project_id.
- Failed searches (degraded with empty results) still fire the audit hook.

**Fallback policy** (`FallbackPolicy::HybridOrFtsOnly`):
- Embedder failure → fall back to FTS-only.
- FTS failure → return empty + degraded.
- Per F-003 Topic 1: MCP-default policy is `HybridOrFtsOnly` (not strict-hybrid).

**Provenance for ae:analyze Round 0 injection**:
- Each result's `(title, source_file, knowledge_type, valid_from)` provides the provenance ae:analyze attributes in its Round 0 prefix block.
- `snippet` is what's shown in Round 0; full content is not retrieved.

**Min-score filter applied pre-audit** — results below `min_score` are dropped before audit hook fires (F-003 Wave 2 invariant).

**Cross-project search**: `scope: "global"` searches across all `project_id`s. Use sparingly — global searches return larger result sets and may dilute per-project relevance. Default per-project scope is the recommended path for AE workflows.
