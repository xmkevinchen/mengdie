---
id: "016"
title: "Dreaming Evolution — Phase 2 Roadmap — Conclusion"
concluded: 2026-04-16
plan: ""
entities: [llm, provider, llm-provider, dreaming, synthesis, dream-synthesis, phase, sequencing, phase-sequencing, daemon, knowledge, graph, knowledge-graph, rag, search, rag-search, decay, clustering, entity, extraction, entity-extraction]
---

# Dreaming Evolution — Phase 2 Roadmap — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | LLM Provider Architecture | Reuse SmartPal/OpenClaw provider pattern — CLI provider for Claude, OAuth for OpenAI. Wrap behind `LlmClient` trait. Zero API cost (existing subscriptions). | User already has this working in two other projects. No need for reqwest + raw API when proven provider infra exists. Study SmartPal/OpenClaw source to extract reusable pattern. | high — trait abstraction allows swapping providers freely |
| 2 | Phase Sequencing | 5 vertical slices: 2.1 Dream MVP → 2.2 Daemon + queue → 2.3 RAG search → 2.4 Knowledge graph → 2.5 Feedback loop. Phase by user-visible capability, not technology layer. Decay bundled into 2.1. | Codex's "smallest proving loop" argument: prove LLM synthesis works before building infrastructure. Architect + Gemini concurred after Round 2. Each phase ships standalone value. | high — each phase additive, no schema breaks |
| 3 | First Deliverable | `mengdie dream` adds LLM synthesis stage after existing promotion pass. Cluster memories by fastembed cosine similarity → pass each cluster to LLM → store as source_type="synthesis". | Architect proposed, Codex reinforced, Doodlestein-strategic added clustering pre-step. Synthesis memories surface in existing memory_search → ae:analyze Round 0 picks them up. Loop closed. | high — synthesis memories are additive, originals unchanged |

## Doodlestein Review

| Agent | Challenge | Resolution |
|---|---|---|
| Strategic | Cluster by embedding similarity before LLM call — don't let LLM pick groupings | Adopted: fastembed clustering (~50 LOC) becomes the data prep step inside Phase 2.1 |
| Adversarial | Synthesis output has no consumer; LlmClient trait over-engineered; naming collision with `dream` | Resolved: (1) consumers = memory_search → ae:analyze Round 0, (2) use SmartPal/OpenClaw provider pattern instead of raw reqwest, (3) extend existing `dream` as two-stage: promote + synthesize |
| Regret | Haiku model swap to Sonnet within weeks — synthesis is a reasoning task | Accepted risk: trait abstraction handles swap. Start with whatever works via CLI provider. |

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| TL | Moderator | Claude | Start |
| architect | Solution design, dependency analysis | Claude | Start |
| codex-proxy | LLM API patterns, cost modeling | Codex | Start |
| gemini-proxy | KG schema, competitor analysis | Claude (Gemini unavailable) | Start |
| doodlestein-strategic | Strategic improvement | Claude | Doodlestein |
| doodlestein-adversarial | Blind spot detection | Claude | Doodlestein |
| doodlestein-regret | Regret prediction | Claude | Doodlestein |

## Process Metadata

- Discussion rounds: 2 (Round 1 independent research, Round 2 cross-challenge)
- Topics: 3 total (3 converged)
- Autonomous decisions: 3
- User escalations: 0
- Doodlestein challenges: 3 raised, 3 resolved, 0 reopened
- Deferred resolved in Sweep: 0

## Key Architecture Decisions

### Dual Runtime Model
- **MCP server** (`mengdie-mcp`): thin transport — search, ingest, invalidate. No LLM calls.
- **Daemon/CLI** (`mengdie dream`): intelligence hub — LlmClient, synthesis, decay, future queue/extraction.
- MCP stores immediately, intelligence runs async or on schedule.

### Provider Pattern (from user input)
- Claude: CLI provider (reuse existing subscription via `claude` CLI)
- OpenAI/Codex: OAuth token (SmartPal/OpenClaw pattern)
- Zero additional API cost
- Study `../SmartPal` and OpenClaw source for reusable provider code

### Phase 2.1 Dream MVP — Detailed Scope
1. `LlmClient` trait + provider impl (from SmartPal/OpenClaw pattern)
2. Embedding clustering: load vectors, greedy cosine clustering (threshold ~0.75), min cluster size 3
3. LLM synthesis: pass each cluster to LLM → "synthesize these related memories into one consolidated insight"
4. Store synthesis as `source_type = "synthesis"` in DB (searchable via existing MCP)
5. Power-law decay: `recall_weight *= 0.95^days_since_last_recall` (bundled, trivial)
6. Config: `~/.mengdie/config.toml` for provider settings

### Future Phases (not planned in detail yet)
- 2.2: Daemon process + SQLite job queue + async entity extraction on ingest
- 2.3: `memory_query` MCP tool (RAG: retrieve + synthesize answer)
- 2.4: `memory_relationships` table + typed edges + graph-aware search
- 2.5: `memory_feedback` tool + RL-like signal for Dreaming tuning

## Next Steps

→ Study SmartPal (`../SmartPal`) and OpenClaw source for provider pattern
→ `/ae:plan` for Phase 2.1 Dream MVP
