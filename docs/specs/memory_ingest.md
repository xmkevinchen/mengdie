---
title: "spec: memory_ingest"
type: spec
created: 2026-05-08
as_of_commit: 1b48c92
stability: draft-until-v0.0.1
implementation_status: current
audience: [operator, llm-at-mcp-tool-discovery]
---

# spec: `memory_ingest` (MCP tool)

Ingest a new memory into Mengdie. Optionally invalidate prior memories atomically.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn ingest(&self, Parameters(params): Parameters<IngestParams>)
    -> Json<IngestOutput>
```

**MCP tool description** (verbatim, 60 words / ~95 tokens):

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
| `source_file` | `String` | no (default: `""`) | path to original file (e.g., `docs/discussions/027/conclusion.md`); empty string OK if not applicable |
| `source_type` | `SourceType` enum | yes | one of: `conclusion` \| `review` \| `plan` \| `retrospect` \| `synthesis` |
| `knowledge_type` | `KnowledgeType` enum | yes | one of: `decisional` \| `experiential` \| `factual` |
| `entities` | `String` | yes | ≤ 1,000 chars; comma-separated entity tags; ASCII recommended (Chinese OK but FTS5 trigram tokenization required for matching) |
| `project_id` | `String?` | no | overrides default project_id (inferred from cwd at server startup) |
| `resolves` | `Vec<String>?` | no | IDs of existing memories to invalidate atomically (see Notes) |

**Enum semantics**:
- `SourceType::Conclusion` — AE pipeline conclusion.md output (a discussion that concluded with decisions)
- `SourceType::Review` — AE pipeline review.md output (a feature review verdict)
- `SourceType::Plan` — AE pipeline plan.md output (a feature plan)
- `SourceType::Retrospect` — AE pipeline retrospect.md output (a project-level reflection)
- `SourceType::Synthesis` — Mengdie-generated synthesis (output of `mengdie dream --synthesize`); marks LLM-generated meta-fact, not primary source
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

**`entry_id`** — opaque ID assigned to the new memory (empty string on error)

**`conflicts`** — detected conflicts based on entity-tag overlap + recency:
- `evolution_candidate` — high similarity + same entity tags (likely the new memory supersedes an older one)
- `recent_conflict` — recent memory with same entity tags but content disagreement (surface to user before resolving)

When conflicts are detected and `resolves` was NOT passed, the caller SHOULD prompt the user before invalidating. When `resolves` was passed, conflicts in the response are informational only (the listed IDs were invalidated atomically).

**`error`** — non-null indicates ingestion failed:
- `"content too long (max 100000 chars)"` — content exceeded limit
- `"field too long (max 1000 chars)"` — title or entities exceeded limit
- `"ingestion failed"` — internal error (logged); caller should retry

## Errors

| Condition | Behavior |
|---|---|
| `content.len() > 100_000` | returns empty entry_id + `error: "content too long ..."` |
| `title.len() > 1_000` OR `entities.len() > 1_000` | returns empty entry_id + `error: "field too long ..."` |
| Embedder failure | hard error (per F-003 Topic 4 — was soft "store without embedding" pre-F-003) |
| DB transaction failure | returns empty entry_id + `error: "ingestion failed"` (logged at error level) |
| Invalid `source_type` / `knowledge_type` enum | rejected at deserialization (rmcp returns parse error to host) |

**Behavior change vs v0.x**: pre-F-003 MCP path soft-failed embed errors (stored memory without embedding). v0.0.1 hard-fails to converge with file-ingest path's behavior.

## Examples

### Example 1 — Simple ingest, no conflicts

Request:

```json
{
  "name": "memory_ingest",
  "arguments": {
    "title": "F-004 council validates v0.0.1 narrow OSS-adoption",
    "content": "Council survey confirms mem0/Letta/Graphiti are MCP-aware but pursue different technical paths...",
    "source_file": ".ae/features/active/F-004-project-doc-structure-overhaul/analysis.md",
    "source_type": "conclusion",
    "knowledge_type": "decisional",
    "entities": "F-004,v0.0.1,narrow-oss-adoption,survey"
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
    "title": "v0.0.1 thesis: narrow OSS-adoption (revised 2026-05-05)",
    "content": "Per operator clarification: minimum-viable AE-brain that avoids re-inventing wheels...",
    "source_type": "conclusion",
    "knowledge_type": "decisional",
    "entities": "v0.0.1,thesis,oss-adoption",
    "resolves": ["01h8_old_thesis_full_replace", "01h8_older_thesis_repair"]
  }
}
```

Response:

```json
{
  "entry_id": "01h9_new_thesis",
  "conflicts": [],
  "error": null
}
```

Both old memories are invalidated with `superseded_by: 01h9_new_thesis` in one DB transaction.

## Notes

**Conflict resolution workflow** (per server `instructions` field):

1. Caller invokes `memory_ingest` without `resolves`. Response includes `conflicts: [{id, title, reason}, ...]`.
2. For `evolution_candidate` conflicts (high similarity + same entity tags) — caller may invoke `memory_invalidate(entry_id, superseded_by=new_entry_id)` to link old → new.
3. For `recent_conflict` conflicts — caller MUST surface to the user before resolving (the new memory may genuinely contradict older memory, requiring human judgment).
4. **Atomic alternative**: pass `resolves=[id, ...]` to `memory_ingest` to insert + invalidate in one transaction. Use this when caller has already decided which memories to supersede (e.g., based on prior `memory_search` + reasoning).

**Audit + metrics**:
- Every successful ingest increments `METRIC_INGEST_COUNT`.
- Every ingest with non-empty conflicts increments `METRIC_CONFLICT_COUNT`.
- F-002 audit-returned-facts substrate does NOT fire on ingest (search-only invariant).

**Source-type epistemic weight**:
- `synthesis` source type marks LLM-generated meta-facts (output of dreaming pipeline); ae:analyze Round 0 injection MAY weight these lower than primary sources (`conclusion` / `review` / `plan` / `retrospect`).

**Project scoping**:
- `project_id` is the scope key for cross-project search separation.
- Default = inferred at server startup from cwd via git remote.
- Override via `project_id` param when host running outside a project tree.

**Provenance traceability**:
- `source_file` is the canonical "where this memory came from" pointer.
- F-002 audit substrate logs (search_call_id, returned_entry_ids) — not `source_file` directly — but `source_file` is preserved on every memory entry for downstream tooling.

**Embedding lifecycle**:
- fastembed is sync/blocking — wrapped in `tokio::task::spawn_blocking`.
- `Arc<Mutex<Embedder>>` is the embedder lifecycle pattern (preserved through F-002 + F-003 — plan AC10 invariant).

**Resolves semantics**:
- `resolves=[]` (or absent) → simple insert, no invalidation.
- `resolves=[id1, id2, ...]` → atomic transaction: insert new + set `superseded_by=new_id` on each listed ID + invalidate them.
- If any ID in `resolves` doesn't exist or is already invalidated, the entire transaction rolls back.
- This is the recommended path for caller-decided supersession (cleaner than 2-call insert-then-invalidate sequence).
