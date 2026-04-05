---
id: "004"
title: "MVP Assessment — Limitations, Feasibility Gaps, and Path Forward"
type: analysis
created: 2026-04-05
status: draft
---

# MVP Assessment — Limitations, Feasibility Gaps, and Path Forward

## 1. MVP 做了什么、局限在哪

### 已实现

| 组件 | 做了什么 | 工作方式 |
|------|---------|---------|
| MCP server (stdio) | 3 个 tool：search / ingest / invalidate | Claude Code 按需启动，AE subagent 自动继承 |
| Hybrid search | FTS5 + vector (all-MiniLM-L6-v2, 384d) + RRF (k=60) | 双路搜索，rank 级融合，score 归一化到 0-1 |
| Embedding | fastembed 本地推理，metadata-in-chunk 编码 | `[type] [entities] [project] Title --- content` 前缀着色 |
| 去重 | `UNIQUE(project_id, source_file)` 索引 | 同 key 走 UPDATE |
| 矛盾检测 | entity overlap + cosine > 阈值 | ingest 时检查，返回 conflict list |
| Dreaming | recall_count >= 3, avg_relevance >= 0.65, 14天窗口 | 被搜到就记 recall，达标就 promote 为 long-term |
| CLI | dream / import --dir / search / stats | 批量导入 + 手动操作 |
| ae:analyze 读取 | SKILL.md Step 3.5 memory_search | 研究完成后、综合前注入先验上下文 |

### 局限

#### L1: 没有自动写入——循环没闭合

整个知识螺旋的前提是"AI 产出知识 → 自动进入 mengdie → 下次 AI 自动读取"。现在：

- **写入端完全断开**：没有 hook、没有 watcher daemon 在自动调 `memory_ingest`
- **读取端只有一处**：仅 `ae:analyze` 有 Step 3.5
- 结果：DB 是空的（除非手动 `mengdie import`），Dreaming 没有 recall 数据，整个系统处于冷启动且无法自热的状态

#### L2: Ingest 是"存原文"，不是"消化知识"

`memory_ingest` 接受 title + content + 元数据，原样存入 SQLite。没有：

- **知识提取**：一篇 3 页 conclusion.md 存成一整条。搜索返回 200 字 snippet，丢失结构
- **拆分**：一个 conclusion 里可能有 3 个独立决策，全部混在一条记忆里
- **摘要**：content 就是原文，没有精炼
- **self-contained 化**：搜索结果需要回到原文才能完全理解

调用方（AE skill 或人）需要自己提取、精炼、拆分后再调 ingest。**智能全在调用侧，mengdie 只是 storage。**

#### L3: `source_file` 作为去重 key 脆弱

- 文件改名/移动 → 产生重复条目
- MCP tool 调用时 `source_file` 是任意字符串 → 无法防止调用方传不同路径导致重复
- 没有内容级去重（content hash 或 semantic dedup）
- `source_file` 作为溯源路径也不可靠：文件挪了就是死链

#### L4: 没有"已 ingest 列表"或 dry-run

- `mengdie stats` 只给总数，不知道哪些文件/条目已导入
- `mengdie import` 没有 `--dry-run`——无法预览"这次会导入什么、跳过什么"
- 手动操作完全盲目

#### L5: 搜索结果是原始文本片段

`SearchResultItem.snippet` = content 前 200 字符。没有：

- 高亮匹配词
- 根据 query 选取最相关段落
- 结构化输出（决策 + 理由 + 实体，而不是一坨文本）

对 AI agent 来说，原始 snippet 需要额外 token 才能理解和利用。

#### L6: Dreaming 无法验证

Dreaming 依赖 recall 数据。但 recall 只在 `memory_search` 时记录。当前几乎没有 search 调用（只有 ae:analyze 偶尔触发），所以：

- recall_count 全部为 0
- avg_relevance 无数据
- Dreaming 跑了也不会 promote 任何东西
- 无法验证"行为驱动过滤"这个核心假设

#### L7: 矛盾检测粒度粗

- 只看 entity tag overlap + cosine similarity
- 同一 entity 下可能有几十条记忆，全部两两比较但阈值固定
- 没有 temporal reasoning（不知道"这个决策 3 个月前做的，现在可能过时了"）
- conflict 返回后无后续动作——没有 UI、没有 workflow 来处理

---

## 2. MVP 中必须一起验证可行性的

这些不是"nice to have"，而是：**如果验不通，后续方向要改。**

### V1: 知识循环能不能真的闭合

**核心假设**：AI 产出的知识自动进入 mengdie → 下次 AI 自动读取 → 输出质量提升。

**目前状态**：循环断开，无法验证。

**必须验证**：
- 手动闭合一次完整循环：import 一批已有的 AE discussion → 跑 ae:analyze 看它能不能搜到并利用 → 对比有/无 mengdie 的 ae:analyze 输出质量
- 这不需要写代码，用现有的 `mengdie import` + `ae:analyze` 就能做
- **如果 AI agent 搜到了先验知识但不知道怎么用（或者用了反而干扰），整个产品假设需要重新评估**

### V2: Dreaming 的"行为驱动"在 AI-agent 场景下是否成立

**核心假设**：被频繁搜到的知识 = 有价值的知识。

**风险**：
- AI agent 的搜索行为和人类不同。ae:analyze 每次搜一次，query 由 AI 生成，可能高度重复
- 如果所有 ae:analyze 都用相同模板生成 query，recall 数据是虚假信号——不是"这条知识有用"，而是"query 模板恰好匹配这条"
- Dreaming 的 avg_relevance 来自 cosine score，但 cosine score 衡量的是"query 和 content 的语义距离"，不是"这条知识对任务的实际帮助"

**必须验证**：
- 观察 10+ 次 ae:analyze 搜索的 query 多样性
- 如果 query 高度同质，recall_count 是噪声，Dreaming 模型需要换信号源（比如 AI 是否真的在输出中引用了搜索结果）

### V3: 搜索质量在真实数据上是否足够

**已有**：unit test 用合成数据验证了 FTS + vector + RRF 的正确性。

**未验证**：
- 真实 AE 文档（conclusion.md 通常 500-2000 字，含 YAML frontmatter + 多个 section）的 embedding 质量
- 当记忆数量达到 50-100 条时，top-5 的相关性
- metadata-in-chunk 编码是否真的改善了检索（对比有/无前缀的检索效果）

**必须验证**：
- 用现有 AE 讨论做 import → 跑一批 query → 人工判断 top-5 相关性
- 这个直接能做，不需要新代码

---

## 3. 可以一起纳入 MVP 的增强

这些改动小但直接解决上面的局限，且不改变架构。

### E1: `mengdie list` 命令

```
mengdie list                     # 列出当前项目所有记忆（title, source_file, recall_count, is_longterm）
mengdie list --project-id xxx    # 指定项目
mengdie list --format json       # 给脚本用
```

解决 L4"不知道 ingest 了什么"。~50 行 Rust。

### E2: `mengdie import --dry-run`

```
mengdie import --dir ./docs/discussions/ --dry-run
# Would import:
#   docs/discussions/002-mvp-phase1/conclusion.md (new)
#   docs/discussions/003-tech-stack/conclusion.md (new)
# Would skip (already imported):
#   docs/discussions/001-product-vision/product-vision.md
```

解决 L4"盲目导入"。~20 行改动。

### E3: Content hash 去重（替代 source_file 唯一索引）

schema 加 `content_hash TEXT`（SHA-256 of normalized content），唯一索引改为 `(project_id, content_hash)`。

- 文件移动不再产生重复
- MCP ingest 不依赖 source_file 路径一致性
- source_file 降级为纯粹的溯源注释（可以过时，不影响逻辑）

~30 行 schema + db.rs 改动。**这个应该在 MVP 里做，因为后续数据都依赖去重 key。迁移越晚越痛。**

### E4: Ingest 时自动生成摘要 snippet

在 ingest 时，取 content 前 500 字符作为 `snippet` 字段存储（不依赖 AI，纯截断）。search 返回 snippet 而不是实时截断。

为什么现在做：搜索结果的可读性直接影响 AI agent 能否有效利用。当前 200 字符 snippet 在 search 时临时生成，不可配置。存储时生成可以后续升级为 AI 摘要。

~10 行改动。

### E5: `source_file` 改为 optional

MCP ingest 的 `source_file` 从 required 改为 optional。原因：

- 来自 AE skill 自动写入的知识不一定有文件路径（skill 在内存中提取知识点直接调 ingest）
- 来自 Claude Code session 的知识没有文件
- Phase 2 的 GitHub/Slack 来源更不可能有本地文件路径

source_file 有值时存，无值时存空字符串或 null。去重改用 content_hash 后，这个字段不再承担逻辑角色。

~5 行改动。

---

## 4. 后续执行计划

### Phase 1.1: 验证 + 快速修补（1-2 天）

**目标**：用现有代码验证核心假设，同时修复最影响可用性的问题。

并行做两件事：

**验证轨道**（不写代码）：
1. `mengdie import --dir` 导入现有 AE 讨论（至少 10 个 conclusion.md）
2. 跑 5-10 个 `mengdie search` 查询，人工评判 top-5 相关性
3. 跑一次 `ae:analyze` 在有 mengdie 数据的项目上，看注入效果
4. 记录：query 质量、结果相关性、AI 是否有效利用注入内容

**修补轨道**（小改动）：
1. E1: `mengdie list`
2. E2: `mengdie import --dry-run`
3. E3: Content hash 去重
4. E5: `source_file` optional

### Phase 1.2: 闭合写入循环（3-5 天）

**前提**：Phase 1.1 验证通过（search 质量 OK，AI 能利用注入内容）。

1. AE PRD (`mengdie-integration.md`) 中的写入集成——优先 `ae:analyze` 和 `ae:discuss`
   - ae:analyze 完成后提取 2-3 条 key findings，自动调 `memory_ingest`
   - ae:discuss 解决决策后，提取 resolved decision，自动调 `memory_ingest`
2. 验证 Dreaming 在真实 recall 数据下的行为
3. 观察矛盾检测在真实数据上的 false positive rate

### Phase 1.3: 知识消化（探索性，1-2 周）

**前提**：Phase 1.2 证明循环能闭合。

这是从"存储层"到"知识层"的跨越，需要设计讨论：

- **提取粒度**：一个 conclusion.md 应该变成几条记忆？谁来决定拆分？
  - 选项 A：调用侧（AE skill）负责拆分 → mengdie 保持简单
  - 选项 B：mengdie 自己用规则拆分（按 heading、按 decision table row）
  - 选项 C：mengdie 用 LLM 提取知识点 → 引入 AI 判断，与"No AI judgment"原则冲突
- **结构化输出**：search 返回的不是 snippet 而是 `{decision, rationale, entities, confidence}`
- **关联发现**：共现追踪（同一 search session 返回的 A 和 B 建立关联）

这些需要一轮 `/ae:discuss` 来对齐方向。

### Phase 2: 更多来源 + 关联（Phase 1 全部验证通过后）

按产品愿景的 Phase 2 推进：
- **Watcher daemon**：`mengdie watch --daemon` 后台监听 AE 输出目录，文件变更自动 ingest（watcher.rs 已有 notify 库代码，需要接入 CLI + daemon 管理）。参考 qmd 的 `qmd mcp --http --daemon` 模式（fork + PID file + stop 命令），或直接用 macOS launchd plist。
- Claude Code SessionEnd hook（operational memory）
- ae:review / ae:retrospect 写入集成
- Semantic similarity 关联
- Co-occurrence 关联
- Decay / archival

---

## 5. 判断标准

### Phase 1.1 通过标准
- [ ] 导入 10+ conclusion.md 后，`mengdie search` top-3 有至少 1 条相关结果的比率 > 70%
- [ ] ae:analyze 注入的先验上下文被 AI 在综合阶段实际引用（不是被忽略）
- [ ] content hash 去重在文件移动场景下正确工作

### Phase 1.2 通过标准
- [ ] ae:analyze 完成后自动写入 mengdie，下一次 ae:analyze 能搜到
- [ ] Dreaming 在 20+ 次真实 search 后，至少 promote 1 条记忆
- [ ] 矛盾检测 false positive rate < 30%

### Phase 1.3 启动条件
- Phase 1.2 通过
- 有明确的"当前 snippet 不够用"的证据（AI agent 搜到了但无法利用）
