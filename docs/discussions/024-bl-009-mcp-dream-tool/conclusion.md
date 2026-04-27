---
id: "024"
title: "BL-009 MCP Dream Tool — Conclusion"
concluded: 2026-04-27
plan: ""
entities: [bl-009, mcp, dream, tool, mcp-dream-tool, mcp-session-provider, llm-provider, sampling, rmcp, memory-dream, claude-code-session, synthesis-loop, in-session]
---

# BL-009 MCP Dream Tool — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | BL-009 design — whether + what | **Build in v0.9.0** as: (a) **McpSessionProvider** — new `LlmProvider` impl in `src/core/llm.rs` using rmcp `Peer<RoleServer>::create_message()` to send `sampling/createMessage` to host Claude Code; (b) **post-handshake one-time capability check** (NOT at `MengdieServer::new` — the Peer handle doesn't exist before `rmcp::serve_server()` runs, per Doodlestein 3-way convergent finding). Implementation pattern: `OnceLock<Box<dyn LlmProvider>>` lazy-initialized at first `memory_dream` invocation, OR `on_initialized` callback override that captures the Peer and resolves the provider, OR two-phase server construction. Whichever pattern the plan picks, the property preserved is "single resolution per process; no per-call branching." Fallback target: **ClaudeCliProvider** (silent, no user-visible delta) when host doesn't advertise sampling; (c) **thin `memory_dream` tool wrapper** in `src/core/mcp_tools.rs` (~30 LOC) — exposes `memory_dream(project_id?, dry_run?)` triggering `run_synthesis_pass` with the resolved provider; (d) **`SynthesisResult` extended with per-cluster event log** — returns to in-session UX. **No changes** to `run_synthesis_pass`, `insert_synthesis_with_links`, persistence, schema, or `mengdie dream --synthesize` CLI path. **Cleanup**: remove `SourceType::Synthesis` from `memory_ingest` (broken post-v0.8.5 regardless of BL-009). Total: ~50-150 LOC + 1 thin tool. | TL verified rmcp v1.3 supports server-initiated sampling via `Peer<RoleServer>::create_message()` (context7 docs.rs/rmcp query). 4/5 strong convergence on McpSessionProvider after evidence-driven Round 2 movements: architect retracted Shape B preference (Round 1 was conditional on rmcp sampling absent); gemini conceded "new tool" + "prompt-level context reuse" claims (challenger F4 + minimal-change F4); minimal-change FLIPPED Round 1 defer→yes-ship (rmcp gating condition fired); codex confirmed Path C ≡ McpSessionProvider convergence. Challenger conceded all 4 Round 1 technical findings (F1 stub-broken irrelevant under McpSessionProvider; F4 UX-conversational distinction accepted; F6 cost calculus changes per TL verification; F7 not-foreclosing-BL-010 confirmed) but held defer-until-trigger on timing. TL judgment: silent fallback + bounded cost (50-150 LOC) + Phase 2 chain benefits outweigh defer-now-build-later tradeoff. McpSessionProvider satisfies discussion 008 "extend, don't add" precedent (0 new write paths). Pressure-tests LlmProvider trait extensibility per plan 010 first-caller pattern. Cluster-hash invariant transparently satisfied via run_synthesis_pass → insert_synthesis_with_links. BL-010 daemon orthogonal (uses ClaudeCliProvider). | high — provider pattern means impl swap is trivial; thin tool wrapper has zero state; storage path unchanged; ClaudeCliProvider remains canonical fallback. |

## Doodlestein Review

3 fresh agents reviewed the written conclusion. **All 3 independently surfaced the same finding** — strong 3-way convergence indicates a real structural defect in the conclusion's mechanism description. Decision direction (yes-build, McpSessionProvider, silent ClaudeCliProvider fallback) NOT challenged. Fix is mechanism-correction + plan-time directive; no reopen needed.

### Strategic + Adversarial + Regret (3-way convergent)

**Finding**: "Construction-time capability check at `MengdieServer::new`" is structurally impossible. Per `src/bin/mcp_server.rs:37-39`, `MengdieServer::new` runs in `main()` BEFORE `rmcp::serve_server()`. The MCP `initialize` handshake — where the client sends `ClientCapabilities` (including sampling) — happens inside `serve_server()`. There is no `Peer<RoleServer>` at construction time. `peer_info().capabilities.sampling` requires a live peer handle, which only exists in `RequestContext<RoleServer>` (per-call) or `NotificationContext<RoleServer>` (in `on_initialized` callback).

The Round 2 architect + minimal-change convergence on "construction-time check eliminates parallel-path debt" is correct in PROPERTY but misnamed the location. The conclusion's "at `MengdieServer::new`" cannot be implemented as written.

**Disposition**: integrate as mechanism correction (Decision Summary table updated above). The corrected pattern preserves the "single resolution per process; no per-call branching" property by post-handshake lazy init. Plan author picks among:
- **OnceLock lazy init**: `OnceLock<Box<dyn LlmProvider>>` resolved at first `memory_dream` invocation. Pros: simple, deterministic. Cons: first invocation pays detection cost.
- **`on_initialized` callback override**: rmcp `ServerHandler::on_initialized` captures the Peer, resolves provider, stores in `OnceLock` or similar. Pros: no first-invocation cost. Cons: requires confirming rmcp's ServerHandler trait supports this hook.
- **Two-phase server construction**: separate "config" and "live" phases; provider resolved in live phase after handshake. Pros: explicit. Cons: more plumbing.

Plan-time hard gate: **verify rmcp API surface BEFORE writing provider selection code.** If none of the three patterns work cleanly with rmcp v1.3, return to discussion 024 — but this is a low-probability outcome since rmcp has documented `RequestContext<RoleServer>` access patterns.

### Strategic (specific)

Same as above. Smartest improvement: replace location language; preserve property; flag as plan-time API verification step.

### Adversarial (specific)

Same as above. Block is on mechanism-as-written, not on build decision. The correct mechanism is post-handshake selection using OnceLock or equivalent.

### Regret (specific)

Same as above. Construction-time-check decision is "most likely to be reversed in 6 months" — but per the 3-way convergence, the team can prevent the reversal pre-emptively by correcting the conclusion now. The other decision items (build-now, McpSessionProvider shape, fallback to ClaudeCliProvider, sequencing, SourceType cleanup) are lower reversal risk.

## Spawned Discussions

| # | Topic | New Discussion | Reason |
|---|-------|----------------|--------|
| (none — single topic resolved within discussion 024) |

## Deferred Resolutions

| # | Topic | Resolution | Detail |
|---|-------|------------|--------|
| (none — Round 2 swept all questions) |

## Sequencing (relative to other in-flight work)

- BL-009 **design + plan + review** can happen in parallel with v0.8.5 work. McpSessionProvider design is architecturally independent of v0.8.5 (codex Round 2 confirmed; cluster-hash computation happens in `insert_synthesis_with_links` regardless of v0.8.5 NOT NULL trigger active or not).
- BL-009 **implementation should land in v0.9.0 AFTER v0.8.5 ships to main**. Operational safety: v0.8.5's NOT NULL trigger should be active as safety net before any new synthesis-writing code path ships.
- **Cleanup item identified by challenger Round 2** — `memory_ingest` `SourceType::Synthesis` path is broken post-v0.8.5 regardless of BL-009. Should be removed in v0.8.5 sprint or as standalone commit, NOT bundled into BL-009.

## Out-of-scope items called out

- Synthesis prompt content (handled by `src/core/synthesis.rs`, separate concern)
- Quality measurement (BL-audit-collection-discipline territory)
- Multi-LLM support (LlmProvider trait stays as-is; McpSessionProvider is just a new impl)
- Daemon mode (BL-010 territory)
- RAG retrieval (BL-012 territory)

## Open questions for plan-time (NOT blocking conclusion)

- **HARD GATE per Doodlestein 3-way convergent finding**: verify rmcp API surface for post-handshake provider resolution before writing provider selection code. Three patterns to evaluate: OnceLock lazy init at first `memory_dream` call / `on_initialized` callback override / two-phase server construction. Plan must pick one and document the rationale.
- Exact rmcp v1.3 client-capability detection API surface (`peer_info().capabilities.sampling` is plausible but needs verification at code time, AND must be query-able through whichever post-handshake pattern is picked)
- Exact LOC range (50-150 spread across reviewers; settles in plan)
- Per-cluster event log shape on extended `SynthesisResult` (3 reviewers agree direction; specifics in plan)

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude | Start |
| challenger | pure adversarial opposition | Claude | Round 1 |
| minimal-change-engineer | anti-bloat / minimum-machinery | Claude | Round 1 |
| software-architect | system-shape + dependency analysis | Claude | Round 1 |
| codex-proxy | technical-debt + risk-of-deferral lens (OpenAI) | Direct fallback (codex MCP unavailable model-version-mismatch from earlier discussion) | Round 1 |
| gemini-proxy | product / UX / momentum lens (Google) | oMLX gemma4:26b fallback (Gemini API rate-limited) | Round 1 |

Round 0 framing-review team (separate lifecycle): same 5 reviewers ran 3 attempts — overridden by user on attempt 3 (4 APPROVED + 1 REVISE pattern; gemini-proxy REVISE asked to fill mechanism trade-off into framing, conflicted with other reviewers' "no further trim" position).

## Process Metadata

- Discussion rounds: 2 (plus Round 0 framing-review × 3 attempts)
- Topics: 1 total (1 converged, 0 spawned, 0 deferred-explained)
- Autonomous decisions: 1 (TL decided per evidence + spec "Decide, don't ask")
- User escalations: 1 (Round 0 attempt 3 hit rerun-limit; user chose Override + proceed)
- Doodlestein challenges: pending Step 9
- Deferred resolved in Sweep: 0 (no deferred items existed)
- Round-1-to-Round-2 position movements: 4 of 5 (gemini conceded new-tool; minimal-change FLIPPED defer→yes; architect FLIPPED Shape B → McpSessionProvider; codex confirmed Path C convergence; only challenger held overall position with technical concessions)
- TL verification artifact items checked: 11 (9 ✓ verified; 2 unvalidated but not load-bearing for decision)

## Key findings (load-bearing for downstream)

1. **rmcp v1.3 server-initiated sampling is supported** (TL verified via context7): `Peer<RoleServer>::create_message()` exists; `CreateMessageRequestParams` type defined; `enable_sampling_tools` available under `server` features (mengdie has them). This is the load-bearing finding that flipped 3/5 positions in Round 2.

2. **Construction-time capability check eliminates parallel-path debt**: architect + minimal-change converged on this. `MengdieServer::new` resolves provider once per process; no per-call branching; no test-matrix doubling.

3. **Cluster-hash invariant is satisfied transparently**: McpSessionProvider replaces only step 3 (LLM dispatch) of the 4-step `run_synthesis_pass` pipeline (cluster select → prompt build → LLM dispatch → parse-and-persist). Steps 1, 2, 4 unchanged; `insert_synthesis_with_links` is the unchanged single writer.

4. **Discussion 008 "extend, don't add" precedent is satisfiable**: McpSessionProvider adds zero load-bearing tools. The thin `memory_dream` wrapper is a trigger surface (5-30 LOC, no new write paths). 008 precedent applies.

5. **BL-010 daemon orthogonal**: daemon will use ClaudeCliProvider (subprocess); in-session uses McpSessionProvider (host); both implement same trait, both call `run_synthesis_pass`. No queue model foreclosed.

6. **Challenger's "defer-until-trigger" position acknowledged but not winning**: challenger argued runtime-fallback-makes-build-now-pointless. But silent fallback means zero user-visible delta either way; build cost is bounded (~50-150 LOC); future LlmProvider impls (BL-011 entity extraction? BL-012 RAG?) benefit from tested-in-production sampling path. TL judgment: yes-build wins under cost-benefit + 4/5 weight.

## Next Steps

→ `/ae:plan` BL-009 (after v0.8.5 ships to main per Sequencing). Plan should:
   1. Verify rmcp client-capability detection API surface
   2. Implement McpSessionProvider (~50-150 LOC) in `src/core/llm.rs`
   3. Add construction-time provider selection in `MengdieServer::new`
   4. Add `memory_dream` tool wrapper in `src/core/mcp_tools.rs`
   5. Extend `SynthesisResult` with per-cluster event log
   6. Integration test against Claude Code (or equivalent MCP client with sampling)
   7. Cleanup: remove `SourceType::Synthesis` from `memory_ingest` (per challenger Round 2 finding)

→ Per discussion 023 conclusion's sequencing: this discussion concluding unblocks `/ae:roadmap plan v0.8.5`. v0.9.0 plan/work follows v0.8.5 close.

→ Independent: the `memory_ingest` `SourceType::Synthesis` cleanup item should land in v0.8.5 sprint as small ride-along (XS) — it's an independent bug fix that doesn't depend on BL-009 design.
