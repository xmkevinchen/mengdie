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

## What

Two changes to the AE plugin (`agentic-engineering/plugins/ae/`):

### Change 1: `ae:analyze` SKILL.md — Post-Research Injection

**File**: `plugins/ae/skills/analyze/SKILL.md`

**Where**: After the research phase (agents have gathered findings), before
synthesis begins. Add a new step between existing steps.

**What to add** (approximately 3-5 lines in the SKILL.md):

```markdown
### Step 3.5: Prior Context (from Mengdie)

Before synthesizing, search Mengdie for prior decisions on this topic:
- Call `memory_search` MCP tool with the research topic as query
- If results found, present as "Round 0: Prior Decisions" block with provenance:
  - Source file, knowledge type, entities, when it was decided
- Agents synthesize with explicit awareness of prior decisions
- Note whether current evidence confirms, updates, or contradicts prior decisions
```

**Behavior**:

- If the Mengdie MCP server is not registered → skip silently (no error).
  Implementation: wrap the `memory_search` call in try/catch; if the tool
  is not found (MCP error -32601 "method not found" or tool not listed),
  continue without injecting prior context.
- If `memory_search` returns empty → skip (no prior context).
- If results found → display with provenance; agents explicitly address
  alignment/contradiction in their synthesis.

### Change 2: AE Conclusion Template — `entities` Frontmatter Field

**File**: the `conclusion.md` template used by `ae:discuss` (in
`plugins/ae/skills/discuss/SKILL.md` or a referenced template file).

**What to add**: one field to the conclusion frontmatter:

```yaml
---
id: "NNN"
title: "..."
concluded: YYYY-MM-DD
plan: ""
entities: []    # ← NEW: extracted entity tags for Mengdie ingestion
---
```

**Why**: Mengdie's ingestion pipeline extracts entities from the `entities`
field in frontmatter. Without it, ingested conclusions have empty entity
lists, which degrades contradiction detection (no entity overlap → no
conflicts detected, no `memory_entity_facts` hits).

**Who populates it**: the `ae:discuss` skill's conclusion-generation step.
The team lead extracts key entities from the Decision Summary table's
Topic column and includes them in the frontmatter (e.g., if Decision
Summary has topics "Auth middleware", "Session storage" →
`entities: [auth, middleware, session, storage]`).

## How (for the AE plugin maintainer)

### Change 1 implementation

1. Open `plugins/ae/skills/analyze/SKILL.md`.
2. Find the step where research is complete and synthesis begins.
3. Insert Step 3.5 between them (exact text above).
4. The `memory_search` tool is an MCP tool — it's available to any agent
   whose host AI tool session has the Mengdie MCP server registered. No
   AE code change is needed beyond the SKILL.md instruction.

### Change 2 implementation

1. Find the `conclusion.md` template/instructions in `ae:discuss` SKILL.md.
2. Add `entities: []` to the frontmatter template.
3. Add the instruction for the team lead to populate entities from
   Decision Summary topics.

### Verification

- After Change 1: run `/ae:analyze` on a topic where Mengdie has prior
  decisions. Verify "Round 0: Prior Decisions" appears in the output with
  provenance.
- After Change 2: run `/ae:discuss` to conclusion. Verify the
  `conclusion.md` has a populated `entities: [...]` in its frontmatter.

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
