---
title: "spec: memory_search"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_search` (MCP tool)

Search Mengdie memories by query. Returns ranked results with provenance.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn search(&self, Parameters(params): Parameters<SearchParams>)
    -> Json<SearchOutput>
```

**MCP tool description** (verbatim from `#[tool(description=...)]`):

> Search Mengdie memories by query. Returns ranked results with title,
> snippet (first 200 characters of content, not full text), score, and
> provenance. Results are ranked by hybrid FTS5 + vector similarity merged
> via Reciprocal Rank Fusion. Use `min_score` to filter low-relevance
> results. Use `scope='global'` to search across all projects (default:
> current project only).

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
      "short_id": "8-hex-chars",
      "title": "string",
      "source_file": "string",
      "source_type": "conclusion | review | plan | retrospect | synthesis | factual",
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

- `id` — full UUID v4 memory entry ID
- `short_id` — first 8 hex chars of `id`; citable short form for LLM-generated output. Use with `memory_invalidate` or `memory_get` prefix lookup
- `title` — memory title (set at ingestion time)
- `source_file` — original file path (e.g., `path/to/decision.md`); empty string if not applicable
- `source_type` — one of 6 enum values (mirrors ingestion `SourceType`)
- `knowledge_type` — one of 3 enum values (mirrors ingestion `KnowledgeType`)
- `entities` — comma-separated entity tags (no leading/trailing whitespace per item)
- `score` — hybrid relevance score after Reciprocal Rank Fusion (RRF). Higher = more relevant
- `valid_from` — UTC ISO 8601 timestamp when this memory became valid
- `snippet` — **first 200 characters of `content` field**, NOT the full text. Callers wanting full content should follow up with `memory_get`

**`degraded` field** — non-null indicates the search ran in a fallback mode:
- `"degraded: embedding unavailable, FTS-only"` — embedder failed; FTS5-only ranking
- `"query too long (max 10000 chars)"` — query rejected before search
- `"search temporarily unavailable"` — orchestrator failure (DB unreachable, etc.)

When `degraded` is non-null, callers should treat results as lower-confidence.

## Errors

| Condition | Behavior |
|---|---|
| `query.len() > 10_000` | returns empty results + `degraded: "query too long ..."` (soft-fail, no MCP error) |
| Embedder failure | returns FTS-only results + `degraded: "embedding unavailable ..."` |
| DB orchestrator failure | returns empty results + `degraded: "search temporarily unavailable"` (logged at error level) |
| MCP transport failure | rmcp returns transport error to host (out of scope of this spec) |

**Soft-fail contract**: `memory_search` does NOT return MCP-protocol errors for soft failures — it always returns a valid `SearchOutput` (possibly empty + degraded). This guarantees the audit-pipeline invariant: every search call writes one audit row, even on failure.

## Examples

### Example 1 — Successful current-project search

Request:

```json
{
  "name": "memory_search",
  "arguments": {
    "query": "auth middleware decision",
    "limit": 3
  }
}
```

Response:

```json
{
  "results": [
    {
      "id": "01h9mz1234567890abcdef",
      "short_id": "01h9mz12",
      "title": "Switched auth middleware from JWT to session cookies",
      "source_file": "docs/decisions/021-auth-middleware.md",
      "source_type": "conclusion",
      "knowledge_type": "decisional",
      "entities": "auth,middleware,session,jwt",
      "score": 0.94,
      "valid_from": "2026-04-12T00:00:00Z",
      "snippet": "After review of CSRF surface + cookie-flag controls, we switched the production auth middleware from JWT to httpOnly session cookies..."
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
  "arguments": { "query": "test query" }
}
```

Response:

```json
{
  "results": [ { "id": "...", "title": "...", "score": 0.62, "...": "..." } ],
  "degraded": "degraded: embedding unavailable, FTS-only"
}
```

Results are still returned (FTS-only ranking); caller should display the degraded notice to the user.

## Notes

**Scope semantics**:
- Default scope = current project (inferred from cwd at server startup, recorded in `default_project_id`).
- `scope: "global"` searches across all projects.
- `project_id: "<id>"` overrides default for this call (useful when MCP host running outside a project tree).
- If both `scope` and `project_id` are passed: `scope: "global"` wins (project_id is ignored when scope=global).

**Ranking algorithm**:
- Hybrid FTS5 BM25 score + vector cosine similarity, merged via Reciprocal Rank Fusion (RRF) with `k=60`.
- `score` field is the post-RRF score; ranges 0.0-1.0 typical but no hard upper bound (RRF accumulates).

**Audit pipeline invariant**:
- Every `memory_search` call fires exactly one `audit_returned_facts` write hook (post-filter, post-min_score).
- Audit captures: query, returned IDs, took_ms, route (Hybrid | FtsOnly), project_id.
- Failed searches (degraded with empty results) still fire the audit hook.

**Fallback policy** (`FallbackPolicy::HybridOrFtsOnly`):
- Embedder failure → fall back to FTS-only.
- FTS failure → return empty + degraded.
- This is the MCP-default policy (not strict-hybrid).

**Provenance**: each result's `(title, source_file, knowledge_type, valid_from)` provides the provenance fields a caller can attribute results to. `snippet` is what's shown in result lists; full content requires a follow-up `memory_get` call.

**Min-score filter applied pre-audit** — results below `min_score` are dropped before the audit hook fires.

**Cross-project search**: `scope: "global"` searches across all `project_id`s. Use sparingly — global searches return larger result sets and may dilute per-project relevance. Default per-project scope is the recommended path.
