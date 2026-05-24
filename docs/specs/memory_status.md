---
title: "spec: memory_status"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_status` (MCP tool)

DB health snapshot — entry counts, last ingest timestamp, persistent metrics, audit-pipeline stats. Read-only.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn status(&self, Parameters(params): Parameters<StatusParams>)
    -> Json<StatusOutput>
```

**MCP tool description** (verbatim):

> Snapshot of mengdie DB health: entry counts (total / longterm /
> synthesis / per-source-type), last ingest timestamp, operational
> counters (search/ingest/conflict/audit-failure), and audit-pipeline
> stats. Read-only. Scoped to current project by default; pass
> `scope='global'` for cross-project totals. Use this to distinguish
> 'DB is empty' from 'query missed' when memory_search returns nothing,
> or to verify the audit pipeline is healthy.

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `project_id` | `String?` | no | Override the server's startup-cached `default_project_id` for this snapshot's scope. |
| `scope` | `String?` | no | `"global"` for cross-project totals; any other value treated as current-project. |

## Returns

```json
{
  "project_id": "proj_abc123",
  "total_entries": 1247,
  "longterm_count": 312,
  "synthesis_count": 41,
  "by_source_type": {
    "conclusion": 432,
    "review": 287,
    "plan": 156,
    "retrospect": 18,
    "synthesis": 41,
    "factual": 313
  },
  "last_ingest_at": "2026-05-23T14:22:09Z",
  "metrics": {
    "search_count": 8431,
    "search_nonempty_count": 7920,
    "ingest_count": 1247,
    "conflict_count": 93,
    "audit_write_failures": 0
  },
  "audit": {
    "total_audits": 8431,
    "first_audit_at": "2026-04-28T09:14:01Z",
    "last_audit_at": "2026-05-23T14:21:55Z",
    "audit_returned_facts_count": 32104
  },
  "embedding_model": "all-MiniLM-L6-v2",
  "embedding_dim": 384,
  "error": null
}
```

**Field semantics**:

- `project_id` — scope context for this snapshot. `"<global>"` when caller passed `scope: "global"`; otherwise the resolved `project_id`.
- `total_entries` — count of all rows in `memory_entries` (valid + invalidated) in the scope.
- `longterm_count` — count of rows with `is_longterm = true` (promoted by Dreaming).
- `synthesis_count` — count of rows with `source_type = synthesis` (Dreaming-generated).
- `by_source_type` — per-`source_type` counts. Missing keys = 0.
- `last_ingest_at` — `MAX(created_at)` in scope; `null` if DB is empty in scope.
- `metrics` — persistent counters from the `metrics` table. **Always global** (counters are process-wide, not project-scoped).
- `audit` — nested audit-pipeline snapshot. **Always global** (audit table is not project-scoped at row level).
- `embedding_model` — model name used by this `mengdie-mcp` instance (hardcoded to `"all-MiniLM-L6-v2"` today).
- `embedding_dim` — canonical embedding dimension this instance expects (`384` for all-MiniLM-L6-v2).
- `error` — non-empty when status could not be assembled (DB error).

## Errors

| Condition | Behavior |
|---|---|
| DB error | returns mostly-empty status + `error: "DB error: <details>"` (logged at error level) |

This tool has no required input fields and no soft-fail conditions besides DB error.

## Examples

### Example 1 — Current project snapshot

Request:

```json
{ "name": "memory_status", "arguments": {} }
```

Response: as shown in **Returns** above.

### Example 2 — Global counts

Request:

```json
{
  "name": "memory_status",
  "arguments": { "scope": "global" }
}
```

Response: `project_id: "<global>"`, totals span all projects.

## Notes

**Operational use cases**:
- **"Is the DB empty or did the query miss?"** — `memory_search` returning empty + `memory_status` showing `total_entries: 0` confirms an empty DB; non-zero `total_entries` means the query didn't match anything in a populated corpus.
- **"Is the audit pipeline healthy?"** — `audit.total_audits` should be ≥ `metrics.search_count`. Divergence indicates audit writes failing; check `metrics.audit_write_failures`.
- **"Did Dreaming run today?"** — `synthesis_count` should grow daily as new clusters are consolidated. Stalled growth signals a Dreaming pass failure.
- **"Why is search feeling stale?"** — compare `embedding_model` field against the version your corpus was last ingested under. A model swap without reembed leaves stale embeddings — see `mengdie reembed-synthesis` for the backfill CLI.

**Read-only contract**: this tool never writes to the DB. No `recall_count` bumps, no metric increments, no audit row. It only reads + assembles a snapshot.

**Counters are global**: `metrics` and `audit` are NOT project-scoped, even when `project_id` is set. The reason: these counters are process-wide and not partitioned at the row level in the current schema. If multi-operator scope is ever pursued, this asymmetry would need addressing (counters would need per-project partitioning).
