---
title: "spec: memory_invalidate"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_invalidate` (MCP tool)

Mark a memory as no longer valid. Optionally link to a superseding memory.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn invalidate(&self, Parameters(params): Parameters<InvalidateParams>)
    -> Json<InvalidateOutput>
```

**MCP tool description** (verbatim):

> Mark a memory as no longer valid. Set `superseded_by` when a newer
> memory replaces it — links the records for traceability. The `reason`
> field is persisted for audit. Accepts either a full UUID (36 chars)
> or an 8+ char prefix; collision returns an error listing matches.

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `entry_id` | `String` | yes | Full UUID (36 chars) OR 8+ char prefix; prefix is scoped to the resolved `project_id` |
| `reason` | `String` | yes | Free-text reason; persisted for audit |
| `superseded_by` | `String?` | no | ID of the memory that supersedes this one (when applicable) |
| `project_id` | `String?` | no | Override the server's startup-cached `default_project_id` for this call's scope. `None` or absent JSON key → fallback to server default. `Some("")` is normalized to `None` (stale-template safety). |

**`reason` semantics**:
- Operator-facing: "memory was based on incorrect assumption", "decision reversed after security review", etc.
- LLM-facing (if invoked from agent context): the agent should produce a 1-2 sentence rationale.
- No length limit enforced at MCP layer (DB column is TEXT). Convention: ≤ 500 chars.

**`superseded_by` semantics**:
- Pass when invalidating because a new memory replaces this one.
- Omit when invalidating without replacement (e.g., decision was retracted, not changed).

**`project_id` semantics**:
- Caller-authority precedence: `params.project_id (non-empty)` wins over the server's startup-cached `default_project_id` for prefix-lookup scope AND the full-UUID cross-project guard.
- `Some("")` is filtered to `None` and falls through to the default — protects against stale-template callers passing empty-string sentinels.
- **Wire-compatibility**: callers that omit the field continue to work unchanged (`#[serde(default)]` resolves the absent JSON key to `None`).
- **Cross-project guard** on the full-UUID path: if the resolved scope mismatches the fetched entry's project, returns `success: false` with an error that differentiates "no override was passed" vs "override was passed but mismatches".
- **Empty-string normalization is currently `memory_invalidate`-only**: the other 6 MCP tools still pass `Some("")` through. Cross-tool unification is a tracked follow-up (see [technical-design.md §4 Known Problem 1](../technical-design.md)).

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
| Prefix too short (< 8 chars, not full UUID) | returns `success: false` + `error: "Prefix '<x>' is too short (need ≥ 8 chars, or pass a full 36-char UUID)"` |
| Prefix collision (matches multiple memories) | returns `success: false` + `error: "Prefix '<x>' is ambiguous; matches at least: <id1>, <id2>; extend prefix to disambiguate"` |
| Cross-project mismatch (full UUID, resolved scope ≠ entry's project) | returns `success: false` + `error: "Memory '<id>' belongs to project '<x>', not '<y>'; ..."` (wording differs based on whether override was passed) |
| DB error | returns `success: false` + `error: "DB error during ..."` (logged at error level) |
| Missing `entry_id` or `reason` | rejected at deserialization (rmcp parse error) |

**Idempotent design**: invalidating a non-existent or already-invalidated entry is NOT an error — `success: false` is the indicator. This makes caller logic simpler (no need to check existence before invalidating).

## Examples

### Example 1 — Invalidate with supersession

Request:

```json
{
  "name": "memory_invalidate",
  "arguments": {
    "entry_id": "01h8_old_auth_decision",
    "reason": "Security review on 2026-05-12 reversed the JWT acceptability claim; superseded by new auth decision",
    "superseded_by": "01h9_new_auth_decision"
  }
}
```

Response:

```json
{
  "success": true,
  "entry_id": "01h8_old_auth_decision",
  "superseded_by": "01h9_new_auth_decision"
}
```

### Example 2 — Invalidate without supersession (retraction)

Request:

```json
{
  "name": "memory_invalidate",
  "arguments": {
    "entry_id": "01h7_speculative_idea",
    "reason": "Followup review concluded this approach is not viable; retracting without replacement"
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

### Example 4 — Cross-project guard (full UUID with override mismatch)

Request:

```json
{
  "name": "memory_invalidate",
  "arguments": {
    "entry_id": "01h9_full_uuid_belonging_to_project_a",
    "reason": "test",
    "project_id": "project-b"
  }
}
```

Response:

```json
{
  "success": false,
  "entry_id": "01h9_full_uuid_belonging_to_project_a",
  "superseded_by": null,
  "error": "Memory '...' belongs to project 'project-a', not 'project-b'; project_id override was supplied but does not match — pass project_id='project-a' to target the memory's actual project"
}
```

## Notes

**Invalidation semantics**:
- DB-level: sets `valid_until` to now (UTC) and writes `invalidation_reason` + optional `superseded_by` columns.
- Search behavior: invalidated memories are filtered out of `memory_search` results.
- The original memory row is NOT deleted — invalidation is logical, not physical. Rationale: audit trail + future bi-temporal upgrade compatibility.

**Atomic alternative**: when invalidating because a new memory supersedes this one, prefer `memory_ingest(..., resolves=[entry_id])` — the atomic transaction guarantees insert + invalidate happen together. Use `memory_invalidate` when:
- Retracting without replacement (no new memory)
- Linking to an already-existing memory (no new ingest needed)
- Bulk cleanup operations

**Audit trail**:
- Invalidation row carries `invalidation_reason` and `superseded_by` columns indefinitely.

**Prefix resolution**:
- Full 36-char UUID → direct fetch (cross-project guard checks scope post-fetch).
- 8+ char hex prefix → scoped prefix lookup; collision returns ambiguous error; no match returns "not found in project '<scope>'".
- Prefix lookup is always scoped to the resolved `project_id` (no cross-project prefix search).

**Cross-project guard rationale**:
- Full-UUID branch fetches the entry and verifies `entry.project_id == resolved_scope` before invalidating.
- Mirrors `memory_get`'s cross-project guard pattern — destructive operations get the same project-boundary semantics as read operations.
- DB-layer SQL is intentionally NOT project-scoped (`Db::invalidate_memory` SQL has no `project_id` predicate); defense-in-depth lives at the MCP layer. See [technical-design.md §2.3 invariant I6](../technical-design.md) for the asymmetry rationale.

**Concurrent invalidation**:
- Two concurrent invalidate calls on the same entry: the second sees `success: false` (already invalidated). No data corruption.
- Two concurrent invalidate calls with different `superseded_by` values: the first wins (DB constraint); the second sees `success: false`. Caller responsible for resolving.
