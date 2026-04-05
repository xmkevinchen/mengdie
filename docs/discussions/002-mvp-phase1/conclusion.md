---
id: "002"
title: "AI-native Second Brain MVP Phase 1 — Conclusion"
concluded: 2026-04-05
plan: ""
---

# AI-native Second Brain MVP Phase 1 — Conclusion

4 agents (architect + archaeologist + challenger + codex-proxy), converged after 2 rounds of discussion.

---

## MVP Phase 1 Design

### Delivery: MCP Server (stdio)

Registered as a local stdio MCP server in Claude Code's `~/.claude/settings.json`. Claude Code spawns on demand. All AE subagents automatically inherit — zero extra config needed.

Code verification: `src/services/mcp/types.ts:28-35` supports stdio servers; `runAgent.ts:88-166` confirms subagents inherit parent MCP tools.

### 3 MCP Tools

| Tool | Function | Phase 1 Implementation |
|------|----------|----------------------|
| `memory_search(query, scope?)` | Search memories, return results + provenance | SQLite FTS5 + vector similarity hybrid search |
| `memory_ingest(entry)` | Ingest a memory entry | Parse YAML frontmatter + section extraction |
| `memory_invalidate(entry_id, reason)` | Mark memory as invalid | Set `valid_until` + `superseded_by` |

Phase 2 additions: `memory_cite(entry_id, context)` (citation signal, strengthens Dreaming score), `list_memories(filter)`.

### Ingestion: AE Output Watcher Only

Phase 1 ingests only AE pipeline output. No Claude Code session hook (deferred to Phase 2).

**Watched files**:
- `docs/discussions/**/conclusion.md` — decision memory (highest value)
- `docs/reviews/**/*.md` — lessons learned
- `docs/plans/**/*.md` — approach memory
- `docs/analyses/**/*retrospect*.md` — trend memory

**Not watched**: round-NN.md (intermediate process), analysis.md (superseded by conclusion), topic summaries (superseded by conclusion).

**Implementation**: Filesystem watcher (fsnotify / chokidar), detect file create/modify → parse frontmatter + body → call `memory_ingest`.

### Feedback: ae:analyze SKILL.md Modification (Post-Research Injection)

After ae:analyze's research phase completes, before synthesis begins, inject a Second Brain query:

```
### 3.5 Prior Context (from Second Brain)

Before synthesizing, search Second Brain for prior decisions on this topic:
- Call memory_search with the research topic
- If results found, present as "Round 0: Prior Decisions" with provenance
- Agents synthesize with explicit awareness of prior decisions
- Note whether current evidence confirms, updates, or contradicts prior decisions
```

This is ~3 lines of ae:analyze SKILL.md modification. Post-research position avoids anchoring bias (proposed by Architect, accepted by all).

**Why not use Claude Code SessionStart hook instead**: SessionStart fires at session level, not ae:analyze level. Wrong timing, and AE subagents don't see the main session's hook output.

**Phase 1 modifies ae:analyze only**. ae:discuss and ae:plan deferred to Phase 2 because:
- ae:discuss injection requires per-agent-role differentiation (archaeologist gets historical evidence, challenger gets past challenges) — more complex design
- ae:plan implicitly gets Second Brain signal through ae:discuss output

### Storage: Global SQLite, Per-Project Default Search

**Location**: `~/.second-brain/db.sqlite` (global, not inside any project)

**Memory entry schema**:
```
id: uuid
project_id: text       (git-inferred: remote URL or local path hash)
source_file: text      (original file path)
source_type: text      (conclusion | review | plan | retrospect)
knowledge_type: text   (decisional | experiential | factual)
title: text
content: text
entities: text[]       (extracted from frontmatter tags + Decision Summary)
valid_from: timestamp
valid_until: timestamp? (null = currently valid)
superseded_by: uuid?
recall_count: int
avg_relevance: float
last_recalled: timestamp
embedding: blob        (vector)
created_at: timestamp
```

**project_id inference**: git remote URL (`git remote get-url origin`); monorepos use `.ae/` relative path as sub-identifier. No AE frontmatter changes needed.

**Default search scope = current project**. `scope: "global"` parameter available but disabled by default in Phase 1. Dreaming runs globally (cross-project scoring improves signal quality).

### Simplified Dreaming (Phase 1)

Tracks only two signals: recall_count (times searched) + avg_relevance (average match score).

**Promotion rule**:
- `recall_count >= 3` AND `avg_relevance >= 0.65`, within 14-day window
- Qualifies → mark as long-term (boosted weight in search)
- Runs daily (cron or MCP server built-in timer)

**Deferred to Phase 2**:
- consolidation / conceptual signals (requires reading MEMORY.md and content analysis)
- cite() signal (requires AE to explicitly call memory_cite)
- Decay/archival (Phase 1 memories only grow; rely on valid_until for manual invalidation)

**Implementation**: ~150 lines, zero OpenClaw dependency.

### Contradiction Detection (MVP Minimum)

**Entity-tag directed comparison + Temporal Validity**:

Extract entities at ingestion time (from frontmatter tags + Decision Summary table's Topic column). New memories compared only against existing memories with overlapping entities.

**Processing logic**:
- Entity overlap + semantic similarity + `knowledge_type == decisional` → flag "evolution candidate": prompt "This new decision may supersede an older one. Mark the old one as invalid?"
- Entity overlap + semantic opposition + time gap <30 days → flag "conflict"
- User confirms → set old memory's `valid_until` + `superseded_by`

**Not doing**: Cross-terminology semantic conflict detection ("Use Redis" vs "Store in PostgreSQL") — requires embedding comparison, deferred to Phase 2 association discovery layer.

### Cold Start: Batch Import

Users with existing AE discussions: `second-brain import --dir .ae/discussions/` batch-imports all conclusion.md + review.md files.

**Import strategy**:
- Goes directly into long-term memory (skips short-term Dreaming wait) — AE output is already multi-agent-validated
- Marked `recall_count = 0` (not yet validated by actual reference)
- Enters normal Dreaming cycle after first ae:analyze reference

**No AI judgment for cold start** — avoids Challenger's Scenario C (cold-start AI judgment errors getting amplified).

### Observability (MVP Metrics)

| Metric | Collection Method | Success Threshold |
|--------|------------------|-------------------|
| `context_injection_rate` | % of memory_search returning non-empty | > 60% = loop is working |
| `stale_citation_rate` | % of cited memories with expired `valid_until` | < 10% = knowledge is being updated |
| `conflict_detection_rate` | % of new ingestions triggering conflict/evolution flags | 1-15% = detection sensitivity is normal |
| `memory_age_at_retrieval` | Average age of cited memories | Trending up with no updates = memory aging |

**Holdout experiment** (Phase 1 simplified): MCP server returns empty results for 20% of sessions based on session hash. Compare holdout vs injection sessions' ae:review Outcome Statistics. This is a late Phase 1 analysis, not Day 1 functionality.

**Natural quasi-experiment** (available Day 1): Compare runs with context injection vs runs without (Second Brain empty or search missed). No need to actively disable injection.

---

## Topic Decision Summary

### Topic 1: MVP Scope

| Component | Phase 1 | Phase 2 |
|-----------|---------|---------|
| MCP server (3 tools) | ✅ | Add cite + list |
| AE output watcher | ✅ (4 file types) | Add GitHub webhook, Slack bot |
| Claude Code session hook | ❌ | ✅ (SessionEnd + SessionStart) |
| Simplified Dreaming | ✅ (frequency + relevance) | Full 6-signal model + cite |
| ae:analyze feedback | ✅ (post-research Round 0) | Add ae:discuss per-role differentiated injection |
| Contradiction detection | ✅ (entity-tag + temporal validity) | Add cross-terminology semantic detection |
| Decay/archival | ❌ | ✅ |
| Association discovery | ❌ | ✅ |
| Team level | ❌ | Phase 3 |

### Topic 2: Cross-Project

**Decision**: Global storage, per-project default search.
- project_id inferred from git remote URL; no AE changes needed
- Default search scope = current project
- Dreaming runs globally
- Cross-project search reserved as optional parameter; not exposed in Phase 1

**Rationale**: Global storage implementation cost is only one extra `project_id` column and one search parameter above per-project. But per-project-first would require a later migration that breaks Dreaming recall history.

### Topic 3: AE Gaps

**Changes required**:
- ae:analyze SKILL.md: +3 lines (post-research memory_search + Round 0 display)
- AE conclusion.md template: +1 line frontmatter (`entities: [...]`)

**No changes needed**:
- AE pipeline.yml
- Other AE skill SKILL.md files
- AE agent definitions
- AE doesn't need to know Second Brain exists — MCP tool registered in Claude Code settings, AE subagents inherit automatically

### Topic 4: Negative Spiral Defense

**Phase 1 defenses**:
1. Entity-tag + temporal validity contradiction detection (at ingestion)
2. Feedback injection must be non-silent (Round 0 with provenance display)
3. Batch import doesn't use AI judgment (avoids cold-start error amplification)
4. 4 observability metrics + natural quasi-experiment

**Identified but deferred risks**:
- Challenger's Scenario B (workaround calcified as best practice): needs dependency version change trigger — Phase 2
- ae:discuss injection where all agents receive identical context reduces discussion quality — Phase 2 per-role differentiated injection

---

## Implementation Estimate

| Component | Effort | Dependencies |
|-----------|--------|-------------|
| MCP server framework (stdio) | 2-3 days | None |
| SQLite + hybrid search | 5-7 days | None (rewrite, no OpenClaw SDK) |
| AE file watcher | 2-3 days | MCP server |
| Simplified Dreaming | 2-3 days | SQLite |
| Contradiction detection | 3-4 days | SQLite + search |
| ae:analyze SKILL.md modification | 0.5 days | MCP server |
| Batch import CLI | 1-2 days | SQLite + watcher |
| Metrics dashboard | 1-2 days | SQLite |
| **Total** | **~3 weeks** | |

---

## Doodlestein Review

Not run (Investigation Mode; TL judged unnecessary — discussion produced specific implementation plans rather than major architectural decisions; Architect-Challenger adversarial dynamic already covered blind spots adequately).

## Team Composition

| Agent | Role | Backend | Focus |
|-------|------|---------|-------|
| TL | Moderator | Claude | Synthesis + tie-breaking |
| architect | Architecture design | Claude | MVP technical design |
| archaeologist | Code verification | Claude | Feasibility confirmation |
| challenger | Risk analysis | Claude | Negative spiral + scope challenge |
| codex-proxy | Cross-family | Codex | Competitor comparison + metrics |

## Process Metadata
- Discussion rounds: 2 (Architect did 3, others 2)
- Topics: 4 (4 converged)
- Autonomous decisions: 4
- User escalations: 0

## Next Steps
→ `/ae:plan` — create implementation plan (based on this conclusion's 3-week estimate)
→ Shortest path to validate the loop: MCP server + AE watcher + ae:analyze modification + batch import
