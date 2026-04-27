---
id: gemini-proxy
round: 1
type: product-ux
reviewed_by: self-analysis
date: 2026-04-28
---

# Round 1: Product/UX Analysis — BL-009 MCP Dream Tool

## Findings

### F1: CLI Path Breaks Conversation Context
**Evidence**: `src/core/dreaming.rs:399` `run_synthesis_pass` calls `provider.synthesize()` synchronously; `ClaudeCliProvider` (BL-005) spawns subprocess. Subprocess is fresh process, inherits no conversation state.

**UX impact**: Kai in a Claude session discussing memory themes → synthesis fires → subprocess doesn't know about that conversation → synthesized memories ignore the context Kai was just discussing. Claude in-session would synthesize *relative to that conversation*.

**Why it matters for Kai**: Personal system — he's the primary user. If synthesis ignores conversation context, the generated memory is out of phase with what he's thinking about at that moment.

### F2: Blocking Async Breaks Flow
**Evidence**: `mengdie dream --synthesize` is CLI call that blocks until all clusters finish synthesis (see loop at `dreaming.rs:423`). No streaming, no partial results.

**UX impact**: Kai runs `mengdie dream --synthesize` in a session → CLI hangs → conversation stalls → synthesis completes → JSON logged to stdout → Kai has to parse it manually. Hard context switch.

**In-session comparison**: MCP tool returns during same turn. Kai's Claude continues the conversation; synthesis appears as a tool result inline. No context loss, no manual parsing.

### F3: Synthesis Failures Are Invisible
**Evidence**: Current LlmProvider trait (BL-005) returns `Result<String>`. Failure is logged to stderr (never reaches Kai in-session), success is JSON. No granular per-cluster feedback.

**UX impact**: 
- Cluster 1 synthesized, Cluster 2 LLM refusal (quota? content policy?), Cluster 3 failed due to malformed JSON → Kai gets one error result. Did 1/3 succeed? Which one? Why did 2 fail?
- MCP path: tool returns structured { synthesized: [...], failed: [{cluster_id, reason}, ...] }. Kai sees exactly what happened.

### F4: Residuals Silently Skipped
**Evidence**: `dreaming.rs:416-421` logs residuals count to tracing (stderr). Conversation doesn't see it.

**UX impact**: Kai runs synthesis, gets back 5 synthesis results. But if there were 8 clusters and 3 were residuals → he doesn't know he left memories unsynthesized. Visibility is lost.

**MCP path**: tool returns residuals count + ids. Kai can ask Claude "why weren't these synthesized?" → CLI tool can explain selection criteria.

### F5: CLI Is Convenient from Shell, Not from Session
**Evidence**: User memory (user_profile.md) — "prefers direct action." From shell, `mengdie dream --synthesize` is one command. From Claude session, requires: (a) create synthesis memory manually (or (b) shell out (`! mengdie dream`), wait for it, copy-paste JSON back.

**UX impact**: CLI path is easier from shell, harder from session. MCP path is one tool call from session. The default path depends on WHERE Kai triggers synthesis.

**Hypothesis**: if mengdie is loaded as MCP server in Claude settings, Kai will use `memory_dream` tool by default (lower friction in session). If mengdie is not loaded in that session, CLI is the fallback. This is healthy coexistence, not competition.

### F6: "Shared Session Context" Is Real if Synthesis Needs Explanation
**Evidence**: Framing question Q1 asks "is shared session context productive?" The honest answer: *it depends on what the synthesis is about*.

Scenario A (productive context):
- Kai: "we should consolidate these API design discussions"
- [memory_dream tool returns 4 clusters]
- Kai: "why did it group X with Y?"
- Claude (in-session): can refer back to the conversation context, explain clustering

Scenario B (distraction):
- Kai: working on unrelated bug fix
- Synthesis fires as background job
- Claude interrupts with "I synthesized 3 memory clusters!" → Kai has to shift mental context
- This is exactly what BL-010 (daemon) is designed to avoid

**Implication**: MCP tool (BL-009) is best used when Kai *explicitly triggers* it (not auto-fire). Daemon (BL-010) enables true background synthesis without conversation interruption.

---

## Agreements

(None yet — awaiting other reviewers)

---

## Disagreements

(None yet — awaiting other reviewers)

---

## Open Questions

### Q1: Is the CLI Path Actually Painful, or "Just Works"?
The framing's Q1 asks "whether this indirection is worth eliminating." From pure UX:
- **CLI is easier from shell** (one command)
- **MCP is easier from session** (one tool call)
- **Coexistence is healthy** — no need to eliminate CLI

**Decision pressure**: if Kai prefers to trigger synthesis from session (asking Claude "synthesize my current memory clusters"), then MCP is a UX win. If he prefers shell, then CLI is fine. The honest test is: after BL-009 ships, does Kai actually *use* the tool, or does he keep using `! mengdie dream`?

**Recommendation**: Ship BL-009 as default path in-session. Keep CLI as fallback. Let usage patterns tell us if it was worth the ~150 LOC.

### Q2: Does BL-009 Enable or Just Witness the Phase 2 Chain?
The framing says "don't foreclose BL-010." But:
- BL-009 proves Claude in-session can be an LlmProvider (validates BL-005 extensibility)
- BL-009 surfaces UX patterns (visibility, feedback, context reuse) that daemon needs to inherit
- BL-010 (daemon) can then optimize those patterns (async queue, error recovery, retry policy)

**So BL-009 is not just a stepping stone; it's an experiment.** Ship it, observe Kai's usage, then BL-010 builds on the learned patterns.

### Q3: What Happens When Synthesis Partially Fails?
E.g., cluster 1 succeeds, cluster 2 fails (LLM refusal), cluster 3 times out.

**CLI path**: logs all 3 outcomes to stderr, returns error. Kai has to decide: retry? ignore? retry just cluster 2?

**MCP path options**:
- (A) Fail the whole tool (all-or-nothing)
- (B) Return partial results { synthesized: [1], failed: [2, 3] } and Kai decides in-conversation
- (C) Auto-retry with exponential backoff (slow, blocks conversation)

**Recommendation**: (B) — return structured feedback. Kai and Claude can decide together whether to retry or move on. This is the "transparent" UX win.

### Q4: Should Memory_Dream Be Its Own Tool, or Extend Memory_Ingest?
Framing references discussion 008 precedent: "extend existing tools rather than add new ones."

**Arguments for new tool**:
- `memory_dream` semantics are different (read clusters → synthesize → ingest). Not a natural extension of `memory_ingest` (write a memory).
- Clearer API surface: `memory_dream` is "trigger synthesis," `memory_ingest` is "write result."

**Arguments for extension**:
- Reduces tool count (already 3; keep it at 3).
- Could be `memory_ingest` with `source_type: "synthesis"` and `synthesize: true` param. Mengdie synthesizes, then ingest returns the result.

**Recommendation**: **New tool.** The semantics are genuinely different. Discussion 008 was conflict resolution; BL-009 is validation of a new capability. First-caller pattern (plan 010 precedent) suggests: introduce the new tool, let it prove value, then consider consolidation in a future refactor.

---

## UX-Centered Answers to Framing Questions

### "Whether This Indirection Is Worth Eliminating"

**Answer**: YES, but *in the MCP-attached case only*. Kai is primary user (personal system). His win is:
1. **Conversation context** — Claude's synthesis is informed by what he's discussing
2. **Visibility** — he sees exactly which clusters synthesized, which failed, why
3. **Flow** — no context switch to shell, synthesis result appears inline
4. **Control** — he can ask follow-up questions about clusters in the same conversation

**The indirection is worth eliminating for Kai's UX. CLI path remains fine for batch synthesis (e.g., `! mengdie dream` when he's not in an active conversation).**

### "What Must Mengdie Enforce vs. Delegate"

**Mengdie enforces**:
- Cluster selection (based on threshold + min_size)
- Synthesis cluster hash + memory_synthesis_links (per v0.8.5 invariant)
- Residuals counting (for transparency)

**Mengdie delegates to LLM**:
- Synthesis text generation
- Explanation of cluster themes (useful in-session)

**MCP tool shape**: returns `{ clusters_to_synthesize: [...], residuals_skipped: [...] }`, Claude synthesizes and calls `memory_ingest`. Simple, clear boundary.

---

## Honest Assessment

**Would Kai actually USE the memory_dream tool?**

YES, if:
- (1) It's the default path when mengdie is loaded in Claude
- (2) He can see synthesis outcomes in-conversation (structured feedback)
- (3) It doesn't interrupt unrelated work (e.g., doesn't auto-fire while he's debugging)

**The CLI path will NOT go away**, but MCP tool becomes the primary flow for "I want to synthesize my memory clusters while thinking about them" scenarios.

This is healthy. BL-009 validates that Kai's workflow benefits from in-session synthesis. BL-010 then builds on that validation to make background synthesis work without interruption.

