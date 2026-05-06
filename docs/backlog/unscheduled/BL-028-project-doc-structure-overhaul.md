---
id: BL-028
title: "Project-level doc structure overhaul — vision / product roadmap / per-milestone PRDs / per-feature specs (top-down stack to replace bottom-up累加)"
status: open
created: 2026-05-06
origin: "operator self-diagnosis 2026-05-06 night: '我觉得mengdie现在缺的是一个蓝图跟线路图，我们在乱堆积木 [...] 应该不仅仅是针对v0.0.1的，应该是对整个mengdie项目的, 我现在做这个项目完全没有头绪'"
trigger: "Strategic blocker — must complete before v0.0.1 implementation work resumes (BL-026/BL-027 spikes / BL-008/BL-025 AE plugin wiring all paused pending vision + product roadmap drafts)"
depends_on: []
size: L
v_target: "Pre-v0.0.1 strategic — gates v0.0.1 sprint resumption"
---

# BL-028 — Project-level doc structure overhaul

## Origin

Operator self-diagnosis 2026-05-06 evening, after a long session of v0.0.1 thesis revision (修补 → 全部换轮子 → narrow OSS-replacement) revealed that thesis was 易摇摆 because no top-down vision exists to anchor it:

> 我觉得mengdie现在缺的是一个蓝图跟线路图，我们在乱堆积木 [...]
> 应该不仅仅是针对v0.0.1的，应该是对整个mengdie项目的，最起码连带接下来的几个版本，
> 我现在做这个项目完全没有头绪 [...]
> 以及每个功能的spec，每个milestone的prd [...]
> 这个东西要先记录下来，而且我们还需要有个行业现状的调查，最起码open source 圈的情况

## Problem

mengdie has been **bottom-up**: discussions → BLs → sprints → ad-hoc累加. No top-down vision driving. Symptoms:

- Every decision looks reasonable in v0.0.1 myopic view; doesn't aggregate to coherent project direction
- Thesis 易摇摆 (one-day swing of v0.0.1 scope from "rip-and-replace" to "narrow swap"); without clear vision, scope can't anchor
- "完全没头绪" sensation; "乱堆积木" feel
- `blueprint.md` v0.2 标榜 long-lived but is identity-only, no trajectory; reading it doesn't tell you where mengdie goes 1-2 年后
- All current docs (blueprint / 026 OSS survey / 027 industry state / `.ae/roadmaps/v0.0.1.md` / CLAUDE.md Project Status) are **v0.0.1-myopic**

## Expected outcome

A **top-down doc stack** spanning the whole mengdie project (not just v0.0.1):

```
0. Industry survey (broader OSS coding-tooling circle; vision-driving, NOT v0.0.1-driving)
   ↓
1. Vision (mengdie 1-2 年后长什么样; 1-2 pages)
   ↓
2. Product roadmap (v0.0.1 → vNext → ... → v1.0; one-liner per milestone + dependency graph)
   ↓
3. Per-milestone PRDs (each milestone's product spec)
   ↓
4. Per-feature specs (cross-milestone evolution; surface contracts for memory_search / memory_ingest / mengdie CLI / etc.)
   ↓
5. BLs serve specs serve PRDs serve roadmap serve vision
```

**Acceptance test for the new structure**: any BL / decision / phase must be answerable on "this serves which vision element via which milestone via which spec criteria?" Items that can't answer get deprioritized or dropped.

## Why "应该不仅仅是针对v0.0.1"

v0.0.1 is stepping-stone or dead-end depending on what 1-2 年后 is wanted. Without vision, can't tell. The doc rewrite must start at the **project trajectory** level, not the version level.

## Why "行业现状调查" comes BEFORE vision

Vision needs to be **grounded in reality** + **inspired by what's possible**. Existing surveys insufficient for vision-driving:

- 027 industry state 2026: consumer PKM AI tools / OSS memory frameworks — v0.0.1-driving, doesn't survey OSS coding-tool ecosystem at depth
- 026 OSS Rust survey: 14 specific Rust libraries — adoption-driving, doesn't survey larger MCP / OSS coding agent ecosystem evolution

**Missing for vision-driving**:
- OSS coding agent ecosystem (Aider / Cursor open / Continue / Cline / Claude Code / OpenHands / etc.) maturity + evolution
- MCP ecosystem 1-2 年趋势 (97M installs as of March 2026 per 027; how is the toolset evolving?)
- OSS memory / AI-tooling 1-2 年趋势 (who's accelerating / dying / where the gaps are)
- OSS coding tool product 案例 (success vs failure patterns)

This survey is bigger than 027 + 026 combined. Operator estimate: 4-6 hours via WebSearch / WebFetch / GitHub trends reading.

## 5 vision questions (pre-condition for vision draft; analysis.md will enumerate)

Operator must answer at least roughly before vision draft begins:

1. mengdie 自己用 1 年后想要什么具体日常改变?
2. mengdie 给别人用 是否在 vision 里? 1 年后? 永远不?
3. mengdie 跟其他 AI tool (Cursor / Continue / Aider / Cline / OpenHands) 的关系?
4. mengdie 跟 host LLM (Claude / OpenAI / Gemini) 的关系? vendor-neutral / claude-first / vendor-specific 允许?
5. 不再写 mengdie 时希望它 useful by others? (决定 doc / 测试 / 移交性 priority)

## Estimated work (calendar 2-3 days; operator-time ~1-2 days)

| 步骤 | 输出 | Drafter | 估时 |
|---|---|---|---|
| 0 | `docs/surveys/2026-05-oss-coding-tooling.md` (broader OSS survey) | I draft via WebSearch + reading | 4-6 h |
| 1 | `docs/vision.md` (1-2 pages) | I draft after operator answers 5 Qs; operator critiques | 2-3h draft + 1h critique |
| 2 | `docs/product-roadmap.md` (v0.0.1 → v1.0 narrative) | I倒推 from vision; operator confirms | 1-2 h |
| 3 | v0.0.1 重新审视 against new roadmap | Operator | 30 min |
| 4 | `docs/milestones/v0.0.1.prd.md` | I draft | 1-2 h |
| 5 | `docs/v0.0.1-ship-flow.md` (operator-experience narrative) | I draft | 2-3 h |
| 6 | `docs/specs/{memory_search, memory_ingest, mengdie-cli}.md` (4-5 critical specs) | I draft | 3-4 h |

## Sequencing (strict, blocks resumption of v0.0.1 implementation work)

**Do NOT** start v0.0.1 implementation work (BL-026/BL-027 spikes / AE plugin wiring) before steps 0+1+2 are at least drafted. Otherwise more bottom-up累加.

## Reversibility

**Medium**. The artifacts produced (vision / roadmap / PRDs / specs) are project-level docs that will live for the entire mengdie lifecycle. If a year from now the vision needs revision (e.g., operator's project priorities change), the docs evolve in place. The cost of writing them is the operator time + I time noted above; the cost of NOT writing them is recurring "completely no direction" sessions like 2026-05-06.

## Trigger

**Fires immediately** — strategic blocker before any further v0.0.1 implementation work. Once promoted to feature (F-NNN), `/ae:analyze` should produce a fuller analysis (incorporating the broader OSS survey as input), then `/ae:plan` produces step-by-step plan.
