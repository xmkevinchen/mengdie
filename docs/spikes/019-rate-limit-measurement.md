# Spike 019 — synthesis rate-limit measurement

**Plan**: `docs/plans/019-synthesis-cli-json-schema.md` Step 4 / AC4
**Status**: production-run pending — backup made, instrumentation in place

## Backup path

Production DB backed up before any structured-output Dreaming run.
Restore from this file if the production run produces low-quality
synthesis rows (lazy-skip fallthroughs etc.):

```
~/.mengdie/db.sqlite.bak-pre-019-1778428737
```

To restore:

```bash
cp ~/.mengdie/db.sqlite.bak-pre-019-1778428737 ~/.mengdie/db.sqlite
```

## Instrumentation contract

`src/core/llm.rs::extract_structured_output` emits one `tracing::info!`
event per `complete_structured` call carrying:

- `duration_ms` — claude-CLI's reported subprocess duration
- `total_cost_usd` — wrapper's reported API-equivalent cost (note: under
  Claude Code Pro subscription, **actual additional spend is $0**;
  rate-limit token budget IS consumed)
- `usage` — full usage sub-object (cache_creation / cache_read / output
  tokens), captured as `serde_json::Value` for flexibility across CLI
  versions

Hand-crafted test fixtures (no `usage` / `total_cost_usd` / `duration_ms`
fields) skip this log line silently — the `if` guard prevents an all-
`None` entry from cluttering logs without information.

## Production run results

**Status**: pending operator trigger.

Once `mengdie dream --synthesize` runs against `~/.mengdie/db.sqlite`,
fill in the table below from the `tracing::info!` log lines (default
log destination: stderr, per project convention `tracing → stderr`).

| Metric | Value |
|---|---|
| Total clusters processed | `<TBD>` |
| Syntheses written | `<TBD>` |
| Syntheses skipped (LLM refused) | `<TBD>` |
| **parse_errors (target: 0)** | `<TBD>` |
| Total tokens (cache_creation + cache_read + output) | `<TBD>` |
| Total elapsed (wall-clock) | `<TBD>` |
| Per-cluster average duration | `<TBD>` |
| Per-cluster average cost (USD-equivalent) | `<TBD>` |

## Verdict

**TBD** — once the production run completes, write a one-sentence
verdict on whether subscription-budget rate-limit relief is needed:

- If one full Dreaming pass consumes > 50% of the operator's typical
  daily session budget → file `docs/backlog/unscheduled/BL-NNN-
  synthesis-rate-limit-relief.md` capturing "switch to `--bare` +
  `ANTHROPIC_API_KEY`" as a separate decision.
- If under 50% → no follow-up BL needed; capture as acceptable
  operating cost.

## Manual quality spot-check

After the production run, read the 3 most recent synthesis rows from
`~/.mengdie/db.sqlite` (e.g., via `mengdie audit --recent 3` or direct
SQL). Confirm none of:

- Empty `entities`
- Title > 80 chars
- `content` reading like a lazy skip-shape fallthrough (e.g., generic
  "these memories share..." with no specific decision content)

If any row smells lazy → restore from the backup file (above) before
any further Dreaming runs; file a follow-up BL on schema/prompt tuning.
