---
agent: minimal-change-engineer
round: 1
discussion: "024"
topic: "01"
created: 2026-04-27
position: "do-not-ship (defer until trigger)"
---

# Round 1 — Minimal Change Engineer

## Findings

### F1. The CLI path works and was just validated

`src/bin/cli.rs:269,322` already calls `run_synthesis_pass` with a
`ClaudeCliProvider`. The first real run produced 13 syntheses against the
production DB (CLAUDE.md status note + `BL-clustering-validation.md`). That
is the only empirical signal we have about end-to-end synthesis quality
in this repo. The real bottleneck the project flagged from that run is
the 67% residuals rate — a clustering parameter problem, not an LLM
plumbing problem.

### F2. The "user-visible win" of BL-009 is not yet articulated as a problem the user has hit

The framing's "whether" question is answered by enumerating concrete pains
of the shell-out path. Re-reading the framing (lines 14-25), the stub
(`005-phase2-roadmap.md:66-71`), and the Phase 2 chain (BL-009 → BL-010
daemon), I find one stated motivation: "in Claude session, Claude IS the
LLM — no need to shell out." That is a *symmetry* argument, not a *pain*
argument. Latency, context-reuse, UX, cost — none are quantified or even
asserted. The "no need" framing reverses the burden of proof: the
default for a 100-150 LOC tool addition should be "what does it solve."

### F3. The stub's mechanism is broken in exactly the way the framing flags

Stub: "Claude synthesizes inline and calls `memory_ingest`." But
`memory_ingest` (`src/core/mcp_tools.rs:282-409`) goes through
`db.insert_memory` / `db.insert_memory_resolving` — NEITHER of which sets
`synthesis_cluster_hash` or writes `memory_synthesis_links`. The
cluster-hash invariant (framing Reference + v0.8.5 NOT NULL enforcement)
is violated by construction. So the stub-as-written cannot ship; any
`memory_dream` design needs a *second* surface to commit synthesis (or
retrofit `memory_ingest` with a synthesis variant). The framing already
called this out; it's the strongest evidence that the stub was written
before the cluster-hash work landed and has not been re-evaluated.

### F4. The "right" minimum mechanism, IF we ship, is McpSessionProvider

The `LlmProvider` trait (`src/core/llm.rs:180-183`) is exactly two
methods: `complete(system, prompt) -> Future<Result<String>>` and
`model() -> &str`. `run_synthesis_pass`
(`src/core/dreaming.rs:399-579`) takes `&dyn LlmProvider`. If the host
Claude can be modeled as "an LLM that returns text given a (system,
user) pair," then a `McpSessionProvider` impl reuses 100% of:
- clustering (BL-006)
- prompt construction (`synthesis.rs`)
- response parsing + null-escape-hatch (`synthesis.rs`)
- `insert_synthesis_with_links` (db.rs:349 — handles cluster hash + links + dedup)
- all 11 metrics fields on `SynthesisResult`
- pair-cluster skip accounting, truncation counting, content-hash dedup

This is the discussion-008 pattern verbatim: extend the existing
primitive, do not add a new tool surface. **No new MCP tool, no new DB
write path.**

### F5. But McpSessionProvider needs a "reverse tool call" capability that rmcp/MCP does not expose today

This is the load-bearing technical question. `LlmProvider::complete` is
called from inside an MCP tool handler (the hypothetical `memory_dream`
tool, or however we trigger it). To delegate text generation to the host
Claude, the server would need to call back to the client and ask "please
complete this prompt." The MCP spec has *sampling* (`sampling/createMessage`
client capability) for exactly this — but I have not verified that
rmcp v1.3 exposes it as a server-initiated RPC, nor that Claude Code
advertises the sampling capability. **Round 1 should not assume this
works.** If sampling is unavailable, McpSessionProvider degenerates to
a two-tool round-trip ("read clusters" + "commit synthesis") which is
exactly the stub design — and that design is broken (F3).

### F6. The CLI path's cost in the MCP-attached scenario is one fork+exec per cluster

Concrete numbers: `ClaudeCliProvider` spawns `claude -p` with
`--no-session-persistence`. Cold start of a Claude Code subprocess is
~1-3s plus model latency. 13 clusters in the first real run = 13
subprocess spawns. This IS overhead, but it's overhead the user has
already paid once and produced useful output. There is no recurring
hot-loop pressure here (Dreaming runs daily at most).

### F7. Discussion 008 is the precedent

Discussion 008 (`docs/discussions/008-contradiction-detection/conclusion.md`)
explicitly REJECTED `memory_resolve_conflict` as a 4th MCP tool. The
rationale transfers cleanly:

> "Resolution is mechanically complete via existing two-call pattern;
>  gaps are in descriptions, output completeness, and a dropped field —
>  not in tool surface."

For BL-009: synthesis is mechanically complete via `mengdie dream
--synthesize` + `ClaudeCliProvider`. The MCP-attached "gap" is that the
LLM happens to live in the same process tree as the caller — that is a
*symmetry*, not a *correctness* or *capability* gap. Same shape. Same
answer should apply absent new evidence.

### F8. BL-009 → BL-010 chain is not load-bearing

The framing says BL-009 must not foreclose BL-010 (daemon). Inversely:
BL-010 (daemon + SQLite job queue) does NOT require BL-009. A daemon
can shell out to the `claude` CLI exactly the way the CLI does today
— in fact that's the simpler daemon design. BL-011 (async entity
extraction) and BL-013 (KG) depend on BL-010, not on BL-009. So
"BL-009 unlocks the chain" is not true; the chain proceeds whether
BL-009 ships or not.

## Position: do-not-ship in v0.9.0

**Recommendation**: Answer the "whether" question with **no, defer**.
Specifically:

1. Do not add `memory_dream` MCP tool in v0.9.0.
2. Do not add `McpSessionProvider` `LlmProvider` impl in v0.9.0.
3. Keep `mengdie dream --synthesize` (the CLI path) as the synthesis
   entry point.
4. File `BL-009` as `admission_status: defer-until-trigger` (the
   upstream AE feature CLAUDE.md mentions, OR equivalent body
   language) with explicit triggers below.
5. Continue the Phase 2 chain via BL-010 (daemon) + BL-011 (entity
   extraction); both can use the existing `ClaudeCliProvider`.

**Triggers that would re-open BL-009**:

- The user actually attempts to run synthesis from inside a Claude
  Code session and reports concrete friction (latency complaint,
  auth re-prompt, output-formatting issue, context-window benefit
  lost) that the CLI path causes.
- Synthesis frequency moves from daily-cron to interactive
  (e.g., "synthesize the 5 memories I just ingested" inline).
  At that point the per-cluster fork+exec cost matters in a way
  it does not for a daily batch.
- BL-010 (daemon) ships and the daemon design itself surfaces a
  "host LLM is right there" pain that the daemon's own
  ClaudeCliProvider cannot solve.

**If trigger fires, the design is McpSessionProvider, not memory_dream tool**:
Per F4. Add an `LlmProvider` impl that uses MCP sampling
(`sampling/createMessage`) to ask the host. Reuse `run_synthesis_pass`
as-is. Optionally expose a thin `memory_dream` tool that just calls
`run_synthesis_pass(.., &McpSessionProvider, ..)` — but that tool's
body is ~5 lines, not 100-150. The 100-150 LOC estimate in the stub
is itself evidence the stub is shaped wrong.

## Direct answer to framing questions

**Q: Is the shell-out indirection worth eliminating in MCP-attached?**
Not yet. No quantified pain. CLI path has 13 successful syntheses on
the production DB. The "67% residuals" problem the project flagged is
upstream of who the LLM is.

**Q: Boundary between mengdie and the LLM?**
Mengdie owns: cluster selection, prompt construction, response parsing,
persistence with cluster-hash + links, dedup, metrics. LLM owns: text
generation. This is **already** the boundary today — `LlmProvider` is
exactly this contract. BL-009 does not change the boundary; at most it
adds a second `LlmProvider` impl.

**Q: How many MCP tools, if yes-build?**
Zero. The right shape is `McpSessionProvider: LlmProvider` reusing
`run_synthesis_pass`. If a tool surface is needed for ergonomics
(e.g., trigger from inside Claude Code), it's a 5-line wrapper, not
a new write path.

**Q: How does the design interact with v0.8.5 cluster-hash NOT NULL?**
The CORRECT path (insert_synthesis_with_links) already complies. The
stub-as-written (Claude calls memory_ingest) violates it and must be
discarded. Any future BL-009 must route synthesis writes through
`insert_synthesis_with_links`, not through `memory_ingest`.

**Q: Failure mode design?**
The existing `run_synthesis_pass` already handles malformed JSON
(`parse_errors` counter), LLM call failure (`llm_call_errors`), and
the null-escape-hatch (`syntheses_llm_skipped`). McpSessionProvider
inherits all of this for free. Graceful degradation back to
ClaudeCliProvider is a config-level concern (which provider gets
constructed), not a synthesis-loop concern.

## Agreements (none yet)

Round 1 is independent research. No cross-agent agreements to record.

## Disagreements (none yet)

Round 1 is independent research. Anticipated disagreement points:

- Other agents may argue the symmetry argument IS sufficient ("we have
  the LLM, why pay for fork+exec"). I claim that needs a quantified
  pain or it's tooling-for-its-own-sake.
- Other agents may propose a yes-ship-minimum design (single tool, or
  two-tool round-trip). I would push back that the stub's two-tool
  shape is broken (F3) and that McpSessionProvider is strictly less
  machinery if MCP sampling works (F5).
- Other agents may argue BL-009 is on the critical path for BL-010+.
  I claim it isn't (F8).

## Open Questions

1. **Does rmcp v1.3 expose MCP sampling (`sampling/createMessage`) as
   a server-initiated client RPC?** If no, McpSessionProvider is
   infeasible and the only paths are (a) the stub's two-tool shape
   (broken per F3 unless we add a synthesis-aware commit surface) or
   (b) defer. I lean defer.
2. **Does Claude Code advertise the sampling client capability?**
   Even if rmcp supports it server-side, the host has to opt in.
   Empirical check needed.
3. **Has any user actually attempted `mengdie dream --synthesize`
   from inside a Claude Code session and hit a real wall?** If
   nobody has tried, "is the indirection painful" is hypothetical.
4. **What's the v0.9.0 sprint actually trying to deliver?** If it's
   the residuals fix + decay tuning, BL-009 isn't on the path.
   `/ae:roadmap plan v0.9.0` should not auto-include BL-009 just
   because it's the next stub in the file.

---

**Bottom line**: The minimum change is no change. Keep the CLI path,
file BL-009 with explicit triggers, let BL-010 proceed independently.
If a trigger fires, the right design is `McpSessionProvider`
(LlmProvider impl) reusing `run_synthesis_pass` — *not* a new
`memory_dream` tool with a second synthesis-write path. Discussion 008's
"extend, don't add" precedent applies directly.
