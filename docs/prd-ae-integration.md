# PRD: AE Plugin Integration for Second Brain

## Why

The Second Brain knowledge loop has a gap: memories are ingested and searchable, but never fed back into the AI agent workflow. Without feedback injection, the loop is open — agents research from scratch every time, unaware of prior decisions.

The ae:analyze skill is the injection point. After agents finish independent research but before synthesis, Second Brain injects relevant prior decisions as "Round 0: Prior Decisions." This closes the spiral: agents produce knowledge → Second Brain ingests → feeds context back → agents make better-informed decisions → richer knowledge.

Without this integration, Second Brain is a passive archive. With it, it's an active participant in every analysis.

## What

Two changes to the AE plugin (`agentic-engineering/plugins/ae/`):

### Change 1: ae:analyze SKILL.md — Post-Research Injection

**File**: `plugins/ae/skills/analyze/SKILL.md`

**Where**: After the research phase (agents have gathered findings), before synthesis begins. Add a new step between existing steps.

**What to add** (approximately 3-5 lines in the SKILL.md):

```markdown
### Step 3.5: Prior Context (from Second Brain)

Before synthesizing, search Second Brain for prior decisions on this topic:
- Call `memory_search` MCP tool with the research topic as query
- If results found, present as "Round 0: Prior Decisions" block with provenance:
  - Source file, knowledge type, entities, when it was decided
- Agents synthesize with explicit awareness of prior decisions
- Note whether current evidence confirms, updates, or contradicts prior decisions
```

**Why this location**: Post-research avoids anchoring bias. Agents form independent positions first, then see what was decided before. This was a key design decision from Discussion 002 (Topic: ae:analyze feedback timing).

**Behavior**:
- If Second Brain MCP server is not registered → skip silently (no error). Implementation: wrap `memory_search` call in try/catch; if tool is not found (MCP error -32601 "method not found" or tool not listed), continue without injecting prior context.
- If memory_search returns empty → skip (no prior context)
- If results found → display with provenance, agents explicitly address alignment/contradiction

### Change 2: AE Conclusion Template — `entities` Frontmatter Field

**File**: The conclusion.md template used by `ae:discuss` (likely in `plugins/ae/skills/discuss/SKILL.md` or a template file).

**What to add**: One field to the conclusion frontmatter:

```yaml
---
id: "NNN"
title: "..."
concluded: YYYY-MM-DD
plan: ""
entities: []    # ← NEW: extracted entity tags for Second Brain ingestion
---
```

**Why**: Second Brain's ingestion pipeline extracts entities from the `tags` field in frontmatter. AE conclusions currently don't have a `tags` or `entities` field. Without it, ingested conclusions have empty entity lists, which degrades contradiction detection (no entity overlap → no conflicts detected).

**Who populates it**: The ae:discuss skill's conclusion generation step. TL extracts key entities from the Decision Summary table's Topic column and includes them in the frontmatter.

## How (for AE plugin maintainer)

### Change 1 Implementation

1. Open `plugins/ae/skills/analyze/SKILL.md`
2. Find the step where research is complete and synthesis begins
3. Insert Step 3.5 between them (exact text above)
4. The `memory_search` tool is an MCP tool — it's available to any agent whose Claude Code session has the Second Brain MCP server registered. No AE code change needed beyond the SKILL.md instruction.

### Change 2 Implementation

1. Find the conclusion.md template/instructions in ae:discuss SKILL.md
2. Add `entities: []` to the frontmatter template
3. Add instruction for TL to populate entities from Decision Summary topics
4. Example: if Decision Summary has topics "Auth middleware", "Session storage" → `entities: [auth, middleware, session, storage]`

### Verification

- After Change 1: Run `ae:analyze` on a topic where Second Brain has prior decisions. Verify "Round 0: Prior Decisions" appears in output.
- After Change 2: Run `ae:discuss` to conclusion. Verify conclusion.md has populated `entities: [...]` in frontmatter.

## Dependencies

- Second Brain MCP server must be built and registered in `~/.claude/settings.json`
- fastembed model must be downloaded (happens automatically on first run)
- At least one memory must be ingested for `memory_search` to return results

## What Second Brain Provides (Integration Interface)

AE agents interact with Second Brain exclusively through 3 MCP tools. No code dependency, no shared library, no import — just MCP tool calls.

### Registration

Add to `~/.claude/settings.json`:
```json
{
  "mcpServers": {
    "second-brain": {
      "command": "/path/to/second-brain-mcp",
      "args": []
    }
  }
}
```

All AE subagents automatically inherit MCP tools from the parent Claude Code session. Zero per-agent config needed.

### MCP Tools Available to AE

**`memory_search`** — Search prior decisions and knowledge
```json
Input:  { "query": "auth middleware compliance", "scope": "global" }
Output: {
  "results": [
    {
      "id": "uuid",
      "title": "Auth Middleware Decision",
      "source_file": "docs/discussions/003/conclusion.md",
      "knowledge_type": "decisional",
      "entities": "auth,middleware,compliance",
      "score": 0.032,
      "valid_from": "2026-04-04T...",
      "snippet": "Use JWT tokens with..."
    }
  ],
  "degraded": null
}
```

**`memory_ingest`** — Store a new memory (ae:discuss conclusion, review, etc.)
```json
Input:  {
  "title": "Tech Stack Decision",
  "content": "Full conclusion text...",
  "source_file": "docs/discussions/003/conclusion.md",
  "source_type": "conclusion",
  "knowledge_type": "decisional",
  "entities": "rust,mcp,sqlite"
}
Output: {
  "entry_id": "uuid",
  "conflicts": [
    { "id": "old-uuid", "title": "Old Tech Stack", "reason": "evolution candidate (similarity: 0.85)" }
  ],
  "error": null
}
```

**`memory_invalidate`** — Mark a memory as superseded
```json
Input:  { "entry_id": "old-uuid", "reason": "Superseded by new decision", "superseded_by": "new-uuid" }
Output: { "success": true, "entry_id": "old-uuid" }
```

### What AE Does NOT Need to Know

- How embeddings are generated (fastembed, ONNX — internal)
- How search works (FTS5 + vector + RRF — internal)
- Where the database lives (~/.second-brain/db.sqlite — internal)
- How Dreaming promotes memories (internal cron)
- How contradiction detection works (internal, results surfaced in `conflicts` array)

### Delivery Timeline

Second Brain MCP server binary will be provided as:
1. **Binary**: `cargo build --release` → single binary `second-brain-mcp`
2. **Registration**: User adds one line to `settings.json`
3. **First run**: Model downloads (~90MB, cached after)
4. **Batch import**: `second-brain import --dir docs/discussions/` to seed existing AE knowledge

## Not In Scope

- ae:discuss per-role differentiated injection (Phase 2 — requires per-agent-role context)
- ae:plan injection (Phase 2 — gets signal through ae:discuss output)
- Automatic entity extraction from non-frontmatter content (Phase 2)

## Reference

- Discussion 002 MVP Phase 1: `docs/discussions/002-mvp-phase1/conclusion.md` — Section "Feedback: ae:analyze SKILL.md Modification"
- Discussion 003 Tech Stack: `docs/discussions/003-tech-stack/conclusion.md`
- CLAUDE.md: Architecture → Feedback section
