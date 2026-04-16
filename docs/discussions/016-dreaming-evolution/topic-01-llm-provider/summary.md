---
id: "01"
title: "LLM Provider Architecture"
status: pending
current_round: 1
created: 2026-04-16
decision: ""
rationale: ""
reversibility: ""
---

# Topic: LLM Provider Architecture

## Current Status
No LLM access in mengdie. All intelligence depends on this decision.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
mengdie needs LLM for: entity extraction (ingest), RAG synthesis (search), knowledge compilation (dreaming), semantic lint (dreaming). This is the foundation layer — everything else blocks on it. Solo developer, so LLM does the heavy lifting that can't be hand-coded.

Prior art: OpenClaw has no built-in LLM but calls Claude/OpenAI/Gemini APIs. domleca/llm-wiki uses Ollama locally. Engram has no LLM. Mem0 uses configurable providers.

## Constraints
- mengdie is a Rust binary (MCP server + CLI daemon)
- Must work offline for basic operations (search, ingest without extraction)
- API costs matter for a solo dev — can't call LLM on every search
- Multiple providers needed (Claude, OpenAI, Gemini) for resilience and cost control
- Dreaming runs as scheduled daemon (launchd), not inside MCP session

## Key Questions
- How should mengdie call LLMs? Direct HTTP API? Rust SDK? Via MCP as client?
- Should the provider be configurable per-operation (cheap model for extraction, expensive for synthesis)?
- How to handle the MCP server case (Claude is already the caller) vs daemon case (no active AI session)?
- What's the config model? Environment vars? Config file? Both?
