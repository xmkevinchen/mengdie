---
id: "02"
title: "Reflection trigger model — v0.0.1 default + ReflectionTrigger trait"
type: open-with-baseline
baseline: "v0.x cron logic shipped in dreaming.rs but com.mengdie.dream.plist is a template (placeholder path); 13 syntheses were produced via on-demand CLI invocation (mengdie dream --synthesize), NOT cron. Verified by archaeologist round-01."
status: converged
current_round: 2
created: 2026-05-05
decision: "On-demand as v0.0.1 default trigger. Introduce ReflectionTrigger trait (~50-80 LoC seam, mirrors LlmProvider pattern). Cron is a second impl plugged into the trait, deployed via operator launchd plist (1-paragraph setup doc). Salience / composite / debounced triggers are deferred BLs that slot into the same trait when filed."
rationale: "User decision after team split (4 trait vs 2 no-trait). Trait FOR (ai-engineer originator + codex + gemini + challenger): cheap insurance, mirrors existing LlmProvider abstraction, future BLs plug in cleanly. Trait AGAINST (system-architect + minimal-change-engineer): abstracts non-existent strategy gap at v0.0.1, fails Karpathy load-bearing test. Both sides agreed on-demand IS v0.0.1 default (cron NOT actually running per archaeologist plist verification). Synthesis embedding=None bug (archaeologist round-02:135-139) gates which trigger can be default — on-demand works now, cron-default would need the embedding fix."
reversibility: high
reversibility_basis: "Adding or removing a trait abstraction is a refactor; no data migration. The trait wraps existing dreaming.rs synthesis logic; impls are interchangeable."
---

# Topic: Reflection trigger model — v0.0.1 default

## Current Status
**CONVERGED Round 2 with user decision on split.** On-demand default + cron opt-in via `ReflectionTrigger` trait. Salience/composite/debounced filed as deferred BLs.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 4 "cron + on-demand both", 1 "on-demand+trait" (ai-engineer originator), 1 "on-demand only" (challenger). archaeologist: cron plist is template, 13 syntheses were on-demand CLI |
| 2 | split → user-decided | After R1 fact (cron not running), updates: codex/gemini absorbed trait pattern; challenger absorbed; system-architect+minimal-change rejected trait as premature abstraction. 4-vs-2 split escalated to user. User: trait now. |

## v0.x Baseline
A cron-based dreaming pass shipped via `docs/plans/010-dream-synthesis.md`
(2026 mid-April). The first real `mengdie dream --synthesize` run
produced 13 syntheses against the production DB (per CLAUDE.md
Project Status). Cron exists, runs nightly via macOS launchd, and
has produced output. **This topic is therefore not a from-scratch
"pick a trigger" question — it's "is cron alone the right v0.0.1
default, or does the loop need a finer-grained trigger?"**

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
"Reflection" in mengdie is the dreaming pass — clustering related
memories, asking the LLM to synthesize each cluster into a
higher-order memory, persisting the synthesis with provenance back
to the constituent atoms. The mechanism (cluster → synthesize →
store) is settled. The question is **when does it run?**

The literature offers several distinct trigger models:
- Cron / scheduled (mengdie v0.x: daily macOS launchd plist)
- Salience-threshold — fire when accumulated importance exceeds a
  threshold (Generative Agents, Park et al. 2023)
- Composite — entropy > 0.9 OR conflict density > 0.3 OR elapsed >
  1h (SCM, arxiv:2604.20943)
- Debounced submit-dedupe — every write enqueues, executor coalesces
  duplicates within a window (LangMem ReflectionExecutor)
- On-demand — only when explicitly requested via `mengdie dream`

Each trigger model produces different latency, freshness, and
operator-visibility tradeoffs. Each also has different coupling to
the rest of the system (cron is operator-managed; debounced needs an
in-process executor; on-demand needs an external scheduler that
isn't mengdie).

CLAUDE.md flags this as one of three deferred open questions. mem0
explicitly lists "reflection trigger that isn't cron or on-demand"
as unsolved in their state-of-memory-2026.

**Practical constraint for v0.0.1:** salience, composite, and
debounced triggers all require runtime metrics mengdie does not
yet compute (entropy, conflict density, write-event timing).
**Whether adding those metrics is tractable for v0.0.1 is an open
question Round 1 should answer.** The four candidates are:
- **cron** (already shipped, no new metrics needed)
- **on-demand** (no new metrics needed; operator-driven trigger)
- **salience-threshold** (Generative Agents pattern; requires
  per-memory importance scoring)
- **composite** (SCM pattern; entropy > 0.9 OR conflict density >
  0.3 OR elapsed > 1h — requires entropy + conflict-density
  computation)
- **debounced submit-dedupe** (LangMem pattern; requires
  write-event timing + executor)

If Round 1 finds metric-bearing triggers tractable for v0.0.1,
they remain candidates. If not, the defensible options narrow to
cron and on-demand; the rest may then be filed as follow-up BLs
with explicit triggers (e.g., "revisit when ingest volume
exceeds N/day").

## Constraints
- mengdie is stdio MCP server; in-process background executors must
  not block tool dispatch or hold the connection
- Reflection requires LLM calls (claude CLI subprocess); these are
  expensive (latency + token cost), so trigger must avoid trivial
  redundant runs
- Operator runs mengdie locally on macOS; cron / launchd is
  deployment-feasible but adds another surface to monitor
- Existing v0.x code: `dreaming.rs` runs the synthesis logic;
  trigger orchestration lives in CLI + launchd plist
- The chosen default must be defendable empirically (the literature
  contests this; mengdie cannot ship "we picked one because the
  paper said so")

## Key Questions
- What is the actual symptom of "wrong trigger" — stale memories
  outdated by hours? Redundant LLM spend on identical clusters?
  Both?
- Which trigger models are observable from the operator's seat
  (i.e., the operator can tell whether reflection is firing too
  often / not often / on the wrong things)?
- Is a single default sufficient or does mengdie ship one default
  + one or two additional triggers as opt-in (per blueprint §8 phrasing
  "pick one as v0.0.1 default; others remain triggers for evolution")?
- Composite triggers depend on entropy / conflict-density metrics —
  does mengdie already compute these, or do they require new
  instrumentation that hasn't been built?
- If salience-based: how is salience measured for AE pipeline
  artifacts (which carry no like/star/highlight signal)?
- What does the operator actually want to control — when reflection
  fires, or what reflection produces?
