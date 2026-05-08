---
title: "spec: mengdie CLI"
type: spec
created: 2026-05-08
as_of_commit: 1b48c92
stability: draft-until-v0.0.1
implementation_status: current
audience: [operator]                # CLI exempted from llm-at-mcp-tool-discovery audience (no MCP description field)
---

# spec: `mengdie` CLI

The `mengdie` binary is the operator-facing CLI for direct database management,
batch import, and the Dreaming pipeline. The MCP server (`mengdie-mcp` separate
binary) is the agent-facing entry point and is specified separately
(`memory_search` / `memory_ingest` / `memory_invalidate` specs).

## Signature

**Binary**: `mengdie` (built from `src/bin/cli.rs`)

**Global options** (apply to all subcommands):

| Flag | Type | Default | Constraint |
|---|---|---|---|
| `--db-path <PATH>` | path | `~/.mengdie/db.sqlite` | overrides default DB location |

**Subcommands** (7):

```
mengdie dream             [--synthesize] [--decay-dry-run] [...flags]
mengdie import            --dir <PATH> [--dry-run]
mengdie list              [--global] [--format table|json]
mengdie search <query>    [--global] [--limit N] [--min-score F]
mengdie rename            [from] [to] [--list] [--dry-run] [--yes]
mengdie stats
mengdie synthesis-audit <id>
```

## Params (per subcommand)

### `dream`

Run the Dreaming pipeline (promotion + optional decay + optional LLM synthesis).

| Flag | Type | Default | Semantics |
|---|---|---|---|
| `--min-recall <N>` | i64 | `dreaming::DEFAULT_MIN_RECALL` (currently 3) | Minimum `recall_count` for promotion |
| `--min-relevance <F>` | f64 | `dreaming::DEFAULT_MIN_RELEVANCE` (currently 0.5) | Minimum `avg_relevance` for promotion |
| `--window-days <N>` | i64 | `dreaming::DEFAULT_WINDOW_DAYS` (currently 30) | `last_recalled` recency window |
| `--synthesize` | flag | off | Run LLM synthesis after promotion (opt-in: makes network calls + writes synthesis rows) |
| `--threshold <F>` | f32 | `clustering::DEFAULT_THRESHOLD` (0.75) | Cosine clustering threshold |
| `--min-cluster-size <N>` | usize | `clustering::DEFAULT_MIN_SIZE` (3) | Minimum cluster size for synthesis |
| `--max-cluster-size <N>` | usize | `20` | Cluster truncation cap (bounds LLM token budget) |
| `--dry-run` | flag | off | Show synthesis prompts without LLM calls or DB writes. **Requires `--synthesize`** (per review feedback: previously `--dry-run` silently triggered synthesis path even without `--synthesize`) |
| `--decay-dry-run` | flag | off | Dry-run BL-008 exponential decay pass: report would-demote list without clearing `is_longterm`. Incompatible with `--synthesize --dry-run` |
| `--project <ID>` | string | (all projects) | Project scope for synthesis |

### `import`

Batch-import AE discussion files (`conclusion.md` / `review.md` / `plan.md` / `retrospect.md`) from a directory.

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

### `synthesis-audit <id>`

Read-only audit of a synthesis row + its source memories. Scaffolding for future Options 2/3 ship-gate data collection (plan 017, discussion 022) — operator eyeballs fidelity.

| Flag | Type | Required | Semantics |
|---|---|---|---|
| `<id>` | positional | yes | Synthesis memory ID to inspect |

## Returns (exit codes + stdout)

**Exit codes** (POSIX convention):
- `0` — success (operation completed; for `--dry-run`, preview produced)
- `1` — operational error (DB unreachable, embedder failure, etc.)
- `2` — invalid arguments (clap-level)
- `3` — operation refused (e.g., `rename` confirmation declined without `-y`)

**Stdout**:
- `dream` — structured-JSON event lines (per BL-008 `--decay-dry-run` ops doc); summary at end
- `import` — file count + ingest result per file (or preview if `--dry-run`)
- `list` — table format (default) or JSON array (`--format json`)
- `search` — table-format result list with score, title, snippet
- `rename` — confirmation prompt + summary; or list if `--list`
- `stats` — key/value metrics dump
- `synthesis-audit` — formatted synthesis row + source memory references

**Stderr** (per project convention):
- All `tracing` log output (per CLAUDE.md: never to stdout — would corrupt MCP transport in mcp_server, kept consistent here)
- Operator-facing warnings + errors

## Errors

| Subcommand | Error condition | Behavior |
|---|---|---|
| All | `--db-path` doesn't exist (and not creatable) | Exit 1, stderr: "DB path inaccessible: <path>" |
| `dream` | `--dry-run` without `--synthesize` | Exit 2, clap-level rejection per review feedback |
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

## Examples

### Example 1 — Dream pass with synthesis (real run)

```bash
$ mengdie dream --synthesize --project mengdie
[2026-05-08T13:45:21Z INFO  mengdie::dreaming] Promoted 14 memories (recall ≥ 3, avg_relevance ≥ 0.5)
[2026-05-08T13:45:23Z INFO  mengdie::clustering] Clustered 14 memories into 4 clusters (threshold 0.75, min_size 3)
[2026-05-08T13:45:45Z INFO  mengdie::synthesis] Generated 4 syntheses; wrote 4 rows
{"event":"dream_complete","promoted":14,"clusters":4,"syntheses":4,"took_ms":24123}
```

### Example 2 — Decay dry-run (operator pre-mutation review)

```bash
$ mengdie dream --decay-dry-run
{"event":"decay_dry_run","total_longterm":47,"would_demote":3,"breached_ids":["01h8...","01h7...","01h6..."]}

Demote candidates (effective_relevance < 0.20):
  01h8... [F-001 spike outcome] last_recalled 87 days ago
  01h7... [v0.7.0 retrospect] last_recalled 95 days ago
  01h6... [BL-001 closure] last_recalled 122 days ago

Run with --decay-dry-run removed to apply (and use scripts/verify-decay.sh first per docs/operations/dreaming-decay.md).
```

### Example 3 — Batch import preview

```bash
$ mengdie import --dir docs/discussions/ --dry-run
Would ingest:
  docs/discussions/026-rust-oss-survey/conclusion.md  (source_type: conclusion)
  docs/discussions/027-industry-state-2026/conclusion.md  (source_type: conclusion)
  docs/discussions/028-doc-structure/conclusion.md  (source_type: conclusion)

3 files would be ingested. Dry-run; no writes.
```

### Example 4 — Search

```bash
$ mengdie search "v0.0.1 thesis" --limit 3
score    title                                       valid_from
0.94     v0.0.1 thesis: narrow OSS-adoption          2026-05-05
0.81     026 OSS-survey verdicts (final)             2026-05-04
0.73     blueprint.md v0.2 multi-version trajectory  2026-04-27
```

## Notes

**No MCP description field**: `mengdie` CLI is operator-facing only. The `audience` frontmatter excludes `llm-at-mcp-tool-discovery` (only `operator`); MCP tool description requirement (< 200 tokens) does NOT apply per F-004 plan AC3 carve-out.

**Configuration**:
- DB location: `~/.mengdie/db.sqlite` (override via `--db-path`)
- Config file: `~/.mengdie/config.toml` (used by `dream --synthesize` for LLM provider configuration; ignored by other subcommands)
- LLM provider: claude-CLI subprocess (BL-005); inherits user's Claude credentials. No external telemetry.

**Dreaming pipeline phases**:
1. **Promotion** — sweep memories where `recall_count >= min_recall` AND `avg_relevance >= min_relevance` AND `last_recalled within window_days`; set `is_longterm = true`
2. **Decay** (BL-008) — compute `effective_relevance = avg_relevance × 2^(-d/60)` at promotion + search-rerank time; demote `effective < 0.20`
3. **Clustering** (BL-006) — cosine seed-neighborhood clustering on long-term memories
4. **Synthesis** (BL-007) — per cluster, LLM generates a synthesis memory (source_type: synthesis) summarizing the cluster

`mengdie dream` (no flags) runs phases 1+2 only.
`mengdie dream --synthesize` runs all 4 phases.

**Project scoping**:
- All commands except `--global` flag operate on the current project (inferred from cwd via git remote at startup).
- `rename` operates globally (project_id is the renamed key).

**Operator-experience invariants**:
- `--dry-run` flags are NEVER destructive (read-only by contract)
- `--decay-dry-run` is the BL-008 ops-doc-mandated pre-mutation review path (`docs/operations/dreaming-decay.md`)
- All structured-JSON event lines on stdout for `dream` are stable (per BL-008 ops doc); operator scripts may parse them

**Stability**:
- Subcommand list is stable through v0.0.1 (no removals planned).
- Flag additions are forward-compatible.
- Output formats: `--format json` for `list` is stable; `dream` JSON event schema is stable per BL-008 ops doc; other text formats are operator-friendly but not parser-stable.
