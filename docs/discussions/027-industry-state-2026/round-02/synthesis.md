---
round: 02
date: 2026-05-05
synthesis_type: orientation
agents: [system-architect, archaeologist, ai-engineer, minimal-change-engineer, codex-proxy, gemini-proxy, challenger]
---

# Round 2 — TL synthesis

**Purpose**: orientation index for next phase (Step 4 Consensus Verification or direct Step 5 TL scoring). Per-agent files are the primary artifact. This synthesis names where positions sit AFTER Round 2 cross-examination, what changed, and where genuine splits remain.

## Cross-examination outcomes — position updates per agent

Round 2 was a successful cross-examination round: **5 of 7 agents updated 3+ positions** based on Round 1 evidence (especially archaeologist's verified facts).

| Agent | Positions updated in R2 |
|---|---|
| codex-proxy | T2 (cron+on-demand → on-demand+trait), T4 (fwd-compat → no-fwd-compat), T5 (search utilization → ai-engineer's nonempty rate) — **3 updates** |
| gemini-proxy | T1 (hybrid → push-only), T2 (cron+on-demand → on-demand+trait), T4 (fwd-compat → no-fwd-compat), T5 (thumbs → retracted) — **4 updates** |
| challenger | T1 (pull → push, accepting `resolves` atomicity), T3 (cross-project default → ratify with new rationale), T4 (extraction-discipline → ratify with `Manual` enum), T5 (citation rate → lock-compliant nonempty + synthesis-influence rate) — **4 updates** |
| ai-engineer | T2 strengthened with archaeologist evidence; T4 absorbed challenger's reframe — **2 updates** |
| system-architect | T2 (cron+on-demand → on-demand only, REJECTS trait), T4 (AE-files → extraction-discipline), T5 (2 metrics → 3 metrics) — **3 updates** |
| minimal-change-engineer | T2 (cron+on-demand → cron+15-min operator config, REJECTS trait), T5 confirmed lock-compliant — **2 substantive holds + 2 reinforcements** |
| archaeologist | Round 2 = empirical verification, not position-taking |

## Per-topic landscape AFTER Round 2

### Topic 1 — Ingest mechanism — **CONVERGED**

| Verdict | Agents | Evidence |
|---|---|---|
| Push-primary, watcher kept as opt-in library (NOT wired) | All 6 position-taking agents (system-architect, ai-engineer, minimal-change-engineer, codex-proxy, gemini-proxy, challenger-updated) | `resolves` atomicity is push-only by construction (system-architect.md:122-126); `mengdie import` covers cold-start (cli.rs:361 verified by archaeologist.md:43-48); pull-fallback "AE not running" produces no inputs anyway; mem0/LangMem industry pattern |

**Genuine convergence**, not groupthink — challenger updated based on a specific design-merit argument from system-architect, not pressure. Skip Step 4 verification; mark converged.

### Topic 2 — Reflection trigger — **SPLIT (4 vs 2)**

| Verdict | Agents |
|---|---|
| **FOR** trait pattern: on-demand default + cron opt-in via `ReflectionTrigger` trait (~50-80 LoC seam, mirrors `LlmProvider`) | ai-engineer (originator), codex-proxy (R2), gemini-proxy (R2), challenger (R2) |
| **AGAINST** trait pattern: on-demand default + cron opt-in WITHOUT trait abstraction. Cron is operator-config (15-min launchd plist + doc), not new code | system-architect (R2), minimal-change-engineer (R2) |

**Both sides agree**:
- On-demand IS the v0.0.1 default (cron NOT actually running per archaeologist.md:74-77)
- Cron logic exists in `dreaming.rs`; deployment is the gap, not implementation
- Salience / composite / debounced are deferred BLs (require runtime metrics not yet computed)
- Synthesis embedding=None (archaeologist.md:135-139) is a separate P1 BL that gates which trigger can be default (on-demand can ship now; cron-default needs the embedding fix)

**Disagreement is substantive**:
- TRAIT side: trait is cheap insurance; mirrors existing `LlmProvider` pattern; salience/composite/debounced can plug in via the same trait when filed as BLs
- NO-TRAIT side: trait abstracts over a non-existent strategy gap (one real impl); fails Karpathy load-bearing test; "scope creep dressed as engineering discipline"; file the trait with the future BLs that actually need it

**Reversibility**: HIGH. Adding a trait later if salience/composite ships is straightforward; removing trait if not needed is also straightforward.

### Topic 3 — Cross-project scope (ratify check) — **CONVERGED**

| Verdict | Agents | Evidence |
|---|---|---|
| Ratify §5 per-project default | All 7 (challenger updated) | ai-engineer's cluster contamination argument (round-01:476-494); §5's original migration-cost rationale superseded by contamination-risk rationale (challenger R2 update); empirically 1-line diff to flip per archaeologist.md (mcp_tools.rs:192-195); 028's no-ACK lock further constrains cross-project default |

**Refinements integrated**:
- challenger R2: rationale change in conclusion artifact ("contamination risk outweighs cross-project recall benefit", NOT "avoid migration cost")
- system-architect R2: AE skills should specify `scope` per skill explicitly (default-when-omitted is fallback, not load-bearing surface)
- minimal-change-engineer R2: `cwd`-stale `project_id` bug (mcp_server.rs:32-34) is separate BL (independent of T3 ratify)
- gemini-proxy R2: cross-project synthesis explicitly defer to P2 with trigger

Skip Step 4 verification; mark converged.

### Topic 4 — Ingest source boundary (ratify check) — **SPLIT (3 vs 3)**

| Verdict | Agents |
|---|---|
| **FOR** AE-files-only physical boundary, NO fwd-compat scaffolding (the "ratify" reading) | codex-proxy (R2), gemini-proxy (R2), minimal-change-engineer (R1+R2 reinforced) |
| **FOR** extraction-discipline, NOT AE-files-only — keep existing `source_type` enum (typed marker), don't enforce file-source physically (the "extraction-discipline" reading) | system-architect (R2), challenger (R1+R2), ai-engineer (R2) |

**Both sides agree**:
- Ratify the spirit of "AE-only" — mengdie should not become a generic memex
- No new forward-compat scaffolding (typed source markers beyond what already exists)
- Latent bug exists: `is_ingestable` blocklist lets non-AE files through; v5 trigger rejects "unknown" source_type — fix is required regardless of which reading wins

**Disagreement is substantive**:
- AE-FILES-ONLY: enforce physical boundary at parser; tighten allowlist; reject non-AE files at MCP boundary with clear error. Bug fix = `~25 LoC tighten parser + error message`. Reopening trigger only: ≥3 high-value facts unfit for AE per quarter.
- EXTRACTION-DISCIPLINE: existing `source_type` enum is the typed marker; AE-discipline is a quality gate not a physical-source restriction. Bug fix = "rename `unknown` → `direct` in source_type enum"; ad-hoc facts are valid IF caller asserts AE-style structured extraction.

**Reversibility**: MEDIUM. Switching post-ship requires data migration if memories were ingested under one stance and need re-classification. Both sides aren't differing on storage shape — they differ on what the ingest API enforces, which affects callers downstream.

### Topic 5 — Loop-closure signal — **MOSTLY CONVERGED, minor metric variations**

| Verdict | Agents |
|---|---|
| Primary signal: per-search nonempty rate (F-002, no new schema, lock-compliant) + ae:retrospect qualitative | All 7 converge on this primary shape |
| Additional signals (variations) | system-architect adds contradiction-trend; minimal-change adds zero-row-days; ai-engineer adds explicit falsification rule (nonempty <20% over 14d AND two idk verdicts → loop not delivering); challenger keeps synthesis-influencing-search rate as lock-compliant secondary |

**028 no-ACK lock** verbatim from archaeologist.md:142-148: "MCP `memory_search` ACK feedback channel — NO in v0.0.1 contract. All Topic 4 triggers must be server-side observable from the persisted domain audit table."

**What's ruled out by the lock**:
- gemini-proxy R1's thumbs up/down per result (retracted in R2)
- Any "search-result-cited" signal that requires AE-side ACK to mengdie
- F-002 schema extension to add `cited_at` column

**What's lock-compliant**:
- nonempty rate (server-side from audit_returned_facts)
- synthesis-influencing-search rate (server-side join of audit_returned_facts × memory_entries.source_type)
- contradiction-trend (server-side count of contradiction events)
- zero-row days, repeat-query density (server-side aggregation)

**Synthesis embedding=None gates synthesis-influencing-search**: archaeologist.md:135-139 verified `embedding=None` at synthesis creation; clustering.rs:71-79 filters synthesis from clustering. If shipped, synthesis-influencing-search would systematically under-count. → file as P1 BL gated on synthesis-embedding fix.

**Convergence is genuine**, not groupthink — agents propose slightly different metric mixes for different reasons, but all agree on the substrate (F-002 audit table) and shape (one quantitative + one qualitative). Skip Step 4 verification on the primary shape; the metric variation can be resolved in Step 5 TL scoring.

## 4 Mandatory Synthesis Fields

### 1. Pruned

- **gemini-proxy R1 hybrid push+pull → push-only**: pruned in R2 by gemini themselves on cross-exam.
- **codex-proxy R1 fwd-compat for T4 → no-fwd-compat**: pruned by codex on cross-exam.
- **codex-proxy R1 thumbs/cited-rate signals → lock-compliant alternatives**: pruned by archaeologist verification of 028 lock.
- **gemini-proxy R1 thumbs up/down per result**: explicitly retracted by gemini after archaeologist confirmed 028 lock language.
- **challenger R1 pull-default for T1**: pruned after system-architect surfaced `resolves` atomicity as design-merit (not execution gap).
- **challenger R1 cross-project as default for T3**: pruned after ai-engineer's cluster contamination argument.
- **challenger R1 "AE-only as extraction discipline" — partially absorbed**: 3 agents now hold extraction-discipline reading (system-architect, ai-engineer, challenger themselves). 3 maintain AE-files-only. Not pruned — it became a real split.
- **All R1 "cron + on-demand both shipped" claims (codex, gemini, system-architect, minimal-change)**: pruned by archaeologist verification of plist-template + on-demand-CLI-only origin of 13 syntheses. New baseline: cron logic exists; trigger NOT operationally wired.

**Convergence pruning candidates** (recorded but pending Step 5 TL decision):
- T4 reading split: 3 vs 3 — needs decision, not further pruning.
- T2 trait split: 4 vs 2 — majority direction, but minority's Karpathy load-bearing argument is substantive.

### 2. Of-framing disposition

Frame-challenges raised in Round 1 + their disposition after Round 2 cross-examination:

| Challenge (R1 source) | TL R2 disposition |
|---|---|
| T1 push-favoring is v0.x execution gap, not design merit (challenger R1) | RESOLVED — challenger R2 updated after system-architect surfaced `resolves` atomicity as design-merit. |
| T2 cron is NOT actually running (archaeologist R1, factual) | INTEGRATED — synthesis baseline updated; "cron + on-demand both shipped" claims pruned across 4 agents in R2; on-demand established as v0.0.1 default. |
| T3 §5 was migration-cost deferral, disappears in rebuild (challenger R1) | PARTIALLY INTEGRATED — challenger R2 ratified per-project but with rationale change (contamination risk, not migration cost). The conclusion artifact must use the new rationale. |
| T4 "AE-only" should mean extraction discipline (challenger R1) | UNRESOLVED — 3 agents (system-architect, challenger, ai-engineer) absorb the reframe; 3 (codex, gemini, minimal-change) reject it. Step 5 TL decision required. |
| T4 latent bug: AE-only is policy not enforcement (archaeologist R1, factual) | INTEGRATED — both readings agree the bug must be fixed; they differ on HOW (parser tighten vs source_type rename). |
| T5 Goodhart's Law on count-based metrics (challenger R1) | PARTIALLY RESOLVED — challenger R2 updated to inverse-gameable signals (empty-rate, zero-rows = HIGH=BAD); minimal-change confirms "Goodhart dissolves under inverse-gameable signals." |

### 3. Verification artifacts

Round 2 produced these verification artifacts cited across agents:

- **archaeologist round-01.md** (R1) and **round-02.md** (R2) — empirical baseline, all code-level claims grounded with file:line
- **028 no-ACK lock** verified verbatim by archaeologist R2 + minimal-change R2: `docs/discussions/028-v0.0.1-architecture-design/conclusion.md:22-27` — text reproduced in archaeologist.md:142-148
- **`com.mengdie.dream.plist` is a template** (placeholder path) — verified by archaeologist R1; reproduced in archaeologist.md:74-77
- **`watcher.rs` zero call sites** outside tests — verified by archaeologist R1 grep
- **`mcp_tools.rs:192-195` per-project conditional, 1-line diff** to flip — verified
- **`cmd_import` cold-start works** for AE-named files (cli.rs:361-424); rejects "unknown" source_type for non-AE-named files — verified by archaeologist R2
- **`embedding=None` at synthesis creation** (`dreaming.rs:569-570`); zero re-embedding pass anywhere — verified by archaeologist R2 grep; clustering.rs:71-79 SQL filter excludes synthesis
- **`mcp_server.rs:32-34`** — project_id one-time inference at startup, stale on cwd-switch — verified

Claims that remain UNVALIDATED:
- "ChatGPT Memory's actual reflection trigger pattern" (codex R1 cited but not independently verified) — affects T2 reasoning weight
- "Google Drive integration pattern for hybrid push+pull" (gemini R1) — gemini itself updated R2 to drop this position
- ai-engineer's "~50-80 LoC for ReflectionTrigger trait" estimate — not implementation-verified

### 4. Frame-challenge disappearance self-check

Comparing R1 of-framing markers (synthesis.md §2 row table) against R2 outputs:

| R1 Frame-challenge | R2 status | Silent disappearance? |
|---|---|---|
| T1 push favoring v0.x execution gap | Resolved (challenger R2 update with explicit reasoning) | NO — explicitly resolved |
| T2 cron NOT running | Integrated across 4 R2 agent updates | NO — explicitly integrated |
| T3 §5 deferral disappears in rebuild | Partially integrated (rationale change recorded in challenger R2) | NO — explicitly partially integrated |
| T4 extraction discipline reframe | Real split (3 vs 3 in R2) | NO — became substantive disagreement |
| T4 latent bug | Integrated (both readings agree fix required) | NO — explicitly integrated |
| T5 Goodhart's Law | Partially resolved (inverse-gameable signals) | NO — explicitly partially resolved |

**No silent disappearance detected.** All R1 frame-challenges have explicit R2 disposition.

## Decision routing for next phase

| Topic | Status | Recommended next step |
|---|---|---|
| T1 push-primary | CONVERGED 7/7 | Step 5 TL score = `converged` |
| T2 trait vs no-trait | SPLIT 4 vs 2 | Step 5 TL decision OR escalate to user (high reversibility = TL can decide) |
| T3 ratify per-project | CONVERGED 7/7 (with rationale refinement) | Step 5 TL score = `converged` |
| T4 extraction-discipline vs AE-files-only | SPLIT 3 vs 3 | Step 5 TL decision OR escalate to user (medium reversibility = user input valuable) |
| T5 F-002 nonempty + ae:retrospect | CONVERGED on shape; metric variations | Step 5 TL score = `converged` (resolve metric variations in TL synthesis) |

Step 4 Consensus Verification (Debate Mode) is **not warranted** — the splits aren't groupthink and another forced FOR/AGAINST round won't produce new evidence beyond what R1+R2 surfaced. Per spec Step 4 "Skip when ... agents independently reached the same conclusion with strong evidence from different angles (genuine convergence, not groupthink)" — the splits are genuine disagreement on narrowly-scoped engineering preferences, both sides have argued from evidence, and another round of forced stances would be ceremony.

Recommend Step 5 TL scoring on T1, T3, T5 + escalation to user on T4 (medium reversibility). T2 has high reversibility, but the trait/no-trait split is a meaningful design discipline question worth user input as well.

The Sweep step (Step 7) will then resolve any deferred items including the BLs identified across Round 1+2:
- BL: synthesis embedding=None re-embedding pass (gates cron-default + synthesis-influencing-search metric)
- BL: project_id cwd-switch staleness fix
- BL: T4 latent bug fix (`unknown` source_type rejection — exact form depends on T4 decision)
- BL: salience-trigger feasibility study (deferred per all agents)
- BL: composite-trigger feasibility study (deferred per all agents)
- BL: debounced-trigger feasibility study (deferred per all agents — also requires daemon shape)
