---
id: BL-024
title: "Reflection trigger feasibility — salience / composite / debounced as ReflectionTrigger trait impls beyond v0.0.1"
status: open
created: 2026-05-05
origin: "discussion 027 conclusion (Topic 2) — three trigger candidates explicitly deferred"
trigger: "Per-trigger conditions below — each candidate has its own gating condition"
depends_on: ["Topic 2 ReflectionTrigger trait shipped in v0.0.1"]
size: M-L (depending on which trigger fires)
v_target: "Post-v0.0.1 — file the corresponding feasibility study + impl when its trigger fires"
---

# BL-024 — Salience / composite / debounced trigger feasibility

## Origin

Discussion 027 Topic 2 settled on **on-demand as v0.0.1 default trigger** + **`ReflectionTrigger` trait** as the v0.0.1 abstraction (4-vs-2 split escalated to user; user chose trait). Three additional trigger candidates were evaluated and explicitly deferred because they require runtime metrics mengdie does not yet compute:

- **Salience-threshold** (Generative Agents pattern, Park et al. 2023, arxiv:2304.03442) — fire when accumulated importance exceeds a threshold. Requires per-memory importance scoring. ai-engineer's verdict: "violates AE-discipline" (LLM-per-ingest re-processing).
- **Composite** (SCM pattern, arxiv:2604.20943) — entropy > 0.9 OR conflict density > 0.3 OR elapsed > 1h. Requires entropy + conflict-density computation. ai-engineer's verdict at v0.0.1's 214-memory scale: "entropy estimator is statistically meaningless."
- **Debounced submit-dedupe** (LangMem ReflectionExecutor pattern) — every write enqueues, executor coalesces duplicates within a window. Requires write-event timing + an in-process executor. system-architect's note: "debounced requires a daemon — a P0 infrastructure shape change, not a Topic 2 decision."

## Why deferred (not rejected)

The `ReflectionTrigger` trait shipped in v0.0.1 (per Topic 2 user decision) provides the seam for these three impls. Each can land as its own commit when its trigger fires. They are NOT permanently rejected — they're filed pending the empirical conditions that would make them defensible.

## Per-trigger fire conditions

- **Salience-threshold**: when AE pipeline produces structured importance signals at ingest time (not LLM re-classification). E.g., AE plugin tags conclusion facts as "high-importance" vs "incidental" — that signal could feed a salience threshold without re-processing.
- **Composite**: when corpus size exceeds ~5,000 memories (entropy estimator becomes statistically meaningful) AND F-002 audit data shows on-demand triggers are insufficient (e.g., operator forgets to run `mengdie dream`, syntheses fall behind).
- **Debounced**: when v0.0.1 architecture admits a long-running daemon (out of scope per discussion 028 conclusion + Topic 1 push-primary decision). Effectively requires a post-v0.0.1 shape change.

## Implementation sketch (deferred, per-trigger)

- Each trigger implements the `ReflectionTrigger` trait (~50-80 LoC seam shipped in v0.0.1).
- Salience needs a metric storage layer (per-memory importance scores).
- Composite needs entropy + conflict-density computations against the memory corpus.
- Debounced needs an in-process executor + write-event channel + the daemon shape change.

## Acceptance criteria (when filed)

- Trigger impl plugs into the `ReflectionTrigger` trait without modifying call sites
- Empirical justification: trigger fires under the documented condition + does NOT regress on-demand-default behavior
- Documented in `docs/discussions/<followup>/conclusion.md` if it requires its own design discussion

## Trigger

This BL is a placeholder for three follow-on BLs that get split out when individual triggers fire. File them separately at that point — this BL is a record of "we considered these and explicitly deferred them per documented conditions."
