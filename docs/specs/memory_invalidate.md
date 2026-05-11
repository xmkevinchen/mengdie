---
title: "spec: memory_invalidate"
type: spec
created: 2026-05-08
as_of_commit: 1b48c92
stability: draft-until-v0.0.1
implementation_status: current
audience: [operator, llm-at-mcp-tool-discovery]
---

# spec: `memory_invalidate` (MCP tool)

Mark a memory as no longer valid. Optionally link to a superseding memory.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn invalidate(&self, Parameters(params): Parameters<InvalidateParams>)
    -> Json<InvalidateOutput>
```

**MCP tool description** (verbatim, 33 words / ~50 tokens):

> Mark a memory as no longer valid. Set `superseded_by` when a newer
> memory replaces it — links the records for traceability. The `reason`
> field is persisted for audit.

(Comfortably under 200-token cap.)

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `entry_id` | `String` | yes | ID of the memory entry to invalidate |
| `reason` | `String` | yes | Free-text reason; persisted for audit |
| `superseded_by` | `String?` | no | ID of the memory that supersedes this one (when applicable) |

**`reason` semantics**:
- Operator-facing: "memory was based on incorrect assumption", "decision reversed by discussion 028", etc.
- LLM-facing (if invoked from agent context): the agent should produce a 1-2 sentence rationale.
- No length limit enforced at MCP layer (DB column is TEXT). Convention: ≤ 500 chars.

**`superseded_by` semantics**:
- Pass when invalidating because a new memory replaces this one.
- Omit when invalidating without replacement (e.g., decision was retracted, not changed).

## Returns

```json
{
  "success": true,
  "entry_id": "01h8_old_id",
  "superseded_by": "01h9_new_id"
}
```

**Field semantics**:
- `success` — `true` if the DB row was updated (entry existed + was not already invalidated); `false` if entry didn't exist OR was already invalidated.
- `entry_id` — echoes input for caller convenience.
- `superseded_by` — echoes input if provided; `null` if omitted.

## Errors

| Condition | Behavior |
|---|---|
| Entry doesn't exist | returns `success: false` (no MCP error; idempotent semantics) |
| Entry already invalidated | returns `success: false` (no MCP error; idempotent semantics) |
| DB error | returns `success: false` + `superseded_by: null` (logged at error level) |
| Missing `entry_id` or `reason` | rejected at deserialization (rmcp parse error) |

**Idempotent design**: invalidating a non-existent or already-invalidated entry is NOT an error — `success: false` is the indicator. This makes caller logic simpler (no need to check existence before invalidating).

## Examples

### Example 1 — Invalidate with supersession

Request:

```json
{
  "name": "memory_invalidate",
  "arguments": {
    "entry_id": "01h8_old_v0.0.1_thesis",
    "reason": "Operator clarified narrower scope on 2026-05-05; superseded by new thesis memory",
    "superseded_by": "01h9_new_v0.0.1_thesis"
  }
}
```

Response:

```json
{
  "success": true,
  "entry_id": "01h8_old_v0.0.1_thesis",
  "superseded_by": "01h9_new_v0.0.1_thesis"
}
```

### Example 2 — Invalidate without supersession (retraction)

Request:

```json
{
  "name": "memory_invalidate",
  "arguments": {
    "entry_id": "01h7_speculative_idea",
    "reason": "Discussion 029 concluded this approach is not viable; retracting without replacement"
  }
}
```

Response:

```json
{
  "success": true,
  "entry_id": "01h7_speculative_idea",
  "superseded_by": null
}
```

### Example 3 — Idempotent (already invalidated)

Request:

```json
{
  "name": "memory_invalidate",
  "arguments": {
    "entry_id": "01h7_already_gone",
    "reason": "Cleanup pass"
  }
}
```

Response:

```json
{
  "success": false,
  "entry_id": "01h7_already_gone",
  "superseded_by": null
}
```

Caller treats `success: false` as a no-op (the desired state — entry not valid — already holds).

## Notes

**Invalidation semantics**:
- DB-level: sets `valid_until` to now (UTC) and writes `invalidation_reason` + optional `superseded_by` columns.
- Search behavior: invalidated memories are filtered out of `memory_search` results (per current implementation; bi-temporal model would expose them under specific time-window queries — that's a v1.0 candidate per `docs/roadmap.md`).
- The original memory row is NOT deleted — invalidation is logical, not physical. Rationale: audit trail + future bi-temporal upgrade compatibility.

**Atomic alternative**: when invalidating because a new memory supersedes this one, prefer `memory_ingest(..., resolves=[entry_id])` — the atomic transaction guarantees insert + invalidate happen together. Use `memory_invalidate` when:
- Retracting without replacement (no new memory)
- Linking to an already-existing memory (no new ingest needed)
- Bulk cleanup operations

**Audit trail**:
- Invalidation row carries `invalidation_reason` and `superseded_by` columns indefinitely.
- Future `memory_contradictions` MCP tool (v1.0 candidate per roadmap) will expose these for retrospective analysis.

**Cross-project**:
- `memory_invalidate` does NOT take a `project_id` param — operates on the global `entry_id` namespace.
- Caller responsible for ensuring the `entry_id` belongs to the intended project.
- Defensive practice: invoke `memory_search` first to confirm the target memory's project, then invalidate.

**Concurrent invalidation**:
- Two concurrent invalidate calls on the same entry: the second sees `success: false` (already invalidated). No data corruption.
- Two concurrent invalidate calls with different `superseded_by` values: the first wins (DB constraint); the second sees `success: false`. Caller responsible for resolving.
