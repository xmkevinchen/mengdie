---
agent: doodlestein-strategic
attempt: 1
verdict: REVISE
timestamp: 2026-04-27T19:58:02Z
---

# doodlestein-strategic attempt 1: REVISE — re-organize around invariant boundary

**Foreclosed alternative**: "Claude does NOT do synthesis in-session at all — the tool surfaces data and triggers a queued job; synthesis happens via ClaudeCliProvider in the background." Framing rules this out implicitly by labeling subprocess path "structurally wrong." For dream pass triggered from within a Claude session, queue/defer model may be strictly better — avoids holding interactive session on slow synthesis pass over many clusters; reuses already-correct ClaudeCliProvider invariant enforcement.

**MCP as delivery vehicle**: framing takes MCP tool(s) as the answer before the question is fully posed. Alternative not considered: a `synthesis_session` stateful read/accumulate/commit flow. Cluster-hash invariant enforcement is fundamentally stateful — you need all source memory IDs before writing the synthesis. Tool-per-cluster-with-final-commit is different from single-round-trip-tool; framing doesn't surface this choice.

**Smarter problem cut**: current framing asks "how should mengdie expose synthesis to Claude?" — sharper cut is **"what should mengdie enforce regardless of who runs synthesis, and what can it safely delegate?"** Reframes from mechanism to invariant boundary:
- Cluster selection and sourcing: mengdie must own (it has the corpus)
- Synthesis text generation: can be delegated (Claude in-session OR ClaudeCliProvider subprocess OR future LlmProvider impls)
- Persistence with cluster-hash + source links: mengdie must own (invariant)

This reframe immediately surfaces a "separate path" alternative: Claude is just a text generator; mengdie fetches clusters, calls any LlmProvider (including a new McpSessionProvider that asks the *current* Claude via reverse tool call), and persists. Strictly compatible with existing LlmProvider trait — new impl, not new architecture.

**Concrete recommendation**: revise problem statement to lead with invariant boundary question; include "add McpSessionProvider impl to LlmProvider" path alongside "new MCP tools" path as explicit alternatives. The current framing's scope-out of "multi-model / multi-LLM support" is correct, but McpSessionProvider impl is NOT multi-model — it's a new LlmProvider impl for the existing trait, in-scope as alternative mechanism.
