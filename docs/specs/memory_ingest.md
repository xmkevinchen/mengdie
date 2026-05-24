---
title: "spec: memory_ingest"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_ingest` (MCP tool)

Ingest a new memory into Mengdie. Optionally invalidate prior memories atomically.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn ingest(&self, Parameters(params): Parameters<IngestParams>)
    -> Json<IngestOutput>
```

**MCP tool description** (verbatim):

> Ingest a new memory into Mengdie. Returns `entry_id` and any detected
> conflicts (evolution candidates or recent conflicts with existing
> memories sharing entity tags). Pass `resolves=[id, ...]` to atomically
> insert this memory and invalidate the listed memories in one
> transaction. See server instructions for the full conflict resolution
> workflow.

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `title` | `String` | yes | ≤ 1,000 chars |
| `content` | `String` | yes | ≤ 100,000 chars |
| `source_file` | `String` | no (default: `""`) | path to original file; empty string OK if not applicable |
| `source_type` | `SourceType` enum | yes | one of: `conclusion` \| `review` \| `plan` \| `retrospect` \| `synthesis` |
| `knowledge_type` | `KnowledgeType` enum | yes | one of: `decisional` \| `experiential` \| `factual` |
| `entities` | `String` | yes | ≤ 1,000 chars; comma-separated entity tags; ASCII recommended (Chinese OK but FTS5 trigram tokenization required for matching) |
| `project_id` | `String?` | no | overrides default `project_id` (inferred from cwd at server startup) |
| `resolves` | `Vec<String>?` | no | IDs of existing memories to invalidate atomically (see Notes) |

**Enum semantics**:

- `SourceType::Conclusion` — output of a concluded design discussion (decisions with rationale)
- `SourceType::Review` — code/feature review verdict
- `SourceType::Plan` — feature plan with acceptance criteria
- `SourceType::Retrospect` — project-level reflection on shipped work
- `SourceType::Synthesis` — mengdie-generated synthesis row (output of the daily Dreaming pass); marks LLM-generated meta-fact, not a primary source
- `KnowledgeType::Decisional` — captures a decision and its rationale
- `KnowledgeType::Experiential` — captures observed outcomes from doing
- `KnowledgeType::Factual` — captures a factual finding (no decision component)

## Returns

```json
{
  "entry_id": "01h9mz...",
  "conflicts": [
    {
      "id": "01h8...",
      "title": "...",
      "reason": "evolution_candidate | recent_conflict"
    }
  ],
  "error": null
}
```

**`entry_id`** — full UUID v4 assigned to the new memory (empty string on error)

**`conflicts`** — detected conflicts based on entity-tag overlap + recency:
- `evolution_candidate` — high similarity + same entity tags (likely the new memory supersedes an older one)
- `recent_conflict` — recent memory with same entity tags but content disagreement (surface to user before resolving)

When conflicts are detected and `resolves` was NOT passed, the caller should prompt the user before invalidating. When `resolves` was passed, conflicts in the response are informational only (the listed IDs were invalidated atomically).

**`error`** — non-null indicates ingestion failed:
- `"content too long (max 100000 chars)"` — content exceeded limit
- `"field too long (max 1000 chars)"` — title or entities exceeded limit
- `"ingestion failed"` — internal error (logged); caller should retry

## Errors

| Condition | Behavior |
|---|---|
| `content.len() > 100_000` | returns empty entry_id + `error: "content too long ..."` |
| `title.len() > 1_000` OR `entities.len() > 1_000` | returns empty entry_id + `error: "field too long ..."` |
| Embedder failure | hard error — ingestion fails, no partial-stored memory |
| DB transaction failure | returns empty entry_id + `error: "ingestion failed"` (logged at error level) |
| Invalid `source_type` / `knowledge_type` enum | rejected at deserialization (rmcp returns parse error to host) |

## Examples

### Example 1 — Simple ingest, no conflicts

Request:

```json
{
  "name": "memory_ingest",
  "arguments": {
    "title": "JWT-to-session-cookie auth migration",
    "content": "After CSRF surface review, switched production auth from JWT to httpOnly session cookies...",
    "source_file": "docs/decisions/021-auth-middleware.md",
    "source_type": "conclusion",
    "knowledge_type": "decisional",
    "entities": "auth,middleware,session,jwt,csrf"
  }
}
```

Response:

```json
{
  "entry_id": "01h9mz_a7p_q3r",
  "conflicts": [],
  "error": null
}
```

### Example 2 — Atomic resolve (insert + invalidate in one transaction)

Request:

```json
{
  "name": "memory_ingest",
  "arguments": {
    "title": "Updated auth decision: session cookies confirmed for prod",
    "content": "Per security review, session cookies with httpOnly + SameSite=Strict are the prod choice. Prior 'JWT acceptable for v1' note is superseded.",
    "source_type": "conclusion",
    "knowledge_type": "decisional",
    "entities": "auth,middleware,session",
    "resolves": ["01h8_old_jwt_acceptable", "01h8_older_auth_eval"]
  }
}
```

Response:

```json
{
  "entry_id": "01h9_new_auth_decision",
  "conflicts": [],
  "error": null
}
```

Both old memories are invalidated with `superseded_by: 01h9_new_auth_decision` in one DB transaction. If any UPDATE fails, the entire transaction rolls back.

## Notes

**Conflict resolution workflow** (per server `instructions` field):

1. Caller invokes `memory_ingest` without `resolves`. Response includes `conflicts: [{id, title, reason}, ...]`.
2. For `evolution_candidate` conflicts (high similarity + same entity tags) — caller may invoke `memory_invalidate(entry_id, superseded_by=new_entry_id)` to link old → new.
3. For `recent_conflict` conflicts — caller should surface to the user before resolving (the new memory may genuinely contradict older memory, requiring human judgment).
4. **Atomic alternative**: pass `resolves=[id, ...]` to `memory_ingest` to insert + invalidate in one transaction. Use this when caller has already decided which memories to supersede (e.g., based on prior `memory_search` + reasoning).

**Audit + metrics**:
- Every successful ingest increments `METRIC_INGEST_COUNT`.
- Every ingest with non-empty conflicts increments `METRIC_CONFLICT_COUNT`.
- The `memory_search_audit` substrate does NOT fire on ingest (search-only invariant).

**Source-type epistemic weight**:
- `synthesis` source type marks LLM-generated meta-facts (output of daily Dreaming pass); downstream consumers may weight these lower than primary sources (`conclusion` / `review` / `plan` / `retrospect`).

**Project scoping**:
- `project_id` is the scope key for cross-project search separation.
- Default = inferred at server startup from cwd via git remote.
- Override via `project_id` param when MCP host running outside a project tree.

**Provenance traceability**:
- `source_file` is the canonical "where this memory came from" pointer.
- Preserved on every memory entry for downstream tooling (lint, audit, retrieval display).

**Embedding lifecycle**:
- fastembed is sync/blocking — wrapped in `tokio::task::spawn_blocking` per cross-cutting invariant I2.
- `Arc<Mutex<Embedder>>` shared across handlers.

**Resolves semantics**:
- `resolves=[]` (or absent) → simple insert, no invalidation.
- `resolves=[id1, id2, ...]` → atomic transaction: insert new + set `superseded_by=new_id` on each listed ID + invalidate them.
- If any ID in `resolves` doesn't exist or is already invalidated, the entire transaction rolls back.
- Recommended path for caller-decided supersession (cleaner than the 2-call insert-then-invalidate sequence).
