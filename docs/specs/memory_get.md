---
title: "spec: memory_get"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_get` (MCP tool)

Fetch the full content of a single memory by ID. Returns the complete fact (not a snippet) plus provenance, validity, supersession, and recall stats.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn get(&self, Parameters(params): Parameters<GetParams>)
    -> Json<GetOutput>
```

**MCP tool description** (verbatim):

> Fetch the full content of a single memory by ID. Returns the complete
> fact (not a 200-char snippet) plus provenance, validity, supersession,
> and recall stats. Side effect: increments recall_count + last_recalled
> (avg_relevance is NOT touched — direct lookup has no meaningful
> relevance score). Accepts either a full UUID (36 chars) or an 8+ char
> prefix; prefix is scoped to the current project unless scope='global'.
> Use after memory_search to expand a cited fact.

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `memory_id` | `String` | yes | Full UUID (36 chars) OR 8+ char prefix. Prefix lookup is scoped to the resolved `project_id` unless `scope: "global"` overrides. |
| `project_id` | `String?` | no | Override the server's startup-cached `default_project_id` for this call's scope. |
| `scope` | `String?` | no | `"global"` to ignore project scoping (prefix can match any project); any other value treated as current-project. |

**Prefix lookup semantics**:
- Full 36-char UUID → direct fetch.
- 8+ char hex prefix → scoped lookup; returns up to 2 matches; > 1 match is reported as "ambiguous".
- < 8 chars → rejected ("too short").

## Returns

```json
{
  "entry": {
    "id": "01h9mz1234567890abcdef",
    "short_id": "01h9mz12",
    "project_id": "proj_abc123",
    "source_file": "docs/decisions/021-auth.md",
    "source_type": "conclusion",
    "knowledge_type": "decisional",
    "title": "...",
    "content": "FULL content, not snippet",
    "entities": "auth,middleware,session",
    "valid_from": "2026-04-12T00:00:00Z",
    "valid_until": null,
    "superseded_by": null,
    "recall_count": 7,
    "avg_relevance": 0.62,
    "last_recalled": "2026-05-22T18:34:12Z",
    "embedding_dim": 384,
    "is_longterm": true,
    "created_at": "2026-04-12T00:00:00Z"
  },
  "error": null
}
```

**Field semantics**: returned `entry` mirrors the `MemoryEntry` struct in `src/core/db.rs` minus the `embedding` BLOB (which is ~1.5KB f32 and not useful over the MCP wire).

- `content` — **FULL content**, not a 200-char snippet. The whole point of `memory_get` is to expand a cited fact.
- `short_id` — first 8 hex chars of `id`; same citation contract as `memory_search`.
- `valid_until` + `superseded_by` — null on active memories; set when invalidated/superseded.
- `recall_count` + `last_recalled` — incremented as a side effect of this call.
- `embedding_dim` — `384` for memories embedded under the canonical model; `null` for synthesis rows that haven't been embedded yet (rare; see `mengdie reembed-synthesis`).

## Errors

| Condition | Behavior |
|---|---|
| `memory_id.len() < 8` (and not full UUID) | returns `entry: null` + `error: "Prefix '<x>' is too short (need ≥ 8 chars, or pass a full 36-char UUID)"` |
| Prefix collision (multiple matches) | returns `entry: null` + `error: "Prefix '<x>' is ambiguous; matches at least: <id1>, <id2>; extend prefix to disambiguate"` |
| Prefix not found in scope | returns `entry: null` + `error: "No memory matches prefix '<x>' in project '<scope>'; pass scope='global' to search across projects"` |
| Cross-project mismatch (full UUID, resolved scope ≠ entry's project, scope != "global") | returns `entry: null` + `error: "Memory '<id>' belongs to project '<x>', not '<y>'; pass scope='global' or set project_id explicitly"` |
| DB error | returns `entry: null` + `error: "DB error: <details>"` (logged at error level) |

## Examples

### Example 1 — Fetch by short prefix (current project)

Request:

```json
{
  "name": "memory_get",
  "arguments": { "memory_id": "01h9mz12" }
}
```

Response: returns the matching entry (full content). `recall_count` incremented.

### Example 2 — Fetch by full UUID, cross-project blocked

Request:

```json
{
  "name": "memory_get",
  "arguments": { "memory_id": "01h9_uuid_in_other_project" }
}
```

Response:

```json
{
  "entry": null,
  "error": "Memory '01h9_uuid_in_other_project' belongs to project 'project-b', not 'proj_abc123'; pass scope='global' or set project_id explicitly"
}
```

### Example 3 — Cross-project fetch via scope=global

Request:

```json
{
  "name": "memory_get",
  "arguments": {
    "memory_id": "01h9_uuid_in_other_project",
    "scope": "global"
  }
}
```

Response: returns the entry (no scope guard fires).

## Notes

**Side effect**:
- Increments `recall_count` and updates `last_recalled` to now (UTC) on every successful fetch.
- Does NOT touch `avg_relevance` — direct lookup has no relevance signal (the score comes from `memory_search`'s ranking, not from a fetch).

**Provenance contract**: callers using `memory_get` after a `memory_search` cite get the canonical `source_file`, `valid_from`, and `superseded_by` chain — sufficient to present "this fact was last updated when, with what supersession history" to a user or LLM.

**Embedding not returned**: the binary embedding is excluded from the MCP wire response to avoid ~1.5KB/call overhead. `embedding_dim` is preserved as a diagnostic flag (see `memory_lint` embedding drift check).

**Cross-project guard rationale**: same as `memory_invalidate`'s. Default scoping prevents accidental cross-project reads when the operator pastes a UUID without specifying scope.
