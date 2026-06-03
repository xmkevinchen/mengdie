---
title: "AE Plugin Integration"
type: integration-guide
last_updated: 2026-05-23
audience: [ae-plugin-maintainer, operator]
---

# AE Plugin Integration

How the AE plugin (`agentic-engineering`) integrates with Mengdie. AE produces
the structured-markdown artifacts Mengdie ingests, and AE skills query Mengdie
for prior context during analysis. The contract is the MCP-tool surface — no
shared code, no library dependency.

## Why

Mengdie's knowledge loop has a gap if nothing reads it back: memories are
ingested and searchable, but never re-injected into the agent workflow.
Without feedback injection, agents research from scratch every time and
re-discover decisions that were already made.

AE's `ae:analyze` skill is the injection point. After agents finish
independent research but before synthesis, Mengdie injects relevant prior
decisions as a "Round 0: Prior Decisions" block with explicit provenance.
This closes the spiral: agents produce knowledge → Mengdie ingests → feeds
context back → agents make better-informed decisions → richer knowledge.

The post-research injection point (not pre-research) is deliberate: it
avoids anchoring bias. Agents form independent positions first, then see
what was decided before, then synthesize with full awareness of both.

## Integration points

The AE plugin (`agentic-engineering/plugins/ae/`) integrates with Mengdie
in three places — one read path and two write paths. All three are
SKILL.md prose with no AE-side code dependency on Mengdie; the contract
is the MCP tool surface.

### Read path: prior-context lookup before synthesis

**Where**: `plugins/ae/skills/analyze/SKILL.md` → `### Prior context (Mengdie integration)`

After research agents have reported their findings to the team lead and
before the lead writes `analysis.md`, the lead calls `memory_search` with
the feature title as the query and renders results under a
`## Prior Art from Project Knowledge Base` heading with provenance
(`title`, `source_file`, `knowledge_type`, `valid_from`, `snippet`).

Failure modes are tolerated silently:

- Tool unavailable, zero results, or a thrown error → emit
  `Prior context: unavailable (tool not registered / no relevant results)`
  and proceed without prior context.
- Results with a non-null `degraded` field (e.g., embedder unavailable
  → FTS-only fallback) → annotate the block as `(partial — [reason])`.

Prior context is **background only** — it does not constrain the current
evidence the agents gathered. Post-research placement (not pre-research)
avoids anchoring bias: agents form independent positions first, then see
what was decided before, then synthesize.

### Write path 1: knowledge capture after analysis

**Where**: `plugins/ae/skills/analyze/SKILL.md` → `### Knowledge capture (Mengdie)`

After `analysis.md` is written and before the team closes, the lead
ingests up to 3 atomic facts via `memory_ingest`. Each fact is sourced
from one key finding in the analysis's `## Findings` section, with
`source_type: conclusion`, `knowledge_type: factual`, and `entities`
derived from the specific finding (e.g., `fts5-idf-contamination`) —
not from broad frontmatter tags. Findings that restate prior context
surfaced in the read path are skipped to avoid double-counting.

### Write path 2: knowledge capture after each shipped step

**Where**: `plugins/ae/skills/work/SKILL.md` → `### Knowledge capture (Mengdie)`

After `/ae:work` commits a step, the same shared Knowledge Capture
Protocol fires — up to 3 atomic items, with `conflicts` returned by
`memory_ingest` resolved via `memory_invalidate` when an evolution
candidate is detected.

### Entities populated at conclusion time

**Where**: `plugins/ae/skills/discuss/SKILL.md` (conclusion-generation step)

When `/ae:discuss` writes `conclusion.md`, the lead extracts entity tags
from the Decision Summary table's Topic column and writes them to the
frontmatter's `entities` field, derived from each specific decision
rather than from the broad conclusion frontmatter. This is what makes
the ingested conclusion show up in `memory_entity_facts` lookups and
participate in contradiction detection (entity-overlap is the directed
comparison signal).

## Setup

### Dependencies

- The Mengdie MCP server must be built (`cargo build --release` produces
  `mengdie-mcp`) and registered in the host AI tool's MCP config.
- The embedding model is downloaded on first run (~90 MB, cached at
  `~/.cache/fastembed/`).
- At least one memory must be ingested for `memory_search` to return
  results.

### Build Mengdie

```bash
cd /path/to/mengdie
cargo build --release
# Binary: target/release/mengdie-mcp
```

### Register the MCP server

For Claude Code, add to `~/.claude/settings.json` under `mcpServers`:

```json
"mengdie": {
  "command": "/path/to/mengdie/target/release/mengdie-mcp",
  "args": []
}
```

Other MCP hosts follow their own registration format; the binary speaks
stdio MCP.

### Seed test data

```bash
# Import existing structured artifacts into Mengdie
mengdie import --dir docs/discussions/
```

### Verify

```bash
# Search for a known decision
mengdie search "tech stack"
```

Should return the matching ingested conclusion.

### Test the integration

After modifying `ae:analyze` SKILL.md:

1. Restart the host AI tool (to pick up the MCP server registration).
2. Run `/ae:analyze` on any topic where Mengdie has prior decisions.
3. Verify "Round 0: Prior Decisions" appears in the output with
   provenance.

## Integration Interface

AE agents interact with Mengdie exclusively through MCP tools. No code
dependency, no shared library, no import — just MCP tool calls.

All sub-agents spawned by the host AI tool inherit MCP tools from the
parent session. Zero per-agent config needed.

### MCP tools available to AE

See [`docs/specs/`](specs/) for the full per-tool specifications. Summary:

- **`memory_search`** — search prior decisions and knowledge (hybrid
  FTS5 + vector, with provenance).
- **`memory_ingest`** — store a new memory (`ae:discuss` conclusion,
  review, etc.); returns detected conflicts so the caller can resolve
  evolution candidates explicitly.
- **`memory_get`** — fetch the full content of a single memory (used
  when the snippet from `memory_search` is insufficient).
- **`memory_invalidate`** — mark a memory as superseded (used to resolve
  conflicts returned by `memory_ingest`).
- **`memory_status`** — DB health snapshot (entry counts, last ingest,
  audit pipeline state).
- **`memory_lint`** — deterministic health checks (orphan GC,
  contradictions, embedding drift). Read-only.
- **`memory_entity_facts`** — list all facts tagged with a given
  entity name.

### What AE does NOT need to know

- How embeddings are generated (`fastembed`, ONNX Runtime — internal).
- How search works (FTS5 + vector + RRF — internal).
- Where the database lives (`~/.mengdie/db.sqlite` — internal).
- How Dreaming promotes / synthesizes memories (internal pass).
- How contradiction detection works (internal; results surfaced in the
  `conflicts` array returned by `memory_ingest`).

## Not in scope (today)

- Per-role differentiated injection inside `ae:discuss` (e.g., feeding
  different prior context to different team-member angles).
- `ae:plan` injection (the plan step inherits signal through the
  `ae:discuss` conclusion).
- Automatic entity extraction from non-frontmatter content.
