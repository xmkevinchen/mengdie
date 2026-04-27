---
round: 2
date: 2026-04-27
type: tl-synthesis
---

# Round 2 — TL Synthesis

## Per-agent files

- `round-02/challenger.md`
- `round-02/minimal-change-engineer.md`
- `round-02/software-architect.md`
- `round-02/codex-proxy.md`
- `round-02/gemini-proxy.md`

## Position movements (Round 1 → Round 2)

| Agent | R1 | R2 | Driver |
|-------|----|----|--------|
| **gemini-proxy** | YES build, new tool | YES build, **McpSessionProvider** (conceded) | minimal-change F4 + codex Path C; pruned prompt-level context-reuse claim per challenger F4 |
| **challenger** | NO defer (broken stub + no win) | **NO defer maintained**; F1/F4/F6/F7 conceded | TL verification + peer evidence on McpSessionProvider feasibility/cost — but timing question (Claude Code runtime advertisement) keeps defer alive |
| **minimal-change-engineer** | NO defer (gated on rmcp sampling) | **YES build (FLIPPED), McpSessionProvider primary** | TL rmcp verification fired explicit gating condition |
| **software-architect** | Shape B (conditional on rmcp absent) | **McpSessionProvider (conceded)** | TL rmcp verification + structural reasoning that shape E is BL-010-orthogonal under daemon-as-orchestrator model |
| **codex-proxy** | YES Path C | YES (confirmed convergence with minimal-change) | minimal-change F4 = same shape; LOC delta noted |

## Pruned

- **Pruned**: gemini-proxy's "prompt-level context reuse" claim. All 5 (including gemini herself) accept challenger F4 — synthesis prompt is self-contained; prior session context adds nothing at the prompt layer. Survived gemini's claim: UX-conversational structured-feedback visibility (Kai sees outcomes in chat).
- **Pruned**: Shape B (architect's Round 1 preference). Architect explicitly conceded: "Round 1 Shape B lean was structurally driven by the assumption rmcp couldn't support sampling. Falsified → reasoning inverts." 0/5 advocates remain in Round 2.
- **Pruned**: gemini's Round 1 "new MCP tool" preference. Conceded by gemini herself, citing minimal-change F4 + codex Path C. 0/5 advocates remain.
- **Pruned**: per-call runtime branching for sampling capability. Architect + minimal-change explicitly converge: "Construction-time capability check at `MengdieServer::new`, NOT per-call." Eliminates parallel-path debt that challenger flagged in Round 1.
- **Pruned**: BL-009 stub literal text ("Claude calls memory_ingest"). 5/5 already pruned in Round 1.
- **Pruned**: nothing else.

## Of-framing disposition

Round 2 framing-level challenges:

1. **Architect amendment (construction-time capability check)**: framing didn't address WHEN sampling capability is detected. Architect's "construction-time, single resolution" is now load-bearing for the design; eliminates challenger's parallel-path-debt critique. **Disposition: integrate as design-level decision in conclusion.**
2. **Minimal-change F1 (v0.8.5 hard prerequisite)** vs **codex (v0.8.5 fully independent)**: framing's Reference said "v0.8.5 closes the bypass at DB layer" but didn't speak to BL-009 timing. **Disposition: integrate as nuanced — McpSessionProvider design is independent of v0.8.5 (codex correct on architectural independence) BUT production safety wants v0.8.5 NOT NULL trigger ACTIVE before BL-009 starts writing synthesis rows from a new path (minimal-change correct on operational ordering).** Resolution: BL-009 plan can be written / reviewed in parallel with v0.8.5 work; BL-009 implementation should land in v0.9.0 after v0.8.5 ships to main.
3. **UX-conversational feedback visibility (gemini)**: framing left feedback shape open. Round 2 converged on extending `SynthesisResult` with per-cluster event log — the thin `memory_dream` tool wrapper returns this to Kai's session. **Disposition: integrate.**

## Verification artifact

| Claim | Source | Verified? |
|-------|--------|-----------|
| rmcp v1.3 supports server-initiated sampling | TL context7 query (2026-04-27) | ✓ — Peer<RoleServer>::create_message() exists |
| Claude Code MCP client advertises sampling | unverifiable at design time | ✗ — runtime concern; design must include construction-time check + fallback |
| McpSessionProvider feasible as new LlmProvider impl | architect F1 + minimal-change F4 + codex Path C all converge | ✓ — confirmed via reading `src/core/llm.rs` LlmProvider trait |
| McpSessionProvider routes through `insert_synthesis_with_links` (cluster-hash safe) | architect F2 + minimal-change F2 | ✓ — `run_synthesis_pass` calls `insert_synthesis_with_links` (db.rs:349); McpSessionProvider replaces only step 3 (LLM dispatch) |
| Daemon (BL-010) can use ClaudeCliProvider; in-session uses McpSessionProvider; orthogonal | architect Round 2 + codex Round 2 + minimal-change Round 2 | ✓ — both providers implement same trait, daemon picks via config |
| LOC estimate range: 50 (minimal-change) to 100-150 (codex/architect) | minimal-change Round 2 + codex Round 2 + architect Round 2 | unvalidated — exact LOC is plan-time question; doesn't affect decision |
| Discussion 008 "extend, don't add" precedent satisfied (0 new tools at load-bearing layer) | architect Round 2 + minimal-change Round 2 + codex Round 2 + gemini Round 2 (conceded) | ✓ — McpSessionProvider adds zero tools; thin `memory_dream` wrapper is a trigger surface, not a load-bearing tool |
| Construction-time capability check (not per-call) eliminates parallel-path debt | architect Round 2 (explicit) + minimal-change Round 2 (explicit) | ✓ — converges; structural reasoning sound |
| BL-010 daemon queue model NOT foreclosed | architect Round 2 explicit retraction of his Round 1 F5 | ✓ — daemon uses ClaudeCliProvider |
| memory_ingest with `SourceType::Synthesis` will fail post-v0.8.5 | challenger Round 2 finding | ✓ — confirmed via plan 017 + discussion 023; should be removed from `memory_ingest` regardless of BL-009 |

## Frame-challenge disappearance self-check

Round 1 carried 4 disagreements forward to Round 2:

- ✓ #1 Q1 build vs defer: explicitly engaged. 4 build vs 1 defer; challenger conceded technical points but holds timing.
- ✓ #2 Q2 mechanism shape: RESOLVED. Pruned to McpSessionProvider 4/5 explicit + 1 conditional accept.
- ✓ #3 Discussion 008 precedent applicability: resolved. McpSessionProvider satisfies precedent (0 new load-bearing tools).
- ✓ #4 BL-010 compatibility: resolved. Architect F5 (Round 1) corrected — daemon and in-session are orthogonal LlmProvider paths.

No silent disappearances.

## Convergence map

**Convergent (4-5 agree)**:
- ✓ **Q2 mechanism = McpSessionProvider** (4/5 explicit; challenger accepts if forced). New LlmProvider impl, ~50-150 LOC, reuses run_synthesis_pass.
- ✓ **Construction-time capability check** at `MengdieServer::new`, single resolution per process, no per-call branching.
- ✓ **Fallback target = ClaudeCliProvider** when sampling unavailable. No new fallback path; reuses existing CLI provider.
- ✓ **Thin `memory_dream` tool wrapper** (~30 LOC) as in-session entry point. Triggers `run_synthesis_pass` with whichever provider was constructed.
- ✓ **Cluster-hash invariant safe**: McpSessionProvider routes through existing `run_synthesis_pass` → `insert_synthesis_with_links` path. v0.8.5 NOT NULL trigger is safety-net.
- ✓ **Discussion 008 "extend, don't add" precedent satisfied**: zero new write paths; 1 thin trigger tool (debatable as "load-bearing").
- ✓ **BL-010 daemon orthogonal**: daemon uses ClaudeCliProvider; in-session uses McpSessionProvider; both call `run_synthesis_pass`. Phase 2 chain unblocked either way.
- ✓ **memory_ingest `SourceType::Synthesis` should be removed** (challenger Round 2): post-v0.8.5 this path is broken regardless of BL-009; small cleanup item.

**Contested**:
- ✗ **Q1 whether: ship in v0.9.0 vs defer-until-trigger**: 4 build (gemini, codex, architect, minimal-change-flipped) vs 1 defer (challenger). Challenger's defer rationale: "Claude Code may not advertise sampling → fallback is silent → zero-user-delta → why ship now?" But (a) all 4 yes-build agree fallback is silent + acceptable, (b) build cost is small (50-150 LOC), (c) build now means BL-010+ and future LlmProvider impls have a tested-in-production sampling path.
- ✗ **v0.8.5 dependency strength**: minimal-change says HARD prerequisite (BL-009 waits for v0.8.5 in main); codex says fully independent. **TL resolution**: BL-009 design + plan + work are architecturally independent; production rollout should follow v0.8.5 to have NOT NULL trigger active as safety net. This is a sequencing concern, not a design dependency.

## TL recommended decision (for Step 5)

**Topic 01 score: converged**

**Decision**: Build BL-009 in v0.9.0 as **McpSessionProvider (new LlmProvider impl)** + **thin `memory_dream` tool wrapper**, with **construction-time capability check** + **fallback to existing ClaudeCliProvider**.

**Specific design points** (load-bearing for the plan):
1. **McpSessionProvider** (`src/core/llm.rs` new impl): satisfies the 2-method LlmProvider trait by sending `sampling/createMessage` (rmcp `Peer<RoleServer>::create_message()`) to the host MCP client.
2. **Capability detection at construction**: `MengdieServer::new` checks `peer_info().capabilities.sampling`. If advertised → instantiate McpSessionProvider; else → instantiate ClaudeCliProvider. Single resolution per process; no per-call branching.
3. **Thin `memory_dream` tool wrapper** (`src/core/mcp_tools.rs`): exposes `memory_dream(project_id?: String, dry_run?: bool)`. Internally calls `run_synthesis_pass` with the constructed provider. Returns `SynthesisResult` extended with per-cluster event log for in-session UX visibility.
4. **No changes to `run_synthesis_pass`, `insert_synthesis_with_links`, or any persistence path**: cluster-hash invariant satisfied transparently.
5. **No changes to existing `mengdie dream --synthesize` CLI**: continues to work with ClaudeCliProvider.
6. **Cleanup item** (challenger Round 2): remove `SourceType::Synthesis` from `memory_ingest` — that path will fail post-v0.8.5 regardless, and BL-009 ensures synthesis is created via the correct path.

**Sequencing**:
- BL-009 design + plan + review can happen in parallel with v0.8.5 work.
- BL-009 implementation should land in v0.9.0 AFTER v0.8.5 ships to main (operational safety: v0.8.5 NOT NULL trigger active before any new synthesis writer ships).

**Rationale**: 4/5 strong convergence on McpSessionProvider with evidence-driven concessions across both rounds. rmcp v1.3 sampling verified ✓. Architect retracted Round 1 Shape B preference; gemini conceded prompt-level context-reuse + new-tool position; minimal-change FLIPPED defer position once gating condition (rmcp sampling) verified. Challenger conceded F1/F4/F6/F7 technical points but holds timing-only "defer-until-trigger" stance — TL judgment: timing argument outweighed by (a) silent fallback semantics making build-now zero-risk, (b) Phase 2 chain (BL-010+ LlmProvider impls) benefits from tested-in-production sampling path, (c) cost is bounded (50-150 LOC).

**Reversibility**: HIGH. McpSessionProvider is a swappable LlmProvider impl; can be deleted. `memory_dream` tool can be unregistered. No schema changes. ClaudeCliProvider remains canonical fallback path.

**Reversibility basis**: Provider pattern means impl swap is trivial. The thin tool wrapper has zero state. The storage path is unchanged.

## Step 5 escalation check

**Decide autonomously?** YES. Team evidence supports a clear direction: 4/5 explicit convergence on McpSessionProvider mechanism + Q1 leaning yes-build. Challenger's defer position is acknowledged but minority + concedes the technical question (mechanism feasible). Per spec "Decide, don't ask," TL decides.

**User-affecting?** Yes — Kai will eventually invoke `memory_dream` from a Claude Code session. But per discussion 023 conclusion's BL-009 sequencing gate, this discussion's purpose was to DESIGN BL-009 before plan; the design here matches Kai's stated goal of using mengdie inside Claude sessions.

**Decision**: TL scores converged; presents to user as FYI, not as escalation.
