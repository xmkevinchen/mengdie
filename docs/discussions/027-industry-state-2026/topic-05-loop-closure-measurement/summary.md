---
id: "05"
title: "Loop-closure signal — F-002 nonempty rate + ae:retrospect"
type: open
status: converged
current_round: 2
created: 2026-05-05
decision: "Two-signal v0.0.1 minimum, both 028 no-ACK lock-compliant: (1) Quantitative — per-search nonempty rate computed from F-002 audit_returned_facts table (server-side aggregation; no schema changes; no new ACK channel). (2) Qualitative — ae:retrospect hook with prompt 'did mengdie short-circuit anything this week? y/n/idk' + falsification rule: nonempty rate <20% over 14 days AND two consecutive idk verdicts → loop NOT delivering value, escalate. Surfacing: mengdie audit-stats CLI subcommand (BL-014 already filed). Optional secondary signals (file as P1 BLs, gated on synthesis-embedding fix): synthesis-influencing-search rate, contradiction-trend, zero-row-days, repeat-query-density."
rationale: "Genuine convergence on substrate (F-002 audit table) and shape (one quantitative + one qualitative). 028 no-ACK lock verified verbatim by archaeologist+minimal-change at conclusion.md:22-27 — rules out gemini's thumbs up/down (retracted R2), codex's cited-rate (retracted R2), challenger's R0-citation rate (retracted R2). Inverse-Goodhart property: nonempty rate gaming = correct mengdie usage (you must ingest more high-quality facts to raise the score). Falsification rule from ai-engineer round-01 (Perplexity 77→95% recall by storing half as many memories — mengdie should fire a tripwire if it's going the wrong way). Synthesis embedding=None bug (archaeologist round-02:135-139) gates synthesis-influencing-search secondary signal."
reversibility: high
reversibility_basis: "Signals are read-only computed views over existing F-002 audit data. Adding/removing metrics is non-breaking. Falsification thresholds (20%, 14 days, 2 verdicts) can be tuned post-ship without data migration."
---

# Topic: Loop-closure signal — quantitative or qualitative

## Current Status
**CONVERGED Round 2.** Two-signal: F-002 nonempty rate + ae:retrospect qualitative + falsification rule. mengdie audit-stats CLI (BL-014).

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | Multiple signal proposals: F-002 nonempty (ai-engineer), F-002 multi-metric (system-architect, minimal-change), thumbs (gemini), citation rate (codex, challenger). archaeologist: F-002 not exposed via MCP, no "cited" signal in schema |
| 2 | converged | 028 no-ACK lock verified at conclusion.md:22-27. Thumbs/citation-rate retracted as lock-violating. Convergence on F-002 nonempty rate + ae:retrospect + ai-engineer's falsification rule. Secondary metrics (synthesis-influence, contradiction-trend, zero-rows) filed as P1 BLs, some gated on synthesis-embedding fix. |

## Context
mengdie's CLAUDE.md core loop: "AI tools produce knowledge →
Mengdie ingests and filters → feeds context back to AI tools →
better output → richer knowledge → spiral upward."

The blueprint §5 P0 includes "basic instrumentation." This topic
narrows P0 to a concrete question: **what's the minimum signal —
quantitative or qualitative — that confirms the loop is actually
delivering value, not just being called?**

Both modalities are in scope:
- **Quantitative** — counters / metrics derived from F-002 audit
  table or new event streams (e.g., search-result-cited rate,
  contradiction-detected count, re-research time delta).
- **Qualitative** — operator-driven signals (e.g., explicit
  "useful / not useful" marks on retrieval results, periodic
  retrospective verdict on whether the loop felt productive last
  week).
- **Hybrid** combinations are explicitly permitted.

Without measurement, "the loop is closing" is opinion, not fact.
analysis.md "Industry Practice Comparison" point 7 names this as
mengdie's specific concern — no OSS framework instruments
solo-operator-scale loop closure.

What "value" can plausibly mean in this context:
- Mengdie-injected facts (from ae:analyze Round 0) actually
  influence the AI agent's output (cited, contradicted, or
  extended) — vs being injected and ignored
- Subsequent decisions that conflict with prior decisions
  (contradiction-detection events) trend down over time
- Re-research time on previously-discussed topics shrinks (mengdie
  short-circuits the rediscovery)
- ae:retrospect cycles produce fewer "we already discussed this in
  conclusion N" findings

Each of these is an empirical signal but they require different
instrumentation and have different signal-to-noise.

## Constraints
- Solo-operator scale: instrumentation must not require external
  log aggregation, dashboards, or SaaS observability
- Must be cheap to compute (real-time or per-day batch); cannot
  require LLM calls per measurement
- Must produce a signal the operator actually checks — a metric
  that lives only in `~/.mengdie/` and never gets read is not a
  measurement
- F-002 (audit table, recently shipped) provides per-search
  query/scope/took_ms + per-call returned fact IDs as durable
  audit trail — this is foundational instrumentation but is not
  yet a loop-closure metric
- Blueprint §3.3 lists outputs (search results, contradictions,
  syntheses) but not instrumentation surfaces

## Key Questions
- What's the minimum signal set (one or two items) that is both
  *cheap to produce* and *forces the operator to confront whether
  the loop is working*? (One forced signal beats five ignored
  metrics or unread feedback prompts.)
- Where does the signal live — `mengdie stats`, an MCP tool the
  operator runs interactively, a daily report, a launchd-driven
  dump?
- What's the falsification path — what observation would prove
  "the loop is NOT closing for me right now" so the operator
  can stop / fix / abandon rather than burn cycles believing in
  it?
- Is there a baseline period needed (e.g., first month is data
  collection, after that the metric becomes meaningful) and how
  is "we're still in baseline" represented?
- Can F-002's audit table be the measurement substrate, or does
  loop closure need a separate event stream (search-result-used
  / search-result-ignored / contradiction-fired / synthesis-used)?
- Are there OSS / academic precedents for this measurement — or is
  this genuinely new ground (analysis.md says new ground, but the
  team should challenge that)?
