---
agent: software-architect
round: 2
topic: 01
created: 2026-04-27
position_round1: "Shape B (two-tool: read clusters + dedicated commit)"
position_round2: "McpSessionProvider — concedes to codex/minimal-change once rmcp sampling is verified, with one structural amendment"
---

# Round 2 — software-architect

## TL-resolved fact carried into Round 2

`Peer<RoleServer>::create_message()` is available in rmcp v1.3 under
the `server` features mengdie already enables (TL synthesis,
`round-01/synthesis.md:48`). My Round 1 open question 1
(`round-01/software-architect.md:204`) — "does rmcp v1.3 expose MCP
sampling at all?" — is RESOLVED YES. This re-opens option E
(McpSessionProvider) which I had conditionally foreclosed at
`round-01/software-architect.md:75-83` ("rmcp v1.3 with
`['server', 'macros', 'transport-io']` does NOT expose MCP sampling").

## Position update

**I concede Shape B and adopt McpSessionProvider** (codex Path C in
`round-01/codex-proxy.md:26`, minimal-change "right shape" in
`round-01/minimal-change-engineer.md:48-65`). My Round 1 lean toward
Shape B was structurally driven by the assumption rmcp could not
support sampling; that assumption is falsified, and the structural
analysis it rested on now points the other way.

The amendment I add to McpSessionProvider: **runtime sampling-capability
detection MUST be observed at provider construction, not at first
`complete()` call**, and the construction site MUST commit to one
provider per pass — no per-cluster fallback. (Reasoning in F4.)

## Findings

### F1. Round 1's "in-tool LLM dispatch breaks BL-010 queue model" claim was overgeneralized

In `round-01/software-architect.md:F5` I wrote that any shape owning
LLM dispatch inside the tool handler's request lifetime forecloses
BL-010. Re-reading with McpSessionProvider in scope, this is only
half-true. The TL cross-cut Q2 surfaces the correct refinement:

**McpSessionProvider does run inside `run_synthesis_pass`, which the
tool handler calls — so the LLM dispatch IS inside the tool
handler's request lifetime in the in-session case.** That part of
my Round 1 claim stands as a fact about the call stack.

But the *queue compatibility* test is not "is dispatch inside the
tool handler" — it's "does the daemon need to use this provider".
The daemon does NOT need to use McpSessionProvider; it can construct
a `ClaudeCliProvider` exactly as today (`src/bin/cli.rs:269,322` per
minimal-change `round-01/minimal-change-engineer.md:14`). The two
provider impls coexist as orthogonal paths over the same
`run_synthesis_pass`. **Daemon→CLI; in-session-tool→MCP-sampling.**
No queue model is foreclosed.

This is a direct concession to TL cross-cut 2: the daemon's queue
uses ClaudeCliProvider; McpSessionProvider is in-session only. My
Round 1 F5 was correct that "in-tool LLM dispatch" is a property
of the in-session shape — but I conflated "this shape blocks the
daemon" with "this shape is in-tool". The daemon doesn't have to
use it, so it's not blocked.

### F2. Architectural rule "insert_synthesis_with_links is the only synthesis writer" is satisfied MORE strongly by McpSessionProvider than by Shape B

My Round 1 F2 + the rule it produced
(`round-01/software-architect.md:131-138`) — "`insert_synthesis_with_links`
is the only synthesis writer" — is the load-bearing constraint.

Compare:

- **Shape B (two-tool)**: the commit-tool handler calls
  `insert_synthesis_with_links`. New code path, new test surface, new
  trust boundary (the host Claude passes back source_ids; the commit
  tool must validate they're a real cluster). The rule holds, but
  there's a *new caller* of `insert_synthesis_with_links` and a
  *new way* for source_ids to enter that function (from a tool
  argument).
- **McpSessionProvider**: `run_synthesis_pass` calls
  `insert_synthesis_with_links` with source_ids it computed itself
  from `cluster_memories(...)` (`dreaming.rs:408`). **Zero new
  callers, zero new write paths.** The provider impl only changes
  step 3 (LLM dispatch) of the four-step pipeline I mapped in
  `round-01/software-architect.md:F1`. The rule's enforcement
  surface stays exactly where it is today.

Direct answer to TL cross-cut 3 ("any architectural preference?"):
**McpSessionProvider preserves the rule with strictly less surface
area to audit**. Shape B reproduces the same guarantee but expands
the trust perimeter. Both work; McpSessionProvider is structurally
cleaner.

### F3. First-caller pressure-test (plan 010 pattern) — McpSessionProvider tests MORE design bets

Round 1 F6 (`round-01/software-architect.md:140-160`) listed the
design bets BL-009 stress-tests. Re-applied to the two candidate
shapes:

| Design bet | Shape B tests | McpSessionProvider tests |
|---|---|---|
| Cluster selection as read-only primitive | YES (returns clusters to host) | NO (clusters stay internal to `run_synthesis_pass`) |
| Multi-writer cluster-hash dedup | YES (commit-tool is 2nd writer) | NO (still single writer: `run_synthesis_pass` via `insert_synthesis_with_links`) |
| LlmProvider trait extensibility under non-subprocess backends | NO (no new provider impl) | YES (new impl validates the trait carries an MCP-sampling backend) |
| rmcp sampling end-to-end (server-initiated `create_message` → host LLM → response → back to server) | NO (uses tool args only) | YES (this IS the sampling round-trip) |
| Runtime capability negotiation between server and client | NO | YES (must check sampling capability at startup or construction) |

Direct answer to TL cross-cut 4: **McpSessionProvider validates the
LlmProvider trait + rmcp sampling pair**, which is genuinely
new design territory for mengdie. Shape B validates "MCP tool with
write side effects from host-supplied IDs", which is a less
load-bearing bet (we already have `memory_ingest` with
`resolves: Option<Vec<String>>` — same pattern, different name).

The richer pressure-test wins on the plan-010 precedent: the point
of first-caller plans is to prove dependencies work, not to add
ceremony. McpSessionProvider proves more of the dependency stack.

### F4. The runtime-fallback question is where Shape B almost wins back — but doesn't

Minimal-change's open question 2 in
`round-01/minimal-change-engineer.md:212-214` is the operational
risk: even if rmcp v1.3 has sampling server-side, **Claude Code
must advertise the sampling client capability** for the round-trip
to work. The TL synthesis flagged this explicitly: "Claude Code
client sampling capability advertisement: UNVERIFIABLE at design
time" (`round-01/synthesis.md:49,98`). The synthesis directive
mandates: "any McpSessionProvider proposal" must include "runtime
fallback to ClaudeCliProvider if sampling unavailable".

I agree with the mandate but want to constrain *where* the
fallback lives:

- **At provider construction (preferred)**: when
  `MengdieServer::new` runs, query the connected client's
  capabilities. If sampling absent, construct `ClaudeCliProvider`
  for any in-session synthesis trigger. If sampling present,
  construct `McpSessionProvider`. Provider choice is a
  *configuration result*, not a per-call decision. This is the same
  shape as `build_provider(&LlmConfig)` at
  `src/core/llm.rs:190-195` today.
- **At first `complete()` call (rejected)**: detect lack of
  sampling on first failed reverse-call, fall back. This creates
  exactly the parallel-path debt challenger flagged at
  `round-01/challenger.md:111-117`: synthesis runs partially under
  one provider, partially under another, with mid-pass error
  recovery semantics that need their own test matrix.

Direct answer to TL cross-cut 4 ("does runtime detection + fallback
re-create challenger's parallel-path debt?"): **only if the
fallback happens mid-pass.** A construction-time fallback is just
"this server, in this session, picked one provider"  — no parallel
paths, no test-matrix doubling. Construction-time fallback means
challenger's F6 critique
(`round-01/challenger.md:96-103`) — "McpSessionProvider doesn't
solve the persistence bypass" — gets fully neutralized: whichever
provider is chosen, `run_synthesis_pass` still calls
`insert_synthesis_with_links` correctly.

### F5. Discussion 008 precedent: McpSessionProvider satisfies it; Shape B violates it; new-tool (gemini) violates it more

Direct answer to TL cross-cut 5. The 008 precedent
(`round-01/minimal-change-engineer.md:91-104`,
`round-01/codex-proxy.md` doesn't address it directly,
`round-01/gemini-proxy.md:111-122` argues against applying it):

- **McpSessionProvider**: zero new MCP tools, zero new public API
  surface. A `memory_dream` 5-line wrapper (per minimal-change
  F4 close, `round-01/minimal-change-engineer.md:148-151`) is
  *optional* and arguable; the LlmProvider impl alone delivers the
  capability for any caller (including the existing
  `mengdie dream --synthesize` CLI command if invoked under MCP
  attachment, though that combination is contrived). **Cleanest
  fit with 008.**
- **Shape B (two-tool)**: adds two tools (`memory_dream_clusters` +
  commit handler). 008's bar was "what does this provide that
  cannot be expressed by extending existing tools." Shape B's
  affirmative case is "the read and the write are separated by an
  unbounded-latency LLM step" (my Round 1) — but this is *exactly*
  what `complete()` already abstracts in the LlmProvider trait,
  and McpSessionProvider exposes it through that abstraction
  rather than through a new tool surface. **Shape B fails 008
  unless the LlmProvider abstraction is rejected for this
  use case, and there's no evidence to reject it.**
- **Gemini's "new tool with structured feedback"**
  (`round-01/gemini-proxy.md:111-122`): argues for the new tool
  on the basis of "semantics are genuinely different." But the
  semantics gemini lists ("trigger synthesis") are *exactly*
  `mengdie dream --synthesize`'s semantics today, just routed
  through MCP. Same semantics, different entry point — the 008
  precedent applies.

I land where minimal-change landed: McpSessionProvider satisfies
008 strictly; Shape B partially violates it; new-tool fully
violates it. **My Round 1 advocacy of Shape B was wrong on this
axis** — I had been treating the "read and write separated by LLM"
property as if it required a tool-surface manifestation, when it's
already manifested in the LlmProvider trait.

### F6. The "tool surface" question is nearly orthogonal to the provider question

Worth surfacing for round 2 to close cleanly. The choices are:

- **No new tool, McpSessionProvider only.** In-session synthesis
  triggers via... what? CLI invocation isn't natural in-session.
  This requires *some* surface — a tool, a CLI command exposed via
  a hook, etc.
- **Thin tool + McpSessionProvider.** A 5-line `memory_dream` tool
  whose handler is `run_synthesis_pass(.., &self.provider, ..)`.
  This is what minimal-change explicitly proposed
  (`round-01/minimal-change-engineer.md:148-151`). It's a tool,
  but its body has zero synthesis-specific logic — it's a
  trigger, not a write path. **My read: this is consistent with
  008.** 008 rejected `memory_resolve_conflict` because the
  *behavior* could be expressed by extending other tools. Here,
  the behavior is "call the existing `run_synthesis_pass`" — a
  trigger surface for an existing primitive isn't the same as a
  parallel write path.
- **Two new tools, Shape B.** Read clusters + commit. Both have
  meaningful bodies; both touch persistence semantics. Heavy.

I now favor "thin tool + McpSessionProvider": one `memory_dream`
tool, body delegates to `run_synthesis_pass`. The 008 precedent
holds because the tool is a trigger, not a write path; the
discussion-008 reasoning applied to a tool that *expanded
behavior surface*, which this doesn't.

## Agreements

- **Stub is mechanically broken** — agree with all five peers
  (`round-01/synthesis.md:32`, "UAG-style finding"). My Round 1 F3
  said the same independently; cross-validated.
- **Cluster-hash invariant lives at DB layer** — agree with
  challenger F1 (`round-01/challenger.md:13-28`), minimal-change F3
  (`round-01/minimal-change-engineer.md:36-46`), codex 1.A
  (`round-01/codex-proxy.md:14-39`). My Round 1 F2 was a more
  expansive version of the same conclusion.
- **`run_synthesis_pass` is the canonical orchestrator** — agree
  with codex (`round-01/codex-proxy.md:99`). My Round 1 F1
  decomposed it into 4 layers; codex confirms the orchestrator
  identity. No conflict.
- **McpSessionProvider is the right shape if rmcp sampling works**
  — codex Path C (`round-01/codex-proxy.md:26-37`) and
  minimal-change F4 (`round-01/minimal-change-engineer.md:48-65`)
  reach the same conclusion through different lenses (drift-cost
  vs. minimum-machinery). TL verification confirms the precondition.
  I now agree, against my Round 1 lean.
- **Construction-time provider selection is the right fallback site**
  — extends minimal-change F5 (`round-01/minimal-change-engineer.md:67-79`)
  by being specific about *when* the choice is made. Not a
  disagreement — a sharpening.

## Disagreements

### D1. With gemini-proxy on F4 ("Synthesis fires as background job")

Gemini at `round-01/gemini-proxy.md:60-63` envisions BL-009 as
something that "fires as background job" with risk of interrupting
unrelated work. This is BL-010's job (daemon), not BL-009's.
**BL-009 is by-definition a manual trigger** (the user invokes a
tool). The gemini scenario B describes a feature that doesn't
exist in BL-009 and shouldn't. The "shared session context" win
gemini argues for at `round-01/gemini-proxy.md:48-63` collapses
into "the user explicitly invoked synthesis while in session" —
a real-but-narrow scenario.

I think the synthesis pruning at `round-01/synthesis.md:33` got
this right: the *prompt-level* context-reuse claim is phantom
(my Round 1 implicitly + challenger F4); the *conversational-UX*
claim is real but small.

### D2. With challenger on F7 ("BL-009 is a possible dead-end")

Challenger at `round-01/challenger.md:106-119` argues BL-009
forecloses BL-010's queue. This is the same overgeneralization I
made in Round 1 F5 and that TL cross-cut 2 corrected: McpSessionProvider
is in-session-only; daemon uses ClaudeCliProvider; orthogonal paths.
Challenger's F7 is correct *only* about the stub's mechanism (which
all 5 of us agree is broken anyway). For McpSessionProvider, F7
doesn't apply.

### D3. With codex on Path C complexity estimate

Codex at `round-01/codex-proxy.md:26` estimates Path C at "100-150
LOC". I think this is a Shape B / new-tool estimate that got
attached to Path C. The actual McpSessionProvider impl is closer to
minimal-change's "~50 LOC" (`round-01/synthesis.md:26`): a struct
holding a peer handle, an impl of `LlmProvider::complete` that wraps
`peer.create_message(...).await` and maps the result to `String`. The
heavyweight items (cluster selection, prompt construction, parsing,
persistence) are all reused from `run_synthesis_pass`. Codex's
"100-150 LOC" may have included a `memory_dream` tool wrapper +
capability negotiation logic + tests — those are real, but they're
additive, not core.

This isn't a load-bearing disagreement, but the LOC estimate
materially affects the "is this worth doing" math in challenger
and minimal-change. The actual cost is closer to the
minimal-change estimate.

## Open Questions

1. **(Operational, not architectural)** What happens if Claude Code's
   client doesn't advertise sampling? The construction-time
   fallback puts mengdie on `ClaudeCliProvider`, which today
   spawns a subprocess that itself runs Claude. From inside an
   active Claude Code session, this means **the user's session
   spawns a sibling Claude subprocess to do synthesis**, which is
   weird ergonomically (might prompt for auth, contends for rate
   limits) but architecturally correct (BL-010 will face the same
   choice). Not a blocker for design, but worth a note in the plan.

2. **Should the thin `memory_dream` tool also accept a
   `provider_override` parameter** for explicit "force CLI" or
   "force in-session" testing? My instinct: yes, behind a debug
   feature flag, because the construction-time choice will be
   exercised once per server start and we'll want a testing
   override to drive both branches in CI. Keeps the parallel-path
   risk contained to test-only code.

3. **Triggering surface beyond a tool**: minimal-change's F4 close
   noted the tool is "optional, ~5 lines"
   (`round-01/minimal-change-engineer.md:148-151`). If we ship
   McpSessionProvider but no tool, is the in-session entry path
   the existing `mengdie dream --synthesize` CLI command, executed
   in some way that picks up the McpSessionProvider? That doesn't
   compose (the CLI binary doesn't have an MCP peer handle). So
   the tool is *practically* mandatory — not optional — to deliver
   any in-session win at all. Worth nailing down so Round 3 doesn't
   relitigate.

4. **Is gemini's "structured per-cluster feedback"
   (`round-01/gemini-proxy.md:30-33`) a separate BL?** That UX
   improvement is real (visibility into per-cluster
   skip/error/success), but it's orthogonal to the
   in-session-vs-CLI question — `mengdie dream --synthesize`
   should also surface this. If Round 3 endorses it, file as a
   separate small BL ("structured SynthesisResult JSON output
   for both CLI and tool callers") rather than coupling it to
   BL-009's mechanism choice.

## Summary

| Round 1 position | Round 2 position |
|---|---|
| Shape B (two-tool: read + commit) | McpSessionProvider + thin trigger tool |
| Q1 not explicitly taken | Build, conditional on sampling availability checked at construction |
| BL-010 compatibility ruled out C and E | Daemon uses ClaudeCliProvider; McpSessionProvider in-session-only; orthogonal paths; no foreclosure |
| Architectural rule: `insert_synthesis_with_links` is the only synthesis writer | UNCHANGED. McpSessionProvider satisfies this *more cleanly* than Shape B (no new caller of `insert_synthesis_with_links`) |
| Discussion 008 precedent partially violated by Shape B | McpSessionProvider satisfies 008; thin trigger tool is a debatable but defensible 5-line surface |

The verification of rmcp sampling moves McpSessionProvider from
"clean abstraction we can't have" to "clean abstraction we can have".
Once available, its structural advantages (single writer, no new
trust boundary, validates more design bets, satisfies 008) dominate
Shape B on every architectural axis I can name. I concede.
