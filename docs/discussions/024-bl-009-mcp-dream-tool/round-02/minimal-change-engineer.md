---
agent: minimal-change-engineer
round: 2
discussion: "024"
topic: "01"
created: 2026-04-27
position: "yes-ship-conditional — McpSessionProvider only, with runtime fallback; defer if v0.8.5 not landed"
position_change_from_round_1: "shifted from 'do-not-ship/defer-until-trigger' to 'yes-ship via McpSessionProvider' because TL verified rmcp sampling is feasible. The blocker that drove my Round 1 'defer' is now resolved."
---

# Round 2 — Minimal Change Engineer

## Findings

### F1. TL verification flips my Round 1 conditional

My Round 1 position
(`round-01/minimal-change-engineer.md:111`, "Position: do-not-ship in
v0.9.0") was explicitly conditioned on Open Question 1
(`round-01/minimal-change-engineer.md:178-181`, "Does rmcp v1.3 expose
MCP sampling…If no, McpSessionProvider is infeasible…I lean defer").

TL synthesis (`round-01/synthesis.md:48`) reports
`Peer<RoleServer>::create_message()` exists, `CreateMessageRequestParams`
type is defined, `enable_sampling_tools` is available under
`server`/`macros` features which mengdie already enables (Cargo.toml
per architect F4 at `round-01/software-architect.md:90-91`). This
contradicts architect's F4 ("rmcp v1.3…does NOT expose MCP sampling"
at `round-01/software-architect.md:89-102`) — the verification overrides
the architect's claim.

Cleanest architectural option (`round-01/software-architect.md:160-161`,
"E is the cleanest by the LlmProvider abstraction it would slot into")
is now actually feasible. **My conditional fires.** Position shifts to
yes-ship via McpSessionProvider.

### F2. Latency was never the case (I concede challenger F2)

Challenger F2 (`round-01/challenger.md:31-46`, "No measurable latency
win for the solo-user case") is correct. My Round 1 F6
(`round-01/minimal-change-engineer.md:72-77`) acknowledged this:
"~1-3s plus model latency…there is no recurring hot-loop pressure here
(Dreaming runs daily at most)." I concede latency is not the value
proposition.

The actual value is **architectural cleanliness** plus **codex's
"single-backend" argument** (`round-01/codex-proxy.md:36-37`,
"Path C is strongly preferred. Unifying CLI and MCP over a single
backend backend (`run_synthesis_pass`) eliminates the risk of
'MCP-only bugs' or divergent retry/observability logic"). The
McpSessionProvider shape achieves this with ZERO new tool surface
and ZERO new write path. That's the minimum-machinery answer.

### F3. McpSessionProvider beats Shape B on minimum-machinery

Architect's Shape B (`round-01/software-architect.md:154`, "Two-tool
(read clusters / commit synthesis)…clean — commit tool owns hash
computation") is the strongest competitor. Comparing on
machinery-actually-added:

| Aspect | McpSessionProvider | Shape B (two-tool) |
|---|---|---|
| New MCP tools | 0 | 2 (`memory_dream_clusters` + `memory_dream_commit`) |
| New write path | 0 (reuses `insert_synthesis_with_links`) | 1 (commit tool body wraps the same fn) |
| New `LlmProvider` impl | 1 (~50 LOC + sampling call) | 0 |
| Reuses `run_synthesis_pass` | 100% (architect F1, `round-01/software-architect.md:18-34`) | 0% — re-implements the orchestrator across two tool calls |
| Reuses prompt builder | yes (called by `run_synthesis_pass`) | yes — host Claude has to be re-given the prompt by `memory_dream_clusters` output |
| Reuses parse + null-escape-hatch | yes | NO — parsing now happens in commit-tool with all 11 metrics fields to re-plumb |
| Reuses 11 `SynthesisResult` metrics | yes | NO — must reconstruct across two tool invocations or drop |
| Discussion 008 "extend, don't add" | passes (no new tools) | **fails** (adds 2 tools) |

McpSessionProvider is strictly less code, strictly less new state, and
honors the discussion 008 precedent. Shape B fails the precedent test
twice (2 new tools instead of 1). The tradeoff Shape B claims to win
on is BL-010 compat — addressed in F5 below.

### F4. Codex Path C and my McpSessionProvider are the SAME design

Codex (`round-01/codex-proxy.md:23-26`, "Path C — New `McpLlmProvider`
+ reverse tool call infrastructure…CLI and MCP paths unified over
single `run_synthesis_pass` backend") describes the same shape I
proposed in Round 1 F4
(`round-01/minimal-change-engineer.md:54-65`, "McpSessionProvider
reuses 100% of clustering, prompt construction, response parsing,
insert_synthesis_with_links, all 11 metrics fields"). Different name,
same design.

**Delta details I want to flag**:

- **LOC estimate**: Codex says 100-150 LOC
  (`round-01/codex-proxy.md:26`); I said ~50 LOC
  (`round-01/minimal-change-engineer.md:178`). The truth is in between
  and depends on how the rmcp `Peer` handle gets threaded into the
  provider impl. The provider's `complete()` body is small (build
  `CreateMessageRequestParams`, await `peer.create_message()`, extract
  text). The non-trivial cost is **plumbing the Peer into the
  provider** at construction time — `MengdieServer` doesn't currently
  hold a `Peer<RoleServer>` reference; rmcp gives it via the
  `ServerHandler` lifecycle. Realistic estimate: 50-80 LOC for the
  provider + 20-40 LOC for the plumbing + 30-50 LOC for the
  thin-tool wrapper that triggers `run_synthesis_pass`. Call it
  ~120-150 LOC total. Codex's number is closer than mine.
- **Fallback shape**: Codex doesn't fully specify
  (`round-01/codex-proxy.md:115-116` Open Question 2). My proposal:
  at provider construction time, query the client capability via
  `Peer::peer_info()` (or the moral equivalent in rmcp v1.3); if the
  client did NOT advertise sampling capability,
  `build_provider(&cfg)` returns `ClaudeCliProvider` instead of
  `McpSessionProvider`. **One-time decision at startup, no runtime
  branching per cluster.** This keeps the trait surface clean and
  avoids the parallel-path debt challenger flagged
  (`round-01/challenger.md:111-119`).

So I fully concur with Path C direction; the deltas are
implementation-detail refinements, not design disagreements.

### F5. Challenger's BL-010 dead-end concern (F7) and architect's
shape-table BL-010 mark for E (`round-01/software-architect.md:157`,
"E…owning the LLM dispatch inside the tool handler is the
BL-010-incompatible shape") need a direct response.

Both peers argue: McpSessionProvider puts LLM dispatch inside a tool
handler's request lifetime; BL-010's daemon model requires
synthesis to be enqueueable as a job. Shape B (two-tool) maps cleanly
onto enqueue/dequeue.

**My counter**: BL-010's daemon DOES NOT need MCP sampling at all.
A daemon process is *not* attached to a Claude session by definition
— it's a long-running system process. The daemon will use
`ClaudeCliProvider` (subprocess), not `McpSessionProvider`. So:

- In-session synthesis (BL-009) → `McpSessionProvider` →
  `run_synthesis_pass` → `insert_synthesis_with_links`.
- Background synthesis (BL-010) → daemon dequeues a job →
  `ClaudeCliProvider` → `run_synthesis_pass` →
  `insert_synthesis_with_links`.

Same backend (`run_synthesis_pass`), two providers, ONE write path.
This is exactly the codex Path C "single-backend" win
(`round-01/codex-proxy.md:65`, "Unification + messaging layer (one
place) — CLI + MCP fully synchronized"). BL-009 doesn't have to be
queueable because BL-010 doesn't route through MCP.

The architect's "BL-010-incompatible" mark for shape E is conditional
on the wrong assumption — that BL-010 needs to share BL-009's
execution path. It doesn't. Two tools call into one orchestrator;
the daemon and the in-session tool are distinct callers.

### F6. Concession to gemini on UX visibility

Gemini's F3 (`round-01/gemini-proxy.md:27-33`, "Synthesis Failures
Are Invisible") and Q3 (`round-01/gemini-proxy.md:99-109`, structured
`{synthesized, failed}` output) point at a real Shape-B advantage
McpSessionProvider doesn't get for free.

With McpSessionProvider, the user-facing tool is a thin wrapper
around `run_synthesis_pass`. The wrapper returns the
`SynthesisResult` struct (`dreaming.rs` — clusters_processed,
syntheses_created, llm_call_errors, parse_errors,
syntheses_llm_skipped, etc.). This IS structured per-pass feedback,
just not per-cluster real-time visibility during the synthesis loop.

Two paths to address this without crossing into Shape B:

1. **Per-cluster JSON in SynthesisResult.events** — extend the
   already-returned result struct with an event log
   (`{cluster_id, outcome, reason}` per cluster). The thin tool
   wrapper returns this verbatim. Gemini's Q3(B) recommendation
   (`round-01/gemini-proxy.md:107`, "structured feedback") is
   satisfied via aggregate-after-completion rather than streaming.
   Cost: small extension to `SynthesisResult`. Reuses everywhere.
2. **Status quo aggregate metrics + log line** — already produces
   counters that the thin wrapper can return. Pair-cluster skip
   percentage, residuals_skipped, llm_call_errors are all there.
   Possibly enough for Kai (he's the only user; gemini's UX
   argument is for him specifically).

I prefer (1) — small extension, large UX win, no extra tool surface.
Gemini's recommendation maps onto McpSessionProvider's wrapper output
without needing the two-tool architecture.

**However, I concede**: streaming per-cluster feedback (cluster 1
done, cluster 2 in-flight, cluster 3 starting) is genuinely harder
with McpSessionProvider since the whole pass runs inside one
tool-call response. If Kai's UX requires streaming, Shape B wins on
this specific axis. I argue: aggregate-after-completion is enough
for a daily-batch operation. Streaming is not load-bearing.

### F7. Challenger F4 (context reuse is phantom) is correct, and I
already accepted it in Round 1

`round-01/challenger.md:64-71` argues the synthesis prompt is
fully self-contained; in-session conversation context adds nothing.
TL synthesis pruned this benefit at the prompt level
(`round-01/synthesis.md:33`). I agree. McpSessionProvider doesn't
NEED context reuse to win — it wins on architectural cleanliness +
single-backend guarantees, not on prompt enrichment.

This means gemini's F1 and F6
(`round-01/gemini-proxy.md:13-19,48-63`) are partly weakened: the
"Claude synthesizes relative to the conversation" argument is
prompt-level, which is pruned. The conversation-level UX win
(structured feedback + inline visibility, F3 + F4) survives and is
addressable per F6 above.

### F8. Position cost-benefit, refreshed

| Option | Cost (LOC) | New write paths | New tools | BL-010 compat | UX feedback |
|---|---|---|---|---|---|
| **No-build (Round 1)** | 0 | 0 | 0 | trivial | aggregate via CLI |
| **McpSessionProvider** | ~120-150 | 0 | 0 (or 1 thin trigger) | yes via separate ClaudeCliProvider in daemon | aggregate metrics (extend with per-cluster events) |
| **Shape B (two-tool)** | ~150-200 | 1 (commit tool) | 2 | yes (maps to enqueue/dequeue) | per-cluster real-time |
| **New tool with feedback (gemini)** | ~150 | 1 | 1 | conditional | per-cluster |
| **Path A (extend memory_ingest)** | ~30-80 | 1 in extension | 0 | conditional | aggregate |

McpSessionProvider has the lowest tool-surface cost AND the lowest
parallel-write-path cost of any "yes-build" option. It loses to
Shape B only on streaming per-cluster feedback, which I argue is not
load-bearing for a daily-batch operation.

## Position

**Yes-ship in v0.9.0 via McpSessionProvider**, conditional on:

1. v0.8.5 cluster-hash NOT NULL enforcement has landed before BL-009
   ships (per architect F2-F3, codex §4). If v0.8.5 slips past
   v0.9.0 cut, defer BL-009 with v0.8.5 as the trigger. **Do NOT
   ship BL-009 against pre-v0.8.5 schema** — too easy to leak
   `synthesis_cluster_hash IS NULL` rows.
2. Provider construction implements one-time client-capability check
   (`Peer::peer_info()` or rmcp equivalent). If the connected client
   doesn't advertise sampling, `build_provider` falls back to
   `ClaudeCliProvider` for the entire process lifetime. No per-call
   branching.
3. Thin tool wrapper (~30 LOC) that triggers `run_synthesis_pass`
   with whichever provider was built. Tool name: keep stub's
   `memory_dream` for continuity, but its semantics are now
   "trigger a synthesis pass, return aggregate result + per-cluster
   event log" — NOT the stub's broken "return clusters for Claude
   to ingest separately."
4. `SynthesisResult` extended with per-cluster event log
   (gemini F3/Q3 satisfied without Shape B's tool count).

**This is the discussion 008 precedent applied correctly**: do NOT
add a synthesis-aware variant to `memory_ingest` (Path A) and do NOT
add 2 commit/read tools (Shape B). Add ONE narrow tool that triggers
the existing orchestrator with a provider that uses what's already
there (host Claude via rmcp sampling). Tool surface grows by 1 (from
3 to 4); zero new write paths. The minimal change is an LlmProvider
impl + thin wrapper.

## Agreements

- **Codex Path C ≡ my McpSessionProvider** — same design, different
  name (codex Round 1, `round-01/codex-proxy.md:23-26`; me Round 1,
  `round-01/minimal-change-engineer.md:54-65`). I fully concur on
  the direction. Delta details in F4 above.
- **Codex's single-backend argument**
  (`round-01/codex-proxy.md:36-37`, "Unifying CLI and MCP over a
  single backend…eliminates the risk of 'MCP-only bugs' or divergent
  retry/observability logic"). This is the strongest argument for
  why "extend the trait, don't add tools" wins.
- **Architect F1** (`round-01/software-architect.md:18-34`,
  "run_synthesis_pass is a thin orchestrator that calls four
  pure-ish layers…BL-009's structural question reduces to which
  subset mengdie owns and which the host Claude gets — not 'redesign
  synthesis'"). McpSessionProvider re-routes step 3 only; preserves
  1+2+4. Architect's framing is exactly right.
- **Architect F2** (`round-01/software-architect.md:41-67`,
  cluster-hash invariant lives at DB layer, BL-009 must not
  re-implement). McpSessionProvider satisfies this — it goes through
  `insert_synthesis_with_links` via `run_synthesis_pass`, never
  touches the schema invariant.
- **Architect F5** (`round-01/software-architect.md:104-120`, "any
  BL-009 shape that produces (synthesis_text, source_memory_ids) as
  a logical artifact…is BL-010-compatible"). McpSessionProvider
  produces exactly this artifact through `run_synthesis_pass`. The
  caveat that "the LLM caller doesn't need to be in the same
  process as the DB writer" is fully consistent with my F5 above:
  the daemon uses `ClaudeCliProvider`, the in-session path uses
  `McpSessionProvider`, both call `run_synthesis_pass`.
- **Challenger F1** (`round-01/challenger.md:13-28`, stub is
  mechanically broken). 5/5 convergence per
  `round-01/synthesis.md:32`. Stub-as-written is dead.
- **Challenger F2** (`round-01/challenger.md:31-46`, no measurable
  latency win). Conceded explicitly in F2 above.
- **Challenger F4** (`round-01/challenger.md:64-71`, context reuse
  is phantom). Conceded; gemini's prompt-level claim is pruned.
- **Gemini F3 + Q3(B)** (`round-01/gemini-proxy.md:27-33,99-109`,
  structured feedback per cluster is a real UX win). Concession in
  F6 above — fold into `SynthesisResult` event log; addressable
  without Shape B.

## Disagreements

- **Architect F4** (`round-01/software-architect.md:89-102`, "rmcp
  v1.3…does NOT expose MCP sampling"). Contradicted by TL
  verification (`round-01/synthesis.md:48`). The architect's
  shape-table mark of E as needing rmcp sampling
  (`round-01/software-architect.md:157`, "requires rmcp sampling
  feature (not enabled per F4)") is therefore inaccurate. With
  sampling available, shape E becomes the live winner per the
  same shape-table criteria the architect used.
- **Architect F5 / shape-table BL-010 mark for E**
  (`round-01/software-architect.md:157`, "✗ — owning the LLM
  dispatch inside the tool handler is the BL-010-incompatible
  shape"). Disagreed in F5 above. BL-010's daemon does not share
  BL-009's execution path; daemon uses `ClaudeCliProvider`, BL-009
  uses `McpSessionProvider`, both call `run_synthesis_pass`.
  "BL-010 needs the synthesis loop to be enqueueable" is satisfied
  because the daemon's job-row consumer runs `run_synthesis_pass`
  with the CLI provider. Architect's analysis assumed a shared
  execution path that isn't required.
- **Architect Shape B preference** (implicit from
  `round-01/software-architect.md:166-171`, "B (two-tool) maps
  directly onto BL-010's queue model"). Per F3 above, Shape B
  loses on minimum-machinery (2 new tools, 1 new write path
  wrapper, fails 008 precedent twice) once the BL-010-compat
  argument is corrected via F5. Shape E wins.
- **Challenger F6** (`round-01/challenger.md:93-103`,
  "McpSessionProvider is orthogonal and does not help…would require
  rmcp v1.3 reverse tool call support (not confirmed)…it doesn't
  solve Finding 1 (the memory_ingest bypass)"). Two corrections:
  (a) rmcp v1.3 sampling IS confirmed per TL verification, so the
  feasibility blocker is gone; (b) McpSessionProvider doesn't go
  THROUGH `memory_ingest` — it goes through `run_synthesis_pass`
  which goes through `insert_synthesis_with_links`. The
  `memory_ingest` bypass is irrelevant to McpSessionProvider's
  write path.
- **Challenger F7 BL-010 dead-end concern**
  (`round-01/challenger.md:106-119`, BL-009 in-session conflicts
  with daemon queue model). Disagreed in F5: BL-009 and BL-010 use
  DIFFERENT providers calling the same orchestrator. No conflict.
- **Codex Path C ranked over A and B on cost grounds**
  (`round-01/codex-proxy.md:60-73`). I agreed in Round 1 F4 but
  with a lower LOC estimate. Codex's 100-150 LOC is closer to
  reality once Peer-plumbing is counted (per F4 above).
- **Gemini F1 prompt-level context-reuse claim**
  (`round-01/gemini-proxy.md:13-19`, "Subprocess doesn't know about
  that conversation…synthesized memories ignore the context Kai was
  just discussing"). The synthesis prompt is fully self-contained
  per challenger F4 + my Round 1 F2. The conversation context does
  NOT inform synthesis quality. Gemini's UX-conversational claim
  (structured feedback in same chat session) survives; the
  prompt-level claim is pruned per `round-01/synthesis.md:33`.
- **Gemini Q4 "new tool, not extension"**
  (`round-01/gemini-proxy.md:111-122`, "New tool. The semantics are
  genuinely different"). Partially agree: the *trigger surface*
  warrants a new tool (or could be a CLI-only trigger; we've added
  the McpSessionProvider but a tool may still be needed for
  in-session invocation). I disagree that this means BL-009's
  PRIMARY shape is "new tool" — the new MCP tool is a thin
  ~30-line wrapper, not the load-bearing piece. The load-bearing
  piece is McpSessionProvider, which adds zero MCP surface.

## Direct answers to TL cross-cuts

**Cross-cut 1**: McpSessionProvider is now my primary "yes-ship"
recommendation, not just a conditional. With rmcp sampling verified,
the minimum-machinery answer is McpSessionProvider, not defer.
Position changed.

**Cross-cut 2**: Yes, fully concur with codex Path C direction. Delta
details: codex's 100-150 LOC is more accurate than my 50; fallback
shape should be one-time client-capability check at provider
construction, not per-call branching; ClaudeCliProvider is the
fallback target.

**Cross-cut 3**: Shape B is acceptable as fallback but NOT
preferred — adds 2 tools, 1 new write-path wrapper, and fails 008
precedent. McpSessionProvider's runtime fallback should be
ClaudeCliProvider (existing CLI path), not Shape B. Choosing Shape B
as fallback would be exactly the parallel-path debt challenger
flagged.

**Cross-cut 4**: Concede gemini's UX visibility partially — fold
per-cluster events into `SynthesisResult`, returned by the thin
tool wrapper. Streaming during the synthesis loop is genuinely
harder; I argue it's not load-bearing for a daily-batch operation
where Kai watches a single tool result come back.

**Cross-cut 5**: Latency does not matter at all. The value is
architectural cleanliness (single backend per codex F3) plus zero
new write paths (vs. Shape B's 1) plus honoring 008 precedent (zero
new tools at the load-bearing layer). McpSessionProvider wins on
EVERY minimum-machinery axis except the streaming-feedback edge
case from cross-cut 4.

## Open Questions

1. **Has v0.8.5 cluster-hash NOT NULL enforcement actually landed in
   main yet?** If yes, BL-009 v0.9.0 plan can proceed. If no, BL-009
   must wait. Discussion 023 documented the intent; I have not
   verified the migration is committed and the column is NOT NULL.
2. **What's the rmcp v1.3 API surface for `Peer::peer_info()`** (or
   moral equivalent for "did the client advertise the sampling
   capability")? TL confirmed sampling exists on the server side;
   the client-capability check is the runtime-fallback gate. Plan
   work needs to verify this exists and is callable from a
   `ServerHandler` impl.
3. **Does `MengdieServer` get the `Peer<RoleServer>` handle at
   construction time, or only at a request callback?** rmcp's
   pattern in the calculator example
   (`tests/common/calculator.rs`) doesn't surface this for tool
   handlers. If the Peer is per-request rather than per-server,
   McpSessionProvider must be constructed per-request — which
   changes the build_provider lifecycle. Plan-time concern, not a
   discussion-time concern, but flagging.
4. **Should the new tool be `memory_dream` or rename to something
   that signals "trigger synthesis pass" rather than the stub's
   broken semantics?** Not load-bearing; flagging for cleanup.
   `memory_synthesize` would more accurately describe what the tool
   does post-redesign.

---

**Bottom line shift**: My Round 1 position was "defer because
sampling availability unverified." TL verified sampling. My
conditional fires. **Yes-ship via McpSessionProvider** — single
backend, zero new write paths, honors 008 precedent, BL-010
unaffected (different provider, same orchestrator). Concede latency
is not the win; the win is architectural cleanliness. Concede
gemini's UX visibility partially via `SynthesisResult` event-log
extension. Reject Shape B (more machinery), reject Path A (couples
synthesis to ingest), reject status-quo defer (sampling resolved).
