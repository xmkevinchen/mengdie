---
title: "OSS coding tooling + AI memory ecosystem survey (2026-05)"
type: survey
created: 2026-05-08
status: draft
hard_cap:
  primary_projects: 13   # 12 original + 1 for §4 floor compliance (per F-004 review fixup 2026-05-08; AC1 "每 section ≥3 entries" enforcement bumped cap from 12 → 13)
  trends: 5
  implications: 3
  time_budget: 6h
purpose: |
  Vision-driving + v0.0.1-implementation-driving input for F-004 multi-version
  roadmap (docs/roadmap.md). Scope per BL-028 L65-69: 4 dimensions —
  OSS coding agent ecosystem, MCP ecosystem, OSS memory frameworks, success/failure cases.
methodology: |
  WebSearch + WebFetch + GitHub for each candidate. Each entry must include 5 fields:
  as_of (调研日期), maturity (release/commits/contributors), licensing,
  borrowable (mengdie 能借什么), intersection (竞争/互补/可整合/不相干).
  Hard cap: ≤12 主项目 / ≤5 trends / ≤3 implications / ≤6h.
  Overflow → follow-up BL, do NOT inline expand.
---

# OSS Coding Tooling + AI Memory Ecosystem Survey (2026-05)

## §1 OSS 编码 agent 生态

> **Editorial note (2026-05-08)**: Plan Step 1 列举的候选含 Claude Code / Cursor / Roo Code，最终 §1 只 cover Aider / Continue / Cline / OpenHands 4 项。Claude Code + Cursor 移到 §2 "MCP host tools" — 两者主要作 host 角色（前者 proprietary，后者 closed-source 主体 + open parts 不构成 standalone OSS coding agent 类别）。Roo Code 是 Cline 的 fork，Cline 已覆盖核心模式，不重复列。这是 editorial 选择，hard cap ≤12 主项目下的取舍。

### Aider

- **as_of**: 2026-05-08
- **maturity**: 44.3k GitHub stars / 4.3k forks (Aider-AI/aider, fka paul-gauthier/aider). v3.x active release line; self-coding ratio 70-80% per commit history (project dogfoods itself). SOTA on SWE-Bench Lite + main benchmark. Single primary maintainer + community contributors.
- **licensing**: Apache 2.0
- **borrowable**: (a) Terminal-first UX 范式 (mengdie CLI 也是 terminal-first，可参照 Aider 的 prompt → edit → commit 流程); (b) Auto-git-commit-with-descriptive-message 模式 (mengdie ingest 后写入 SQLite 时的 source_file traceability 类似)。
- **intersection**: 不相干 (Aider 是 code-editing agent，mengdie 是 memory store)。但 Aider 用户**未来**可调用 mengdie MCP server 作为 cross-session memory（如 mengdie 走 generic MCP 路径）。当前 v0.0.1 thesis 不绑定。

### Continue.dev

- **as_of**: 2026-05-08
- **maturity**: 33k stars / 4.5k forks (continuedev/continue). TypeScript codebase, ~33k files. Last update 2026-05-06 (extremely active). Multi-IDE support (VS Code + JetBrains). Recent: AST-based deterministic edits, GPT-5 search/replace, MCP server OAuth.
- **licensing**: Apache 2.0
- **borrowable**: (a) **MCP server OAuth 模式** — Continue 已实现 MCP server OAuth 认证，如 mengdie 未来走 multi-host MCP 路径，OAuth 是必经之路，参考 Continue 实现; (b) **AST-based deterministic edits** — Continue 用 AST 而非 full-file rewrite，思路对 mengdie 处理 ingest 时的"片段级覆盖" 决策有借鉴; (c) Tool calling 跨 LLM 兼容模式（mengdie MCP tool description 风格参考）。
- **intersection**: **互补** — Continue 用户用 mengdie 作 memory MCP server。Continue 是 host，mengdie 是 server。即使 v0.0.1 thesis 是 AE-brain 优先，Continue 这条接入路径成本极低（MCP 标准），1-2 年后可低成本扩展。

### Cline

- **as_of**: 2026-05-08
- **maturity**: 61.4k stars / 6.4k forks (cline/cline). v3.82.0 latest. **5M+ installs** across VS Code Marketplace + Open VSX + JetBrains + CLI（≥3.85M 单 VS Code Marketplace）。20k+ Discord 成员。$32M 融资（Emergence Capital + Pace Capital）。最早大规模采纳 MCP 的 OSS 编码 agent 之一。
- **licensing**: Apache 2.0
- **borrowable**: (a) **MCP server 集成范式** — Cline 是早期 MCP host 采纳者，其 MCP server 接入流程是事实标准，mengdie 的 MCP description 字段需对得上 Cline-style host 的 expectations; (b) Plan/Act 双模式 UX（与 mengdie ingest/recall 双向流类似）; (c) Apache 2.0 license + open governance 模式（如 mengdie 走 handover 路径，Cline 是参考）。
- **intersection**: **互补** — 同 Continue，Cline 用户可调用 mengdie 作 memory MCP server。Cline 5M+ installs 是 mengdie 后续可触达的最大潜在 host pool。

### OpenHands

- **as_of**: 2026-05-08
- **maturity**: 70k+ stars / 490+ contributors (OpenHands/OpenHands, fka OpenDevin)。v1.6.0 (2026-03-30) 加 Kubernetes + Planning Mode beta。SWE-bench Verified 53%+（配 Claude 4.5）。学术血统（arXiv 2407.16741）+ 商业化 (All-Hands-AI 组织化运营，OpenHands Index 2026-01)。
- **licensing**: MIT
- **borrowable**: (a) **CodeAct agent loop 设计** — natural language task → stepwise plan → shell+file ops → test-suite-after-each-change → iterate-until-green。这个"每步后跑 test"模式对 mengdie 的 ingest validation 思路有借鉴; (b) Sandboxed Docker 执行模式（mengdie 当前不需要，但如未来 dream 阶段做 LLM 内部 synthesis 验证可参考）; (c) SWE-bench Verified evaluation harness 思路。
- **intersection**: 不相干（OpenHands 是 autonomous engineer，mengdie 是 memory）。**潜在互补**：OpenHands 内部已有 memory 子系统，如未来 mengdie 走 generic MCP 且接入 OpenHands 作为 external memory，竞争 OpenHands 内置 memory；当前 v0.0.1 不绑定，无短期交集。

## §2 MCP 生态

### MCP 协议 + SDK landscape

- **as_of**: 2026-05-08
- **maturity**: **97M 月 SDK 下载** (Anthropic 官方统计)。**5,800+ MCP servers** 公开注册表（1,200 in Q1 2025 → 9,400+ in Apr 2026，MoM +18%）。**7,800 GitHub repos** 带 `mcp-server` topic。TypeScript SDK 34,700+ dependent projects。Spec 已扩展到 **5 primitives**: tools, resources, prompts, sampling, roots（April 2026）。
- **licensing**: MCP spec — MIT (modelcontextprotocol org)；Anthropic 官方 SDK — Apache 2.0 / MIT
- **borrowable**: (a) **5 primitives 全集**：mengdie 当前只用 tools，未来可扩展 resources（暴露 memory entries 作 read-only resources）+ sampling（让 host LLM 触发 mengdie 内部 dream）；(b) **TypeScript SDK + Python SDK 现状成熟**，mengdie 当前用 rmcp Rust SDK 是少数派但 Anthropic 官方支持的 SDK 正在覆盖 Rust（rmcp v1.3 跟标准跟得紧）。
- **intersection**: **底座**（不是竞争）。mengdie 是 MCP server，MCP 协议是 mengdie 存在的物理前提。MCP spec 演进 → mengdie 须跟（rmcp 跟 Anthropic spec 跟）。

### Memory-shaped MCP servers (mem0 / Letta / Graphiti MCP)

- **as_of**: 2026-05-08
- **maturity**: 三大 OSS memory framework 均已发布 MCP server 适配：
  - **mem0-mcp** — 9 MCP memory tools + lifecycle hooks，2026-Mar-Apr 上线 "Mem0 Plugin for AI Editors"；OpenMemory Cloud (managed) + self-hosted 双部署。
  - **Letta MCP** — Letta v0.16.7 配套 MCP server，stateful agent memory 暴露给 host LLM。
  - **Graphiti MCP server** (`graphiti/mcp_server/`) — Knowledge graph memory 表面，bi-temporal validity 暴露为 MCP tool。
  另有大量第三方 mem0 MCP wrappers（如 coleam00/mcp-mem0）。
- **licensing**: 全部 Apache 2.0 / MIT 类
- **borrowable**: (a) **MCP tool naming + parameter shape**：mem0 9 tools / Letta / Graphiti 已 settle 了 memory MCP 通用命名（add_memory / search_memory / get_memories / update_memory / delete_memory），mengdie 的 `memory_search` / `memory_ingest` / `memory_invalidate` 命名跟得上行业标准；(b) **bi-temporal validity 接口**（Graphiti）：mengdie contradiction.rs 已有 valid_from/valid_until 概念，暴露为 MCP field 时可参考 Graphiti 的接口形状；(c) **lifecycle hooks**（mem0）：mengdie 的 dream synthesis 是 batch async，mem0 的 hook 思路（pre-add / post-add / pre-search / post-search）可借鉴让 dream 切到 incremental 触发。
- **intersection**: **直接竞争**（critical 发现）。mem0/Letta/Graphiti 都已是 MCP-aware memory server。mengdie 跟它们的差异点必须能讲清楚 → 见下方 §4 案例 + roadmap implications。**v0.0.1 narrow OSS-adoption thesis** 在此处面临真正考验：sqlite-vec + FTS5 是否能给出 mem0 / Graphiti 给不到的特化（如 AE-plugin 紧耦合 / propositional fact / 中文 tokenization）。

### MCP host tools (Claude Desktop / Cursor / VS Code / Continue)

- **as_of**: 2026-05-08
- **maturity**: 主流 MCP host 全覆盖：**Claude Desktop**（native，spec 共建者）、**Claude Code CLI**（mengdie 当前 host）、**ChatGPT**（Apps SDK + Connectors，2025-04）、**Google Gemini API + Vertex AI Agent Builder**（2026-03）、**Cursor**、**Windsurf**、**Zed**、**JetBrains AI Assistant**、**Vercel AI SDK**、**OpenAI Agents SDK**。**67% CTOs** 调研称 MCP 12 个月内成默认 agent-integration 标准。
- **licensing**: 各异 — Claude Desktop / Cursor proprietary；其余 (VS Code Continue / Zed / Windsurf 部分) Apache 2.0 / MIT
- **borrowable**: (a) **MCP description 字段标准**：跨 host 都按 < 200 tokens 设计 tool description（Spec C 已采纳）；(b) **配置文件格式**：Claude Code 用 `~/.claude/settings.json` 注册 server，跨 host 配置 schema 趋同（mengdie-cli spec 应记录这一点）；(c) **MCP host 反馈循环**：host 工具的 tool-call 结果直接流回 LLM 上下文 — mengdie 的 search 返回 shape 应优化为 LLM-friendly（紧凑 JSON + 重点 metadata）。
- **intersection**: **载体**（不是竞争）。每个 MCP host 是 mengdie 一个潜在 deployment surface。v0.0.1 thesis 是 Claude Code 单 host；1 年后向 Continue / Cline / Cursor 扩展是低成本（同 MCP 协议）。

## §3 OSS memory frameworks

### mem0

- **as_of**: 2026-05-08
- **maturity**: **53.5k stars / 6k forks** (mem0ai/mem0) — **OSS memory ecosystem 第一**（最 starred + 最 funded）。**v2.0.0** (2026-04-16) — entity linking 取代旧 graph memory。21 frameworks/platforms 集成（Python + TypeScript SDK）。3 部署模式：managed cloud / self-hosted OSS / local MCP。
- **licensing**: Apache 2.0
- **borrowable**: (a) **Entity linking 范式**：mem0 v2.0.0 把"graph memory"重做为更轻的 entity-linking — mengdie 当前 entity-tag directed comparison（contradiction.rs）思路类似，可参考 mem0 的 entity 抽取 prompt 工程; (b) **9-tool MCP API**（add_memory / search_memory / get_memories / update_memory / delete_memory / 等）— mengdie MCP tool surface 应跟 mem0 命名对齐（已对齐：memory_search / memory_ingest / memory_invalidate）; (c) **State of AI Agent Memory 2026** 报告（mem0 自家发的）— 行业基准数据可引用，避免重做调研。
- **intersection**: **最直接竞争**（severity high）。mem0 是 mengdie 的"为什么不直接用 mem0"质问的主体。mengdie v0.0.1 的差异化必须针对 mem0：
  - mem0 是 generic 多语言 memory；mengdie 是 **AE-plugin 专属 + 中文优化** (FTS5 trigram tokenizer 已在 docs/discussions/004 settled)
  - mem0 是 cloud-first，self-hosted 是次选；mengdie 是 **local-first / claude-CLI inherits credentials**（无外部遥测）
  - mem0 的 propositional fact 抽取是 LLM 触发；mengdie 是 AE pipeline 已 structured（discussion → conclusion → ingestion，信号 噪音比远高）

### Letta (formerly MemGPT)

- **as_of**: 2026-05-08
- **maturity**: **22.4k stars** (letta-ai/letta，from 13k+ in MemGPT 时代). **v0.16.7** (2026-03-31) — global context window 32k → 128k，compaction 重写。**$10M Felicis seed**。学术血统（UC Berkeley Sky Computing Lab，MemGPT paper），现已商业化。**Context Repositories** — programmatic context 管理 + git-based versioning（独特设计）。
- **licensing**: Apache 2.0
- **borrowable**: (a) **Stateful agent runtime 模式**：Letta 把 LLM context 当 virtual memory 管，分段、page-in/out — mengdie 当前不需要 stateful agent 但 Letta 的 context-as-memory 类比对未来 dream 阶段处理"长 context 触发的 synthesis"有借鉴; (b) **Context Repositories git-based versioning**：mengdie 的 audit substrate（F-002）已有 audit_returned_facts 表，Letta 的 git-versioning 思路对 mengdie 后续 fact-versioning（temporal validity）有强参考价值; (c) Compaction 重写经验 — mengdie 没有 context compaction 概念但 dream synthesis 是类似职责，可借鉴 Letta 的 compaction 策略选型。
- **intersection**: **次级竞争 + 远期参考**。Letta 是 stateful agent framework + memory；mengdie 是 memory-only。Letta 偏 agent 端，mengdie 偏 store 端。短期不冲突，1-2 年后 Letta 如下沉到 memory-as-MCP-server（已有部分）则会重叠。

### Graphiti (Zep)

- **as_of**: 2026-05-08
- **maturity**: **20k+ stars** (getzep/graphiti). Zep 公司出品 (Y Combinator 系)，论文 "Zep: A Temporal Knowledge Graph Architecture for Agent Memory"。**Bi-temporal model** — 每条 edge 含 `event_time`（事件实际发生时间）+ `ingest_time`（mengdie 学到的时间）+ validity intervals。**MCP server** 已发布（`graphiti/mcp_server/`）。Neo4j 后端为主。
- **licensing**: Apache 2.0
- **borrowable**: (a) **Bi-temporal data model（强借鉴）**：mengdie contradiction.rs 当前只有 valid_from / valid_until（单 temporal），Graphiti 的 event_time + ingest_time 双时间是 superior 模型；mengdie schema.rs 应在 v0.0.1 后吸收（filed as follow-up BL，不在 F-004 内）; (b) **Knowledge-graph-from-unstructured-text 抽取流程**：mengdie 当前 ingest 是 markdown frontmatter + entity-tag，Graphiti 的"autonomous build context graph from unstructured data"对 mengdie 未来从非结构化 chat 历史 ingest 时有参考价值; (c) **Validity interval API shape**：mengdie 的 contradiction MCP tool（如未来出）应跟 Graphiti 的 API 对齐。
- **intersection**: **互补 + 远期借鉴**。Graphiti 是 knowledge graph memory（图存储 + Neo4j），mengdie 是 propositional fact memory（SQLite + FTS5/vector）。两者数据模型差异大，短期不冲突。**v0.0.1 不上 KG**（Karpathy minimum + KG 复杂性远超 v0.0.1 的 narrow scope），1-2 年后如确实需要 graph 关系（多 entity 复合查询），考虑借 Graphiti 而非自建。

## §4 OSS coding tool product 案例

### mem0 vs Quivr — survival comparison

- **as_of**: 2026-05-08
- **maturity**: **Quivr 39k stars (steady from 38k in 2024-Q1)** vs **mem0 53.5k stars (后起，2024 mid 启动 → 2026 反超)**。Quivr 仍 maintained，但 momentum 已转移；mem0 1-2 年内成 OSS memory ecosystem 第一。
- **licensing**: 两者都 Apache 2.0
- **borrowable**: **scope discipline 教训**（核心 borrowable 不是技术，是定位）：
  - **Quivr 定位过宽**："Opinionated RAG framework + chat UI + 5000 vector DBs on Supabase + second brain" — 试图同时是 framework / SaaS / consumer product。
  - **mem0 定位锐利**："Memory layer for AI agents" — 单一抽象层，不做 UI，generic backend。
  - 两年内结果：mem0 成为大多数 AI agent 项目的 memory 选择，Quivr 留在 "另一个 RAG demo" tier。**mengdie 应学 mem0**，不学 Quivr。
- **intersection**: 不直接（两者都不影响 mengdie 实施），但**mengdie 的 v0.0.1 narrow scope thesis 直接对应 mem0 模式**：不做 UI、不做 SaaS、专注一层抽象（"AE-brain"）。Karpathy minimum + 此 case = 强收敛信号。

### Letta MemGPT pivot — academic → product

- **as_of**: 2026-05-08
- **maturity**: 时间线：**2023** MemGPT paper（UC Berkeley Sky Computing Lab，arXiv "MemGPT: Towards LLMs as Operating Systems"）→ **2024** 学术 OSS 版 13k+ stars → **2024 Q4** 改名 Letta + 商业化 → **$10M Felicis seed** → **2026-Q2** 22.4k stars / v0.16.7。从 paper 到 funded company 约 18 个月。
- **licensing**: Apache 2.0
- **borrowable**: **commercialization vs 1-person OSS 路径分叉教训**：
  - Letta 路径需要：学术血统（UC Berkeley 团队）+ 创业团队 + VC 融资 + product runtime + cloud + enterprise sales。1-人 personal project 复制不到。
  - mengdie 不在这条路径上 — 是 **Karpathy "build your own personal LLM tool" 路径**（参考 Andrej "build something for yourself you'd want to use yourself"）。两条路径都 valid。
  - 启示：mengdie 不需要 Letta-grade runtime / context repositories / git-versioning。这些是 Letta 的 product 卖点，不是 mengdie 的 minimum。如果 mengdie 1 年后想走 Letta 路径，**必须成立团队 + 融资**，仅靠 1 人是不够的。
- **intersection**: 远期参考（不直接竞争）。Letta 是 stateful agent runtime，mengdie 是 memory store。如 mengdie 永远 personal，跟 Letta 永不交叉。如 mengdie 1-2 年后想做 generic memory MCP server 商业化，需要参考 Letta 商业化路径（**远期考虑，v0.0.1 thesis 明确 reject**）。

### Zep → Graphiti pivot — open core + commercial layer 模式

- **as_of**: 2026-05-08
- **maturity**: 时间线：**2023-2024** Zep 是 full-stack agent memory framework（含 chat history、session management、knowledge retrieval、商业化产品）→ **2024-2025** 战略 pivot：**抽出 Graphiti 作 OSS 时间图引擎核心**（getzep/graphiti，Apache 2.0，20k+ stars 2026-Q2）+ Zep 公司继续作 commercial product（managed cloud + enterprise feature）。Graphiti 独立成 GitHub repo + 单独 release cadence + 单独 docs；Zep cloud built-on-top。
- **licensing**: Graphiti = Apache 2.0；Zep cloud = proprietary commercial product
- **borrowable**: **decompose-monolith pattern** — 跟 Letta MemGPT pivot 形成 **对比 case**：
  - Letta = "everything OSS + commercial cloud add-on"（Apache 2.0 framework + Letta Cloud SaaS；framework 几乎全功能）
  - Zep = "OSS engine extracted + commercial product separate"（Graphiti core 是 OSS，Zep 是 product；两者代码 boundary 清晰）
  - mengdie 远期参考（fork-in-road 1-2 年后）：如 mengdie 走商业化路径，**Zep/Graphiti split 是 cleanest pattern** — 抽出 narrow OSS-leverage memory backend，commercial layer 不需要也 fork OSS framework；保留每层的 license + maintenance boundary 清晰。
- **intersection**: 远期参考（不直接竞争）。当前 v0.0.1 thesis 明确 reject 商业化（操作员决定 C：advisory until per-milestone PRD approves）。但 1+ 年如操作员 dogfood 触发 "是否 spin off commercial layer" 决策，Zep/Graphiti decompose pattern 是首选 reference (vs Letta-style 整体 OSS)。

## Trends (5)

- **T1: MCP 协议成事实标准**（97M monthly SDK downloads，5,800+ servers，9,400+ registry，67% CTOs adoption 12mo）。所有主流 AI tool / IDE / agent framework 已或即将支持 MCP。**对 mengdie**：v0.0.1 选 MCP server 是正确赌注，与行业完全对齐。

- **T2: 编码 agent 趋同到 "Claude Code baseline"**（Aider 44k / Continue 33k / Cline 61.4k+5M installs / OpenHands 70k）。功能集趋同：terminal + IDE 双形态、MCP 集成、test-driven iteration、自动 git commit。差异化窗口在 **特化垂直场景**（Aider 走 terminal-pure；Continue 走 IDE-deep；Cline 走 plan/act 双模式；OpenHands 走 SWE-bench 评测）。

- **T3: OSS memory 框架进入 "三足鼎立"**（mem0 53.5k / Letta 22.4k / Graphiti 20k）。三家技术路径分化：mem0 走 entity-linking、Letta 走 stateful runtime、Graphiti 走 bi-temporal knowledge graph。**对 mengdie**：v0.0.1 narrow scope（FTS5 + sqlite-vec）不重叠任一家技术路径，差异点是 **AE-pipeline 紧耦合 + 中文 + local-first**。

- **T4: cloud-vs-local 分化加深**（mem0/Letta 都加了 managed cloud；Graphiti 偏 self-hosted；Aider/Cline 都强调 local-first）。MCP 协议本身是 local-first 友好（stdio transport），但商业化 OSS memory framework 都在向 cloud 倾斜寻求收入。**对 mengdie**：v0.0.1 thesis "本地优先 / claude-CLI inherit credentials / 无外部遥测" 是明确 anti-cloud 立场，与 Aider / Cline 同阵营。

- **T5: knowledge graph 复兴 + bi-temporal validity 成新 baseline**（Graphiti、Letta Context Repositories、mem0 v2.0 entity linking）。"事实有效区间" 概念被三家独立采纳。**对 mengdie**：contradiction.rs 已有 valid_from / valid_until，但 Graphiti 的 event_time + ingest_time 双时间是 superior 设计，应 file 为 follow-up BL（不在 v0.0.1 范围）。

## Roadmap implications (3)

- **I1: v0.0.1 narrow OSS-adoption thesis 验证通过**。Survey 三家 memory framework 都已 MCP-aware（mem0-mcp / Letta MCP / graphiti/mcp_server），但 mengdie 的差异点（AE-pipeline 紧耦合 + 中文 FTS5 trigram + local-first 无遥测 + Karpathy 1-人定位）站得住脚。**继续按 narrow OSS-adoption thesis 走**：sqlite-vec (BL-026) + rig::Extractor (BL-027) + 自建 SQLite/FTS5/fastembed 主干，是合理结构。

- **I2: 多 host 支持是低成本扩展**（v0.0.1 不做，filed 为 vNext 候选）。MCP 是真标准 — Continue / Cline / Cursor 都已是 MCP host，mengdie 接它们零代码改动（同 stdio MCP 协议）。**roadmap 应在 vNext 段落显式记录**："Continue / Cline 接入是 testing 任务（验证 mengdie MCP description 在不同 host 下正常工作），不是开发任务。"

- **I3: bi-temporal validity 升级到 v1.0** 候选（不在 v0.0.1）。Graphiti 的 event_time + ingest_time 双时间模型 superior 于 mengdie 当前 valid_from/valid_until 单时间。但是 v0.0.1 的 contradiction.rs 已 work（13-14 syntheses 实测），**Karpathy "don't fix what's not broken"** 适用。**filed 为 follow-up BL**（survey 末段 + roadmap.md vNext 候选段）：当 contradiction 检测出现 false-positive（操作员实测） 时升级 schema。

