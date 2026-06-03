---
title: "spec: memory_lint"
type: spec
last_updated: 2026-05-23
stability: stable
implementation_status: current
audience: [operator, host-AI-tool-at-MCP-tool-discovery]
---

# spec: `memory_lint` (MCP tool)

Run deterministic health checks on the mengdie DB: orphan GC, unresolved contradictions, embedding drift. Read-only; detection only — never mutates.

## Signature

**Rust** (`src/core/mcp_tools.rs`):

```rust
async fn lint(&self, Parameters(params): Parameters<LintParams>)
    -> Json<LintReport>
```

**MCP tool description** (verbatim):

> Run Memory Lint deterministic health checks: orphan GC (dangling
> references in `superseded_by` / `synthesis_links` / `audit_returned_facts`),
> unresolved contradictions (half-resolved supersession state + size-2
> cycles + high-entity-overlap unsuperseded pairs), embedding drift
> (NULL on non-synthesis + dim mismatch + synthesis-row NULL embedding
> surface). Read-only, idempotent (same DB → same findings except
> `generated_at`). Scoped to current project unless `scope='global'`.

## Params

| Field | Type | Required | Constraint |
|---|---|---|---|
| `project_id` | `String?` | no | Override the server's startup-cached `default_project_id` for this scan's scope. |
| `scope` | `String?` | no | `"global"` for cross-project scan; any other value treated as current-project. |

## Returns

```json
{
  "generated_at": "2026-05-23T14:30:00Z",
  "orphan_gc": {
    "superseded_by_dangling_count": 0,
    "superseded_by_dangling": [],
    "synthesis_links_orphan_count": 0,
    "synthesis_links_orphan": [],
    "audit_facts_orphan_count": 0,
    "audit_facts_orphan": []
  },
  "unresolved_contradictions": {
    "half_v_only_count": 0,
    "half_v_only": [],
    "half_s_only_count": 0,
    "half_s_only": [],
    "circular_count": 0,
    "circular": [],
    "entity_overlap_unsuperseded_count": 2,
    "entity_overlap_unsuperseded": [
      { "fact_a": "01h9_a...", "fact_b": "01h9_b...", "overlap": 0.83 }
    ]
  },
  "embedding_drift": {
    "embedding_null_non_synthesis_count": 0,
    "embedding_null_non_synthesis": [],
    "embedding_dim_mismatch_count": 0,
    "embedding_dim_mismatch": [],
    "synthesis_null_embedding_count": 0,
    "synthesis_null_embedding": []
  },
  "error": null
}
```

**Three check categories**:

### `orphan_gc` — dangling foreign-key references

- `superseded_by_dangling` — `memory_entries.superseded_by` points to a non-existent ID.
- `synthesis_links_orphan` — `memory_synthesis_links` rows whose `source_fact_id` or `synthesis_fact_id` no longer exists.
- `audit_facts_orphan` — `audit_returned_facts` rows whose `fact_id` no longer exists (memory was deleted post-audit).

Each list contains compact identifier strings (UUIDs or `"fact_id→synthesis_id"` / `"fact_id (audit=N)"` format).

### `unresolved_contradictions` — supersession-state integrity

- `half_v_only` — `valid_until` set but `superseded_by` NULL (memory invalidated without a replacement linked — may be intentional, may be a bookkeeping gap).
- `half_s_only` — `superseded_by` set but `valid_until` NULL (replacement linked but memory still marked valid — definitely a bug).
- `circular` — size-2 supersession cycle: `A.superseded_by = B` AND `B.superseded_by = A`.
- `entity_overlap_unsuperseded` — active facts (`valid_until` NULL) with ≥0.7 Jaccard entity-tag overlap that have no supersession link between them. Surfaces likely candidate contradictions missed by the ingest-time check.

### `embedding_drift` — vector index sanity

- `embedding_null_non_synthesis` — `embedding IS NULL AND source_type != 'synthesis'`. Non-synthesis facts should always have embeddings.
- `embedding_dim_mismatch` — `embedding_dim != 384` (the canonical dimension for `all-MiniLM-L6-v2`).
- `synthesis_null_embedding` — synthesis rows with `embedding = NULL`. Recoverable via `mengdie reembed-synthesis`.

**`error`** — non-empty when the lint pass aborted partway (lock poisoned, schema mismatch, SQL error). All-zero counts WITH `error` set means the lint pass aborted; all-zero counts WITHOUT `error` means the DB is genuinely clean.

## Errors

| Condition | Behavior |
|---|---|
| DB error mid-scan | returns partial counts + `error: "<details>"` (logged at error level) |

No other soft-fail conditions — `memory_lint` is read-only.

## Examples

### Example 1 — Clean DB

Request:

```json
{ "name": "memory_lint", "arguments": {} }
```

Response: all `_count` fields are `0`, all list fields are `[]`, `error: null`.

### Example 2 — Embedding drift detected

Response excerpt:

```json
{
  "embedding_drift": {
    "embedding_null_non_synthesis_count": 0,
    "embedding_dim_mismatch_count": 0,
    "synthesis_null_embedding_count": 5,
    "synthesis_null_embedding": ["01h8_a...", "01h8_b...", "01h8_c...", "01h8_d...", "01h8_e..."]
  }
}
```

Recovery: run `mengdie reembed-synthesis` to backfill the 5 synthesis rows.

### Example 3 — Likely contradiction surfaced

Response excerpt:

```json
{
  "unresolved_contradictions": {
    "entity_overlap_unsuperseded_count": 1,
    "entity_overlap_unsuperseded": [
      { "fact_a": "01h9_auth_v1", "fact_b": "01h9_auth_v2_implicit", "overlap": 0.91 }
    ]
  }
}
```

Operator action: review the two memories; either link them via `memory_invalidate(..., superseded_by=...)` or confirm they're independently valid (high overlap is a heuristic, not ground truth).

## Notes

**Idempotent + read-only**: running this tool on an unchanged DB produces byte-identical output except for the `generated_at` timestamp. Run it repeatedly without side effects.

**Determinism**: every check is pure SQL — no LLM judgment. The Jaccard overlap threshold (`0.7`) and the canonical embedding dimension (`384`) are hardcoded constants in `src/core/lint.rs`; future changes would be schema migrations, not config.

**Cost characteristics**:
- Orphan GC: 3 LEFT JOIN scans, O(n) per relationship table.
- Unresolved contradictions: half-state checks are O(n); entity-overlap pairwise check is O(n²/2) over active facts in scope. Becomes expensive on corpora > 10K active rows.
- Embedding drift: 3 indexed scans, O(n).

For large corpora, prefer scoping to a single project (`scope` omitted) rather than `scope: "global"`.

**Detection only**: this tool never writes. To act on findings, the operator (or LLM agent) must follow up with `memory_invalidate`, `mengdie reembed-synthesis`, or direct DB cleanup. The separation keeps the lint pass safe to run on schedule.

**Composing with `mengdie audit-stats` CLI**: `memory_lint` checks DB integrity; `mengdie audit-stats` reports search-pipeline observability. Different surfaces, no overlap.
