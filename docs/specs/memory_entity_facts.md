---
title: "spec: memory_entity_facts"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_entity_facts` (MCP tool)

List all valid facts tagged with a given entity name. Complements `memory_search` when semantic match isn't specific enough.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn entity_facts(&self, Parameters(params): Parameters<EntityFactsParams>)
    -> Json<EntityFactsOutput>
```

**MCP tool description** (verbatim):

> List all VALID (non-invalidated, non-superseded) facts tagged with a
> given entity name. Returns `SearchResultItem` shape, BUT `score` field
> is `recall_count` (i64 cast to f64), NOT a 0.0-1.0 similarity score —
> entity-tag lookup has no relevance signal. Results ordered by
> `recall_count` desc (most-consulted facts surface first). Use this for
> "show me everything related to <X>" queries when `memory_search`'s
> semantic match isn't specific enough. Scoped to current project unless
> `scope='global'`.

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `entity_name` | `String` | yes | Lowercased canonical form (ingestion lowercases at write time; callers should pass the lowercased term) |
| `project_id` | `String?` | no | Override the server's startup-cached `default_project_id` for this lookup's scope. |
| `scope` | `String?` | no | `"global"` for cross-project lookup; any other value treated as current-project. |

## Returns

```json
{
  "entity_name": "auth",
  "facts": [
    {
      "id": "01h9mz1234567890abcdef",
      "short_id": "01h9mz12",
      "title": "Switched auth middleware from JWT to session cookies",
      "source_file": "docs/decisions/021-auth-middleware.md",
      "source_type": "conclusion",
      "knowledge_type": "decisional",
      "entities": "auth,middleware,session,jwt,csrf",
      "score": 12.0,
      "valid_from": "2026-04-12T00:00:00Z",
      "snippet": "After CSRF surface review, switched production auth from JWT to httpOnly session cookies..."
    },
    {
      "id": "01h8_older_auth_eval",
      "short_id": "01h8older",
      "title": "Initial auth approach evaluation",
      "...": "...",
      "score": 3.0
    }
  ],
  "error": null
}
```

**Important field semantics** — `score` is **recall_count**, NOT a similarity score:

- `score = 12.0` means this fact has been recalled (via `memory_search` or `memory_get`) 12 times.
- Ordering is `recall_count DESC` — most-consulted facts surface first.
- Range is `[0, ∞)`, NOT `[0.0, 1.0]`. Do NOT compare against `memory_search`'s score field.

**Filtering**: only **VALID** facts are returned:
- `valid_until IS NULL` (not invalidated)
- `superseded_by IS NULL` (no replacement linked)

Invalidated or superseded facts are excluded. Use a direct DB query if you need to see them.

**`error`** — non-empty on DB error.

## Errors

| Condition | Behavior |
|---|---|
| Entity not found in scope | returns `facts: []` + `error: null` (empty is the no-match indicator, not an error) |
| DB error | returns `facts: []` + `error: "DB error: <details>"` (logged at error level) |

## Examples

### Example 1 — List all auth-tagged facts in current project

Request:

```json
{
  "name": "memory_entity_facts",
  "arguments": { "entity_name": "auth" }
}
```

Response: all valid facts tagged `auth` in the current project, ordered by `recall_count` desc.

### Example 2 — Cross-project lookup

Request:

```json
{
  "name": "memory_entity_facts",
  "arguments": {
    "entity_name": "rust",
    "scope": "global"
  }
}
```

Response: facts tagged `rust` across all projects.

### Example 3 — Entity not found

Request:

```json
{
  "name": "memory_entity_facts",
  "arguments": { "entity_name": "nonexistent-tag" }
}
```

Response: `{ "entity_name": "nonexistent-tag", "facts": [], "error": null }`.

## Notes

**When to use `memory_entity_facts` vs `memory_search`**:

- `memory_search` — semantic match on a query string. Returns top-K ranked by hybrid FTS5 + vector relevance. Use for "find me decisions related to <topic>".
- `memory_entity_facts` — exact entity-tag lookup. Returns ALL valid facts with the tag (no top-K cap). Use for "show me everything we've decided about <X>" when you know `<X>` is a canonical entity tag.

The two tools are complementary: `memory_search` is fuzzy + relevance-ranked; `memory_entity_facts` is exact + completeness-oriented.

**Entity name canonicalization**:
- Ingestion lowercases entity tags before storage (e.g., `"Auth"` and `"AUTH"` both become `"auth"`).
- Callers must pass the lowercased form. The tool does NOT lowercase its input — case mismatch returns empty.
- Tags with hyphens, underscores, or multi-word entities are stored as-typed (after lowercase): `"auth-middleware"` and `"auth middleware"` are distinct entities.

**Score field misuse warning**:
- A common mistake: aggregating `memory_search.score` and `memory_entity_facts.score` in the same calling code as if they were the same scale. They are NOT.
- `memory_search.score` is RRF-derived hybrid relevance (0.0-1.0 typical).
- `memory_entity_facts.score` is `recall_count` (integer, 0-∞).
- Always check which tool returned the result before reasoning about score.

**Why no top-K limit**:
- `memory_entity_facts` returns all matching facts because the caller usually wants completeness ("show me ALL").
- For very high-cardinality tags (e.g., a project-wide tag attached to thousands of facts), this can return large result sets. Caller should consider whether `memory_search` with a query containing the tag would be more targeted.

**Data source**:
- Uses the normalized `entities` + `fact_entity` link tables (indexed lookup), not the comma-separated `entities` TEXT column on `memory_entries` (which would require LIKE-scan). Performance is O(log n) per lookup.
