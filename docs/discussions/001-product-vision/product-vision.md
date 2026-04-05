---
id: "001"
title: "AI-native Second Brain — Product Vision"
type: product-vision
created: 2026-04-05
status: draft
origin: "Claude Code source analysis (007) + OpenClaw memory system comparison + AE pipeline output analysis"
tags: [product, second-brain, memory, knowledge-management]
---

# AI-native Second Brain

## One-liner

AI writes and maintains. Humans consume and judge.

Traditional Second Brain (Obsidian / Logseq / Notion) problem: humans do all the writing, organizing, and connecting. AI-native flip: information flows in passively from tools, gets filtered and organized through actual usage, humans only step in for critical judgments.

---

## Where This Comes From

### Claude Code's Four Memory Mechanisms

Claude Code built four independent systems to keep its AI coding assistant from losing memory:

- **Compaction**: Auto-compresses conversation when nearing context limits. Three tiers — delete old tool results, replace summarized messages with Session Memory output, full compression. Integrated with Session Memory (tries SM-based compaction first, falls back to full).
- **Session Memory**: Background forked subagent extracts structured info using a fixed template (task state, file list, errors, learnings). Updates every 5000 tokens + 3 tool calls.
- **Scratchpad**: `/tmp` directory for coordinator mode — worker agents read/write freely. Dies with the session.
- **AutoDream**: Triggers after 24 hours + 5 sessions. Scans raw conversation logs, consolidates into persistent memory files.

Problem: AutoDream doesn't read Session Memory's structured extractions (re-greps raw logs from scratch). Scratchpad knowledge dies with the session. No quantitative mechanism for "what's worth remembering."

### What OpenClaw Got Right

OpenClaw's Dreaming system tracks every `memory_search` call. Weighted scoring (frequency 35% + relevance 35% + diversity 15% + recency 15%) decides whether to promote from short-term to long-term memory.

**Key distinction**: Not AI judging "is this important?" — usage behavior decides. Frequently recalled information is important. Never-recalled information naturally fades.

OpenClaw also abstracted context management into a pluggable interface (Context Engine), making memory backends swappable.

### AE Pipeline — A Natural Structured Knowledge Production Line

AE plugin's analyze → discuss → plan → work → review → retrospect flow produces structured Markdown at every step:

| AE Output | Contains | Why It's Valuable |
|-----------|----------|-------------------|
| conclusion.md | Decisions + rationale + evidence + reversibility + Doodlestein challenge records | Not just "we chose A" but "why A, what was considered, who dissented" |
| plan.md | Steps + expected files + acceptance criteria | Full implementation contract with drift detection baseline |
| review.md | P1/P2/P3 findings + fix records + Outcome Statistics | "What scenarios tend to produce what problems" |
| retrospect.md | Trend analysis (rework rate / P1 escape / drift / auto-pass) | "Is the team getting better or worse, and why" |
| backlog/*.md | Deferred work + reasons + priority | "What was postponed, why, when to revisit" |

These outputs are **natively structured** — YAML frontmatter, fixed schema, typed fields. No AI extraction needed; ingest directly.

More importantly: AE produces **decision context**, exactly what traditional Second Brain lacks. In Obsidian you can record "we chose React" but struggle to preserve "why React, what was considered, who objected, how it validated later." AE output naturally includes all of this.

### What's Missing

Three projects (Claude Code / OpenClaw / AE) each solved part of the problem, but nobody connected them:
1. Claude Code and OpenClaw only capture AI chat information. AE's structured output, GitHub reviews, Slack discussions don't flow in automatically.
2. No associations between memories. A and B are related but finding A doesn't surface B.
3. Individual-only. No "team shared memory" concept.
4. **No feedback loop**: AE produces knowledge → but next AE run doesn't consult prior knowledge. Every run starts from scratch.

---

## Core Design: Upward Spiral Feedback Loop

The core of this product is not a four-layer architecture — it's a feedback loop:

```
AI Workflow (Claude Code / AE pipeline / OpenClaw)
    │
    │ Produces knowledge (conversation experience, decision records, review findings, trends)
    ↓
Second Brain: Passive Ingestion
    │
    │ Behavior-driven filtering (Dreaming: frequency × relevance × diversity × recency)
    ↓
Structured Long-term Memory (personal + team)
    │
    │ Auto-association (semantic + co-occurrence + timeline evolution)
    ↓
Input context for next AI workflow
    │
    │ "What was decided last time in a similar scenario? How did it turn out?"
    ↓
Better AI output → Higher-quality knowledge → Richer Second Brain → ...
```

**Not linear input→process→output. It's an upward spiral**:
- First `ae:analyze` on a problem: research from scratch
- Analysis results enter Second Brain
- Second time a similar problem arises: `ae:analyze` automatically pulls prior decisions and experience as input context
- This analysis is higher quality because it doesn't re-discover what was already known
- New results feed back into Second Brain, correcting or supplementing prior knowledge

Each turn deepens accumulated knowledge; each AI workflow starts from a higher baseline.

### How the Loop Closes

**Output side (AI workflow → Second Brain)**:

| Source | Ingestion Method | Content | Structure Level |
|--------|-----------------|---------|----------------|
| Claude Code | SessionEnd hook | What changed, errors encountered, how resolved | Medium (Session Memory template) |
| AE pipeline | File watcher or post-write hook | Decisions + rationale + evidence + challenges + validation | **Very high** (fixed schema) |
| OpenClaw | Existing session-memory hook | Conversation summaries, action items | Medium |
| GitHub | Webhook | PR review architectural opinions, issue discussions | Low-medium |
| Slack / Discord | Bot | Technical discussions, decisions | Low |

**Input side (Second Brain → AI workflow)**:

| Consumer | How It Pulls | What It Gets |
|----------|-------------|--------------|
| `ae:analyze` | Auto-search Second Brain before synthesis | Prior analyses and decisions from same project/domain |
| `ae:discuss` | Load related prior decisions before discussion | "We discussed a similar topic before, conclusion was X" |
| `ae:plan` | Reference prior review findings and retrospect trends | "This module's last review found DB migration tends to produce P1s" |
| `ae:work` | Reference prior experience with similar tasks | "Last time this API was modified, watch out for timeout settings" |
| Claude Code daily use | `memory_search` auto-trigger | Related past experience and decisions |

**The loop's key**: AE pipeline and Claude Code don't need core logic changes. Only two hooks:
1. At AI workflow completion: auto-ingest produced knowledge into Second Brain
2. At AI workflow start: auto-inject relevant Second Brain content into context

Both are hooks — no core code invasion.

---

## Product Design

### Positioning

For technical teams (3-15 people) using AI tools. Not a Notion competitor (that's a collaboration doc tool). This is the "memory layer" for AI workflows — all knowledge produced by AI tools auto-aggregated, auto-filtered, auto-associated, and fed back into AI workflows.

### Target Users

- Engineers using Claude Code / Cursor / Copilot daily
- Technical teams using AE pipeline or similar tools for research, design, code review
- Tech leads who want "what one person learns, others don't have to re-discover"

### Non-goals

- Not a document tool (not competing with Notion / Confluence)
- Not a note-taking app (not competing with Obsidian / Logseq)
- Not a search engine (not competing with Glean / Dashworks)
- Only does one thing: manage and feed back knowledge produced by AI workflows

---

## Four-Layer Architecture

### Layer 1: Passive Ingestion

**Core principle**: Users never need to "remember to write things down." Information flows in from tools automatically.

**Two categories**:

**Operational memory** — "what was done today":
- Claude Code session hook: files changed, errors hit, how they were resolved
- OpenClaw session-memory hook: conversation summaries, action items

**Decision memory** — "why it was done this way":
- AE pipeline output: conclusion.md (decisions+rationale), plan.md (approach+criteria), review.md (findings+trends)
- GitHub webhook: architectural opinions in PR reviews
- Slack/Discord bot: decisions from technical discussions

**Ingestion ≠ remembering.** Everything ingested enters short-term storage (daily notes) at near-zero cost. Only what's validated by the filtering layer enters long-term memory.

AE pipeline output, being natively structured (YAML frontmatter + fixed schema), can be directly extracted as knowledge entries without AI doing additional understanding or transformation. This is why it's more efficient than other sources.

### Layer 2: Behavior-Driven Natural Selection

**Core principle**: What gets repeatedly used in actual work is worth remembering long-term. No dependency on AI "judgment."

**Scoring flow**:

```
Short-term storage (daily notes, all ingested info)
    ↓ Track every search / reference / AI-use event
    ↓ Score: frequency × relevance × diversity × recency × [team level: team prevalence]
    ↓
Long-term memory
    ↓ Monitor: unused for extended period → score decay → demote back to short-term or archive
    ↓
Archive (inactive but present — lower weight in search, not proactively surfaced)
```

**Loop accelerator**: When a memory is referenced by AE pipeline (e.g., `ae:analyze` pulls a prior decision as input), that reference is a high-weight "usage" event. Higher-quality AE output → more frequent subsequent AE references → higher Dreaming score → more persistent in long-term memory. **Good knowledge self-reinforces; poor knowledge naturally fades.**

**Contradiction detection**: When new information conflicts with existing memory, auto-flag it. Don't auto-delete the old one — mark "these two entries conflict, you may need to judge." Flagged conflicts surface both sides in search results for user or AI to make the final call.

### Layer 3: Auto-Association

**Core principle**: Knowledge value lies not just in individual entries, but in connections between them.

**Three association mechanisms**:

1. **Semantic similarity**: Vector similarity above threshold → weak association. "These two notes are about related things."
2. **Co-occurrence**: A and B frequently retrieved together in the same conversation/work context → usage association. Stronger signal than semantic similarity — two entries may look unrelated textually but are frequently used together in practice.
3. **Timeline evolution**: Same topic's understanding changes over time. "Jan: decided on approach A (discussion 001). Mar: review found A has perf issues (review 003). May: switched to approach B (discussion 007)." Don't delete old entries — annotate the evolution relationship.

**Significance for the loop**: When `ae:analyze` searches Second Brain, it gets not just direct matches but associated memories. "You're asking about API timeout. Direct match: set 30s last time. Timeline: earlier tried 10s, review found it insufficient. Co-occurrence: this API frequently has issues alongside the auth module."

Richer input context → higher quality AI output → stronger upward spiral.

### Layer 4: Team Level

**Core principle**: When multiple team members independently learn the same thing, it's organization-level knowledge.

**Two storage tiers**:

```
Personal memory (private, only visible to me)
    ↕ Auto-suggest promotion / manual demotion
Team memory (shared, visible to team members)
```

**Promotion mechanism**: When N people (default 3) on a team have semantically similar entries in their personal memories, the system auto-suggests promotion to team memory. Not auto-promoted — privacy and accuracy require human confirmation.

**Special team memory source**: AE pipeline's conclusion.md and review.md are inherently team-level outputs (multi-agent discussion decisions, multi-perspective review findings). These can enter team memory directly without the "3 people recorded similar things" detection — they already represent team consensus.

**Team loop**: New hire onboards → auto-loads team memory → first `ae:analyze` gets accumulated team decision context → no need to dig through history or ask colleagues.

---

## Technical Foundation

### Directly Reusable

| Source | Component | Use |
|--------|-----------|-----|
| OpenClaw | Context Engine interface | Standard ingestion interface |
| OpenClaw | Dreaming scoring model | Core filtering algorithm |
| OpenClaw | memory_search (semantic+keyword hybrid) | Search layer |
| OpenClaw | Pluggable backends (SQLite/LanceDB) | Storage layer |
| Claude Code | Session Memory template | Structured extraction for operational memory |
| Claude Code | Permission system design | Reference for team-level access control |
| AE plugin | Full output schema | Direct source for decision memory |
| AE plugin | pipeline.yml config pattern | Output paths and naming conventions |

### Needs Building

- Multi-source ingestion adapters (Claude Code hook, AE output watcher, GitHub webhook, Slack bot)
- Association discovery engine (semantic similarity + co-occurrence + timeline)
- Team Dreaming (cross-personal-memory similarity detection + promotion mechanism)
- Contradiction detection and flagging
- Decay and archival mechanism
- **Feedback hook**: Pull relevant context from Second Brain at AI workflow start

### Not Needed

- Knowledge graph (over-engineering; simple association lists suffice)
- NL-generated summaries (score-based filtering is more reliable than AI-generated summaries)
- Complex UI (target users are engineers; Markdown + CLI + IDE plugin is enough)

---

## Key Risks

### "Garbage in" risk

The enemy of all passive ingestion systems: too much noise.

Mitigation:
- AE pipeline output is natively structured, very low noise — the cleanest source
- Other sources get lightweight triage at ingestion
- Better to ingest some noise (Dreaming filters it out) than miss valuable information

### Cold start problem

New users have no usage behavior data; Dreaming can't score.

Mitigation: If user has existing AE pipeline output (e.g., 20+ discussions), batch-import directly — cold start becomes hot start. No AI judgment fallback needed for cold start period.

### Privacy

At team level, personal memory may contain private information. Promotion to team memory must go through human confirmation.

### Credibility

AI-extracted information may be inaccurate. Every memory annotated with source (which conversation/tool/timestamp). Search results show provenance for users to judge. Long-unvalidated memories auto-downweight.

### Negative spiral in the feedback loop

If early incorrect knowledge gets referenced by subsequent AI workflows, producing new knowledge based on wrong premises, the error self-reinforces.

Mitigation: Contradiction detection is the key defense. When new information conflicts with existing memory, flag the conflict instead of silently overwriting or perpetuating. Source provenance lets users trace errors to their origin.

---

## Minimum Viable Product

**Phase 1: Personal feedback loop**
- AE pipeline output watcher (decision memory)
- Local SQLite + vector index
- MCP server with `memory_search`, `memory_ingest`, `memory_invalidate`
- Simplified Dreaming (frequency + relevance scoring)
- **Feedback hook**: `ae:analyze` post-research auto-searches Second Brain, injects relevant context

This phase validates whether the loop works: AE output → ingest → next AE references it → better output.

**Phase 2: More sources + associations**
- Claude Code session hook (operational memory)
- GitHub webhook + Slack bot
- Lightweight triage at ingestion
- Semantic similarity + co-occurrence associations

**Phase 3: Team level**
- Cross-personal-memory similarity detection
- AE team output goes directly to team memory
- Promotion mechanism + confirmation flow
- Team search (merged personal + team libraries)

---

## Open Questions

1. **Storage format**: Markdown files (human-readable, git-manageable) vs database (performance, associations)? OpenClaw's Markdown + vector index combo may be the best compromise.

2. **Feedback granularity**: How much context to inject into AI? Too little is useless, too much wastes tokens. May need dynamic adjustment based on task type and context window budget.

3. **Business model**: Personal free/open-source + team level paid? Or fully open-source with hosted service revenue?

4. **Relationship with OpenClaw**: As an OpenClaw plugin or independent product? Reuse OpenClaw components underneath (Context Engine, Dreaming, memory_search) but maintain independent product positioning.

5. **AE output deduplication**: A discussion may produce index.md + analysis.md + conclusion.md + multiple topic + round files. Ingesting everything creates massive duplication. Ingest only "final outputs" (conclusion.md, plan.md, review.md) or ingest everything and let Dreaming filter?
