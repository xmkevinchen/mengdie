---
title: "spec: mengdie CLI"
type: spec
last_updated: 2026-05-23
stability: stable
audience: [operator]
---

# spec: `mengdie` CLI

Mengdie is an MCP server and CLI for AI-native knowledge memory — it
captures structured-markdown artifacts produced by AI development
workflows, lets agents query them back via MCP tools, and runs a daily
filtering pass that promotes frequently-recalled facts and consolidates
related ones via LLM synthesis. See
[`docs/technical-design.md`](technical-design.md) for the architecture.

The `mengdie` binary is the operator-facing CLI for direct database management,
batch import, and the Dreaming pipeline. The MCP server (`mengdie-mcp` separate
binary) is the agent-facing entry point and is specified separately under
[`docs/specs/`](specs/).

## Signature

**Binary**: `mengdie` (built from `src/bin/cli.rs`)

**Global options** (apply to all subcommands):

| Flag | Type | Default | Constraint |
|---|---|---|---|
| `--db-path <PATH>` | path | `~/.mengdie/db.sqlite` | overrides default DB location |

**Subcommands**:

```
mengdie dream                [--synthesize] [--decay-dry-run] [...flags]
mengdie import               --dir <PATH> [--dry-run]
mengdie list                 [--global] [--format table|json]
mengdie search <query>       [--global] [--limit N] [--min-score F]
mengdie rename               [from] [to] [--list] [--dry-run] [--yes]
mengdie stats
mengdie audit-stats          [--format table|json]
mengdie synthesis-audit <id>
mengdie reembed-synthesis    [--dry-run]
```

## Params (per subcommand)

### `dream`

Run the Dreaming pipeline (promotion + optional decay + optional LLM synthesis).

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `--min-recall <N>` | i64 | `dreaming::DEFAULT_MIN_RECALL` (currently 3) | Minimum `recall_count` for promotion |
| `--min-relevance <F>` | f64 | `dreaming::DEFAULT_MIN_RELEVANCE` (currently 0.45) | Minimum `avg_relevance` for promotion |
| `--window-days <N>` | i64 | `dreaming::DEFAULT_WINDOW_DAYS` (currently 14) | `last_recalled` recency window |
| `--synthesize` | flag | off | Run LLM synthesis after promotion (opt-in: makes network calls + writes synthesis rows) |
| `--threshold <F>` | f32 | `clustering::DEFAULT_THRESHOLD` (0.75) | Cosine clustering threshold |
| `--min-cluster-size <N>` | usize | `clustering::DEFAULT_MIN_SIZE` (currently 2) | Minimum cluster size for synthesis |
| `--max-cluster-size <N>` | usize | `20` | Cluster truncation cap (bounds LLM token budget) |
| `--dry-run` | flag | off | Show synthesis prompts without LLM calls or DB writes. **Requires `--synthesize`**. |
| `--decay-dry-run` | flag | off | Dry-run the exponential decay pass: report would-demote list without clearing `is_longterm`. Incompatible with `--synthesize --dry-run`. |
| `--project <ID>` | string | (all projects) | Project scope for synthesis |

### `import`

Batch-import structured markdown artifacts (`conclusion.md` / `review.md` /
`plan.md` / `retrospect.md`) from a directory.

| Flag | Type | Required | Semantics |
|---|---|---|---|
| `--dir <PATH>` | path | yes | Directory to scan recursively for ingestable files |
| `--dry-run` | flag | no | Preview without writing to DB |

### `list`

List all memories in the current (or global) scope.

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `--global` | flag | off | Show memories from all projects |
| `--format <FMT>` | string | `"table"` | Output format: `table` or `json` |

### `search <query>`

Debug-mode search (parallels `memory_search` MCP tool).

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `<query>` | positional | required | Search query text |
| `--global` | flag | off | Search globally (all projects) |
| `--limit <N>` | usize | `10` | Maximum results |
| `--min-score <F>` | f64 | (no filter) | Minimum score threshold |

### `rename`

Rename a `project_id` in the database.

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `[from]` | positional | optional (with `--list`) | Source project_id |
| `[to]` | positional | optional (with `--list`) | Target project_id |
| `--list` | flag | off | List all `project_id`s with memory counts (no rename) |
| `--dry-run` | flag | off | Preview without writing |
| `-y` / `--yes` | flag | off | Skip confirmation prompt |

### `stats`

Print observability metrics. No flags.

### `audit-stats`

Read the persistent search-audit substrate and print per-query / per-fact
recall stats. Useful for spotting under-recalled facts or pipeline drift.

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `--format <FMT>` | string | `"table"` | Output format: `table` or `json` |

### `synthesis-audit <id>`

Read-only audit of a synthesis row + its source memories. Operator-eyeball
fidelity check.

| Flag | Type | Required | Semantics |
|---|---|---|---|
| `<id>` | positional | yes | Synthesis memory ID to inspect |

### `reembed-synthesis`

Backfill embeddings for synthesis rows missing an `embedding` (most often
older synthesis rows created before embedding-on-write landed).

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `--project <ID>` | string | (all projects) | Limit backfill to a single `project_id`; omit to scan all projects in the global DB |
| `--dry-run` | flag | off | Report rows that WOULD be re-embedded without performing inference or DB writes. Skips embedder load for fast preview. |

## Returns (exit codes + stdout)

**Exit codes** (POSIX convention):
- `0` — success (operation completed; for `--dry-run`, preview produced)
- `1` — operational error (DB unreachable, embedder failure, etc.)
- `2` — invalid arguments (clap-level)
- `3` — operation refused (e.g., `rename` confirmation declined without `-y`)

**Stdout**:
- `dream` — structured-JSON event lines (stable format — operator scripts may parse); summary at end
- `import` — file count + ingest result per file (or preview if `--dry-run`)
- `list` — table format (default) or JSON array (`--format json`)
- `search` — table-format result list with score, title, snippet
- `rename` — confirmation prompt + summary; or list if `--list`
- `stats` — key/value metrics dump
- `audit-stats` — table or JSON of per-query / per-fact audit aggregates
- `synthesis-audit` — formatted synthesis row + source memory references
- `reembed-synthesis` — per-row backfill log + summary count

**Stderr**:
- All `tracing` log output (never to stdout — stdout is the MCP transport in `mengdie-mcp`; convention kept consistent in `mengdie` CLI)
- Operator-facing warnings + errors

## Errors

| Subcommand | Error condition | Behavior |
|---|---|---|
| All | `--db-path` doesn't exist (and not creatable) | Exit 1, stderr: "DB path inaccessible: <path>" |
| `dream` | `--dry-run` without `--synthesize` | Exit 2, clap-level rejection |
| `dream` | `--synthesize --dry-run` AND `--decay-dry-run` | Exit 2, "incompatible flags" |
| `dream --synthesize` | LLM provider unconfigured | Exit 1, "no LLM provider configured; check ~/.mengdie/config.toml [llm] section" |
| `dream --synthesize` | LLM call fails mid-cluster | Soft-fail per cluster (continue); exit 0 if any cluster succeeded; exit 1 if all failed |
| `import --dir` | Directory doesn't exist | Exit 1, "directory not found: <dir>" |
| `import --dir` | No ingestable files found | Exit 0 (not an error), stderr warns "no ingestable files in <dir>" |
| `list --format json` | Invalid format | Exit 2, "format must be 'table' or 'json'" |
| `search` | Embedder unavailable | Soft-fail to FTS-only with stderr warning; exit 0 (parallels MCP soft-fail) |
| `rename` (no flags) | Missing `from` and `to` | Exit 2, prints help |
| `rename` | Conflict (target project_id already exists) | Exit 1 unless `--yes` (which forces merge) |
| `synthesis-audit <id>` | ID doesn't exist | Exit 1, "synthesis ID not found: <id>" |
| `synthesis-audit <id>` | ID is not a synthesis row | Exit 1, "ID is not a synthesis row (source_type != synthesis)" |
| `reembed-synthesis` | Embedder load fails (non-dry-run) | Exit 1, "embedder unavailable: <details>" |

## Examples

### Example 1 — Dream pass with synthesis (real run)

```bash
$ mengdie dream --synthesize --project mengdie
[INFO  mengdie::dreaming] Promoted 14 memories (recall ≥ 3, avg_relevance ≥ 0.5)
[INFO  mengdie::clustering] Clustered 14 memories into 4 clusters (threshold 0.75, min_size 3)
[INFO  mengdie::synthesis] Generated 4 syntheses; wrote 4 rows
{"event":"dream_complete","promoted":14,"clusters":4,"syntheses":4,"took_ms":24123}
```

### Example 2 — Decay dry-run (operator pre-mutation review)

```bash
$ mengdie dream --decay-dry-run
{"event":"decay_dry_run","total_longterm":47,"would_demote":3,"breached_ids":["01h8...","01h7...","01h6..."]}

Demote candidates (effective_relevance < 0.20):
  01h8... [spike outcome]      last_recalled 87 days ago
  01h7... [v0.7.0 retrospect]  last_recalled 95 days ago
  01h6... [closure note]       last_recalled 122 days ago

Run with --decay-dry-run removed to apply (see docs/operations/dreaming-decay.md for the full pre-mutation procedure).
```

### Example 3 — Batch import preview

```bash
$ mengdie import --dir docs/decisions/ --dry-run
Would ingest:
  docs/decisions/021-auth-middleware.md  (source_type: conclusion)
  docs/decisions/022-session-storage.md  (source_type: conclusion)

2 files would be ingested. Dry-run; no writes.
```

### Example 4 — Search

```bash
$ mengdie search "tech stack" --limit 3
score    title                                       valid_from
0.94     Rust tech-stack rationale                   2026-04-04
0.81     OSS library survey verdicts                 2026-05-04
0.73     Storage layer choice                        2026-04-27
```

### Example 5 — Reembed synthesis backfill

```bash
$ mengdie reembed-synthesis --dry-run
Would re-embed 5 synthesis rows: 01h8_a..., 01h8_b..., 01h8_c..., 01h8_d..., 01h8_e...
(Skipping embedder load; pass without --dry-run to apply.)
```

## Notes

**Configuration**:
- DB location: `~/.mengdie/db.sqlite` (override via `--db-path`)
- Config file: `~/.mengdie/config.toml` (used by `dream --synthesize` for LLM provider configuration; ignored by other subcommands)
- LLM provider: `claude` CLI subprocess; inherits the user's Claude credentials. No external telemetry.

**Dreaming pipeline phases**:
1. **Promotion** — sweep memories where `recall_count >= min_recall` AND `avg_relevance >= min_relevance` AND `last_recalled within window_days`; set `is_longterm = true`
2. **Decay** — compute `effective_relevance = avg_relevance × 2^(-d/60)` at promotion + search-rerank time; demote `effective < 0.20`
3. **Clustering** — cosine seed-neighborhood clustering on long-term memories
4. **Synthesis** — per cluster, LLM generates a synthesis memory (`source_type: synthesis`) summarizing the cluster

`mengdie dream` (no flags) runs phases 1+2 only.
`mengdie dream --synthesize` runs all 4 phases.

**Project scoping**:
- All commands except `--global` flag operate on the current project (inferred from cwd via git remote at startup).
- `rename` operates globally (project_id is the renamed key).

**Operator-experience invariants**:
- `--dry-run` flags are NEVER destructive (read-only by contract).
- `--decay-dry-run` is the documented pre-mutation review path — see [`docs/operations/dreaming-decay.md`](operations/dreaming-decay.md).
- Structured-JSON event lines on stdout for `dream` are stable; operator scripts may parse them.

**Stability**:
- Subcommand list is stable (no removals planned for current major).
- Flag additions are forward-compatible.
- Output formats: `--format json` for `list` is stable; `dream` JSON event schema is stable; other text formats are operator-friendly but not parser-stable.
