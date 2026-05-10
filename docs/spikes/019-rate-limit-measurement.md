# Spike 019 — synthesis rate-limit measurement

**Plan**: `docs/plans/019-synthesis-cli-json-schema.md` Step 4 / AC4
**Status**: ✅ done — production run 2026-05-10 16:43-16:46 UTC

## Backup path

Production DB backed up before any structured-output Dreaming run:

```
~/.mengdie/db.sqlite.bak-pre-019-1778428737
```

Restore (if a future run regrets):

```bash
cp ~/.mengdie/db.sqlite.bak-pre-019-1778428737 ~/.mengdie/db.sqlite
```

## Schema-shape post-mortem (2026-05-10)

The plan's original `oneOf` design was rejected by Anthropic API at
runtime:

```
API Error: 400 tools.1.custom.input_schema:
input_schema does not support oneOf, allOf, or anyOf at the top level
```

claude-CLI translates `--json-schema` into a tool-call's `input_schema`,
and Anthropic's tool-API enforces a single-shape JSON object subset
(top-level `type: "object"` + `properties` + `required`). JSON Schema
combinators are not accepted at the top level.

**Resolution**: switched to flat schema with `skip:bool` discriminator
required, all other fields optional. The structural "exactly one of two
shapes" guarantee is lost, but `parse_synthesis_response`'s existing
runtime validation (`MissingField`, `EmptyTitle`, `EmptyContent`) covers
the semantic layer; the prompt still discourages lazy skips. Net effect:
schema validation is shape-checking + maxLength/minLength + typed-fields,
not mutually-exclusive-shape.

The `$schema` and `$comment` JSON-Schema-draft-07 metadata fields
documented in `docs/milestones/019/notes.md` Step-1 deferral were also
dropped — Anthropic's input_schema subset doesn't formally accept them
either, and the BL-027 verification schema didn't carry them.

Both changes shipped in the Step 4 commit; pre-fix probes are recorded
in `/tmp/claude-probe-stdout.json` (oneOf rejected) and
`/tmp/claude-probe2-stdout.json` (top-level type still oneOf, also
rejected). Post-fix probe `/tmp/claude-probe3-stdout.json` succeeded.

## Instrumentation contract

`src/core/llm.rs::extract_structured_output` emits one `tracing::info!`
event per `complete_structured` call carrying:

- `duration_ms` — claude-CLI's reported subprocess duration
- `total_cost_usd` — wrapper's reported API-equivalent cost
- `usage` — full usage sub-object captured as `serde_json::Value`

Hand-crafted test fixtures (no `usage` / `total_cost_usd` /
`duration_ms` fields) skip this log line silently — the `if` guard
prevents an all-`None` entry.

## Production run results (2026-05-10 16:43-16:46 UTC)

Run command: `RUST_LOG=mengdie=info ./target/release/mengdie dream --synthesize`

| Metric | Value |
|---|---|
| Total clusters processed | 5 |
| Syntheses written | **5** (100% success) |
| Syntheses LLM-skipped | 0 (0/3 pair-clusters skipped) |
| **parse_errors** | **0** ✓ |
| LLM-call errors | 0 |
| Memories truncated (>4000 chars) | 10 |
| Total elapsed (5 LLM calls) | ~160 sec |
| Per-cluster duration range | 24-39 sec |
| Per-cluster cost range (USD-equivalent) | $0.068 – $0.089 |
| Total cost (USD-equivalent) | ~$0.40 |
| Total tokens (cache_creation + cache_read + output) | ~275K |

**Note on cost**: `total_cost_usd` is what Anthropic API would have
charged; under Claude Code Pro flat-fee subscription, **actual additional
spend is $0**. Subscription rate-limit token budget IS consumed.

## Verdict

**Comfortable.** One full Dreaming pass on the operator's personal KB
(~5 clusters, 13K avg cluster context) consumed ~275K subscription
tokens in ~3 minutes wall-clock. Far under typical daily session
budget. **No follow-up rate-limit-relief BL filed.**

Caveats for future scale:
- Cluster count grows linearly with KB size; if production scales 10x
  the current 5-cluster shape, one Dreaming pass would be ~30 min and
  ~2.7M tokens. That hits Pro daily quota territory.
- Cache hit ratio looks favorable (`cache_read_input_tokens` >
  `cache_creation_input_tokens` after first call) — sustained dreaming
  invocations would benefit. If cache ratio degrades, re-measure.

## Manual quality spot-check (2026-05-10)

All 5 syntheses written to `~/.mengdie/db.sqlite` inspected via SQL:

| Synthesis | Title (truncated) | Content len | Entities |
|---|---|---|---|
| `b640934b` | F-015 AE output standards: 4 rules, 8-doc pyramid... | 1102 | f-015,output-standards,8-doc-pyramid,... |
| `f80e7680` | Consecutive Regret Analyses (016–017): Single Frag... | 1548 | plan-regret-analysis,dreaming-internals,... |
| `4ddb3ab9` | No silent drops: backlog entry + trigger required... | 1212 | backlog-hygiene,plan-closure-policy,... |
| `801b4540` | Mengdie multi-agent discussion conclusions — scope... | 1095 | mengdie-sprint-planning,ae-discussion-co,... |
| `17533124` | mengdie v0.0.1 architecture decisions — audit, sea... | 1368 | mengdie-v0.0.1,f-002-audit-table,search-... |

**Quality verdict**: ✓ All rows have substantial content (1095-1548
chars), descriptive titles within the 80-char schema cap, multi-tag
entity strings. No empty entities. No lazy "these memories share..."
fallthroughs. The flat-schema + anti-lazy SYSTEM_PROMPT combination
preserves output quality despite losing the `oneOf` structural
guarantee.

No restoration from backup needed.
