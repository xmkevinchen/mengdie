---
id: "027"
title: "v0.0.1 Step 0 — industry state of personal AI memory 2026"
status: active
created: 2026-04-27
pipeline:
  analyze: done
  discuss: done
  plan: pending
  work: pending
plan: ""
parent_plan: "docs/v0.0.1-rebuild-plan.md"
tags: [v0.0.1, industry-survey, state-of-the-art, personal-ai-memory, agent-memory, reflection, self-improving]
---

# v0.0.1 Step 0 — industry state of personal AI memory 2026

What does the 2026 personal AI memory landscape actually look like?
Before mengdie defines its blueprint and audits its v0.x code, it
needs a map of what other people have already built — commercial
products, open-source memory layers, academic research, integrations
with coding tools.

Step 0 of the v0.0.1 redesign. Inserted ahead of Step A re-do at the
operator's request: the prior Step A (025/026) implicitly assumed
"minimum-change to v0.x" framing; Step 0's industry survey is the
prerequisite for re-framing under "OSS-by-default + clear product
definition."

## What this is NOT
- Not a product roadmap (that's the blueprint, written after this)
- Not a re-survey of Rust libraries (already covered by 026)
- Not a feature-by-feature comparison (that comes during blueprint drafting)

## What this IS
A landscape map covering:
- Commercial PKM AI tools (NotebookLM, Mem.ai, Reflect, Heptabase AI, Obsidian Smart Connections, Recall, etc.)
- Open-source memory layers (mem0, Letta/MemGPT, LangMem, LlamaIndex Memory, Cognee, Zep, Graphiti, etc.)
- Academic research 2024–2026 on agent memory, self-improving systems, reflection
- Memory integrations with coding tools (Cursor / Continue / Aider / Cline / Claude Code MCP memory servers)
- OpenAI ecosystem reality (Files / Vector Stores / Memory features)
- Google ecosystem reality (NotebookLM internals, Gemini Files API)
- Architectural patterns surfaced across all of the above (hot/cold storage, episodic vs semantic, temporal validity, provenance, reflection triggers)

The /ae:analyze phase is complete (`analysis.md` + `docs/blueprint.md`
v0.2). This /ae:discuss phase resolves the five architectural open
questions documented in blueprint §8 — these gate P1 / P2 BL filing
under the v0.0.1 rebuild.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Ingest mechanism — delivery pattern | [topic-01-ingest-mechanism/](topic-01-ingest-mechanism/) | converged | Push-primary; watcher.rs kept as opt-in library, NOT wired |
| 2 | Reflection trigger model | [topic-02-reflection-trigger/](topic-02-reflection-trigger/) | converged | On-demand default + `ReflectionTrigger` trait; cron as second impl |
| 3 | Cross-project default retrieval scope | [topic-03-cross-project-scope/](topic-03-cross-project-scope/) | converged | Ratify §5 per-project; rationale = contamination risk (refined) |
| 4 | Ingest source boundary | [topic-04-ingest-source-boundary/](topic-04-ingest-source-boundary/) | converged | Extraction-discipline (not physical AE-files-only); `source_type::unknown`→`direct` |
| 5 | Loop-closure signal | [topic-05-loop-closure-measurement/](topic-05-loop-closure-measurement/) | converged | F-002 nonempty rate + ae:retrospect + falsification rule |

## Documents
- [Framing](framing.md)
- [Analysis](analysis.md)
- [Conclusion](conclusion.md) *(after discussion complete)*
