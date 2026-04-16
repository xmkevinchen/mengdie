---
id: "016"
title: "Analysis: Dreaming Evolution — From Passive Scoring to Active Knowledge Management"
type: analysis
created: 2026-04-16
tags: [dreaming, knowledge-compiler, entity-extraction, memory-lint, decay, compilation, landscape]
---

# Analysis: Dreaming Evolution

## Question

mengdie 的 Dreaming 只是个 recall+relevance 打分器，ingest 只是存储，recall 只是搜索。整个系统没有"智能层"。分析 llm-wiki 项目和更广泛的 second brain 领域，找到 mengdie 值得存在的方向。

## Findings

### Prior Art from Project Knowledge Base

- **[analyze] is_longterm has zero effect on search** (009-dreaming-promotion/analysis.md, factual, 2026-04-06): Dreaming subsystem was disconnected from retrieval — plan 004 later added 1.2x LONGTERM_BOOST but the finding validates the "empty shell" concern.
- **[analyze] recall_count inflates from session bursts** (009-dreaming-promotion/analysis.md, factual, 2026-04-06): No session dedup means a single ae:analyze session can inflate recall_count by 10x.
- **[analyze] Dreaming promotion permanently inert** (013/analysis.md, factual, 2026-04-16): RRF normalization capped scores at 0.50, threshold was 0.65 — mathematically unreachable. Plan 004 lowered to 0.45.

### Mengdie Current State (Archaeologist)

**What exists:**
- `dreaming.rs`: single SQL UPDATE, flips `is_longterm = 1` on rows meeting 3 thresholds. No demotion, no decay, no synthesis.
- `parser.rs`: entities 100% from frontmatter `tags:`. Missing tags = contradiction detection silently disabled.
- `contradiction.rs`: fires at ingest only. Entity-overlap + cosine heuristic. Never runs on a schedule.
- `search.rs`: RRF merge + 1.2x longterm boost. Side-effect: `record_recall()` on every hit feeds Dreaming.
- Schema: single flat table. No layers, no graph, no relationships.

**What doesn't exist:** demotion, decay, synthesis, lint, clustering, entity auto-extraction, connection graph, confidence scoring.

### Two llm-wiki Projects (Standards Expert)

**domleca/llm-wiki (Obsidian plugin):**
- Full LLM call per note for entity extraction (qwen2.5:7b via Ollama)
- 9 entity types, 9 connection types, vocabulary injection for dedup
- Hybrid search with query-type-aware RRF weights
- No dreaming/promotion concept

**ekadetov/llm-wiki (Claude Code plugin, Karpathy pattern):**
- Raw→Wiki→Output three-layer architecture
- Compilation: LLM synthesizes raw sources into wiki pages
- Lint: structural only (dead links, orphans, missing sections) — NOT semantic contradiction detection despite README claims
- Git-backed all changes

**Key finding:** Both projects use LLMs for the "intelligence" layer. Neither does post-hoc offline processing (their "compilation" is at ingest-time, not dream-time).

### Broader Landscape (Landscape Researcher)

**Closest sibling: Engram** — Go, SQLite+FTS5, MCP server, power-law decay (`strength *= 0.95^days`), prune threshold. Almost identical stack, already has mathematical decay.

**CortexGraph** — Ebbinghaus forgetting curve for AI memory. Validates mengdie's recall-based model but adds temporal decay math.

**LLM Wiki v2 (rohitg00/agentmemory)** — 43 MCP tools, confidence scoring (source count + recency + contradiction status), explicit supersession chains, memory lifecycle (birth → reinforcement → decay → death).

**Graphiti (getzep, ~14K stars)** — Temporal knowledge graphs with validity windows. Proves valid_from/valid_until works at scale.

**Field consensus:** Knowledge management ≠ search. The trend is toward "knowledge compilers" — synthesis, decay, typed relationships, health checks.

### Challenges (Challenger)

**Rejected directions:**
1. Auto entity extraction at Dreaming time — wrong boundary. AE outputs already have tags. Fix at ingest (term extraction from title+headings), not Dreaming.
2. Raw→Synthesized layering — architecturally backwards. AE outputs ARE synthesized. Adding immutability semantics to a search index is cargo-culting.
3. LLM-free compilation — can cluster but can't write summaries. Extractive clustering is useful; true synthesis needs LLM.

**Validated directions:**
1. **Decay** — highest ROI. is_longterm is one-way, never demotes. Power-law decay (Engram-style) closes a real monotonic growth problem. No LLM needed.
2. **Memory lint ("maintenance pass")** — 3 checks only: orphan GC, unresolved contradiction queue, embedding version drift. Deterministic, no LLM.
3. **Tag enforcement at ingest** — prevent silent contradiction-detection bypass. Title+heading term extraction covers 80% of cases.

**Premature directions:** Connection graph, entity taxonomy, 5-layer architectures — solving problems mengdie doesn't have at 200 entries.

### Feasibility Assessment (Codex)

| Direction | LOC estimate | LLM required? | ROI at 200 entries |
|---|---|---|---|
| Memory lint (3 checks) | 250-600 | No | High — immediate |
| Decay (power-law demotion) | 200-400 | No | High — prevents longterm bloat |
| Entity enrichment at ingest | 300-700 | No | Medium — improves contradiction coverage |
| Clustering in Dreaming | 300-600 | No | Medium — diagnostics, not production |
| Full synthesis/compilation | 1200-2000 | Yes | Low at 200 entries — defer |

## Summary

mengdie 的问题不是"Dreaming 怎么优化"，而是**整个系统没有 intelligence layer**。ingest 是存储，search 是检索，dreaming 是统计——三个动作都是机械的。

两个 llm-wiki 解决的是"非结构化笔记 → 结构化知识"的问题，对 mengdie 的结构化 AE 输入大部分不适用。但领域共识清晰：知识管理 ≠ 搜索。

**mengdie 最需要的不是模仿 llm-wiki 的功能清单，而是回答一个架构问题：intelligence layer 放在哪？**

三个可能：
1. **Ingest-time intelligence** — 更好的实体抽取、关系推断、冲突检测（需要 LLM 或高质量 NER）
2. **Dream-time intelligence** — 衰减、清理、聚类、合并（不需要 LLM，但产出有限）
3. **Query-time intelligence** — 让 AI 工具在读取记忆时做合成（现状，但 mengdie 无法控制质量）

**最务实的路径**：先做 dream-time 的 decay + lint（不需要 LLM，立即有用），同时在 ingest-time 加 tag 推断（提升矛盾检测覆盖），为将来的 LLM-backed synthesis 留好接口。

## Possible Next Steps

1. `/ae:discuss` — 讨论 Dreaming 2.0 的具体设计：decay 曲线选择（Engram 的 0.95^days vs Ebbinghaus）、lint 的 3 项检查实现、ingest tag 推断策略
2. 或者先跑 2 周验证当前系统，用数据决定优先级——但如 challenger 所说，当前系统的"验证"更像是"验证搜索能搜到东西"，意义有限
