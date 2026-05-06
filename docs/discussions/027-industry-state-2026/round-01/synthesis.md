---
round: 01
date: 2026-05-05
synthesis_type: orientation
agents: [system-architect, archaeologist, ai-engineer, minimal-change-engineer, codex-proxy, gemini-proxy, challenger]
---

# Round 1 — TL synthesis

**Purpose**: orientation index for Round 2. Per-agent files are the primary artifact. This synthesis names where positions sit, where evidence converges, and where Round 2 must cross-examine. Do NOT derive arguments from this file — read the per-agent round-01 files directly.

## Per-topic landscape

### Topic 1 — Ingest mechanism

| Verdict | Agents | Key reasoning |
|---|---|---|
| Push-primary (watcher kept as opt-in library, NOT wired) | system-architect, ai-engineer, minimal-change-engineer | resolves atomicity is push-only; cold-start via `mengdie import` already exists; pull/hybrid is reinvention failure mode |
| Push-primary (with explicit deletion of watcher dead code OR keeping it) | codex-proxy | mirrors mem0/LangMem/Graphiti convergence + Vector Stores API |
| Hybrid push-primary + pull-fallback | gemini-proxy | Google Drive integration pattern |
| Pull-default | challenger | "watcher never wired" is execution gap, not design flaw; daemon cost marginal |

**Convergence**: 4 push-primary, 1 hybrid (also push-primary), 1 pull. Direction is push.

**Round 2 cross-examination needed**:
- Does watcher.rs stay as opt-in library or get deleted entirely? (system-architect + minimal-change agree keep; archaeologist confirms zero call sites; deletion is a separate dead-code question)
- Hybrid vs push-only — does keeping pull-fallback for "AE not running" environments earn its keep, or is `mengdie import` sufficient?
- Challenger's pull-default position deserves cross-exam: if pull is genuinely sounder, archaeologist's "watcher unwired" only weakens it as a v0.x execution failure; the design merits stand independently.

### Topic 2 — Reflection trigger

| Verdict | Agents | Key reasoning |
|---|---|---|
| On-demand default + cron opt-in via `ReflectionTrigger` trait (~80 LoC) | ai-engineer | salience needs LLM-per-ingest (violates AE-discipline); composite entropy meaningless at 214-memory scale; debounced needs daemon (P0 infra shape change) |
| Cron + on-demand (both v0.0.1 defaults) | codex-proxy, gemini-proxy, system-architect, minimal-change-engineer | both already exist (per ai-engineer's claim); file salience/composite/debounced as deferred BLs with triggers |
| On-demand only (cron is sunk-cost reasoning) | challenger | 13 syntheses = output count not quality; cron's burst-session temporal mismatch is structural problem (SCM) |

**Critical Round 1 verification artifact** (archaeologist): `com.mengdie.dream.plist` is a TEMPLATE (path placeholder `<!-- Update ... -->`). The "13 syntheses" first run was on-demand CLI, NOT cron. Therefore:
- The framing's premise that "cron-based synthesis is already running" is **empirically false**.
- "Cron + on-demand both shipped" claims by codex/gemini/system-arch/minimal-change need to be re-examined: cron is half-shipped (logic exists in dreaming.rs; trigger NOT wired).
- challenger's "cron is sunk-cost reasoning" position gains weight from this fact.
- ai-engineer's trait-based proposal becomes more attractive: it codifies on-demand-as-default while keeping cron pluggable (matching real v0.x state, not aspirational state).

**Additional verification artifact** (archaeologist): synthesis rows are stored with `embedding=None` — they cannot participate in vector search. Independent of trigger choice, this affects whether reflection's output is queryable. Round 2 should surface whether this is a Topic 2 concern or a separate BL.

**Round 2 cross-examination needed**:
- Given cron is NOT actually running, does "cron + on-demand both v0.0.1 defaults" still make sense, or does ai-engineer's on-demand-default + cron-opt-in (trait) become the natural answer?
- Should the synthesis-embedding bug surface as a Topic 2 dependency (synthesis must be queryable for "synthesis-influencing-search" Topic 5 metric to work)?

### Topic 3 — Cross-project scope (ratify check)

| Verdict | Agents | Key reasoning |
|---|---|---|
| Ratify §5 (per-project default) | codex-proxy, ai-engineer, gemini-proxy, system-architect, minimal-change-engineer | §5 commitment holds; minor variations on reopening trigger spec (≥30% global opt-in / 10% global queries / 3 retro incidents) |
| Cross-project should be DEFAULT, per-project opt-in | challenger | Single operator unified identity across projects; §5 deferral was migration-cost, not design — disappears in fresh rebuild |

**Convergence**: 5 ratify, 1 contrarian. Strong direction.

**Verification artifact** (archaeologist): per-project default is a 4-line conditional in `mcp_tools.rs:192-195`; storage is already global; changing default = 1-line diff. Both directions are cheap to implement; the question is which is correct.

**Round 2 cross-examination needed**:
- Challenger's "single operator unified identity" framing: if true across projects, what's the failure mode of cross-project default? (Cross-contamination risk: a project-A-specific decision incorrectly applied to project B.)
- Reopening trigger: which formulation is testable from F-002 data alone? Each reviewer suggested a slightly different metric.
- archaeologist also surfaced `project_id` staleness on cwd-switch — separate bug; integrate or defer?

### Topic 4 — Ingest source boundary (ratify check)

| Verdict | Agents | Key reasoning |
|---|---|---|
| Ratify AE-only, NO forward-compat scaffolding | minimal-change-engineer, system-architect (YAGNI) | forward-compat is reinvention failure mode; reopening trigger only |
| Ratify AE-only, WITH forward-compat (typed source markers, per-source filters) | codex-proxy | bake in forward-compat from v0.0.1 to avoid v1 API break |
| Ratify AE-only | ai-engineer (Perplexity admission-filtering grounds), gemini-proxy (NotebookLM precedent) | strict boundary is strategic; expandable post-v0.0.1 |
| "AE-only" should mean AE extraction discipline (quality gate), NOT physical AE files | challenger | ad-hoc debug-session facts are load-bearing edge case |

**Verification artifact** (archaeologist): AE-only is **policy not enforcement**. `memory_ingest` accepts any text. `infer_source_type` returns "unknown" for non-AE filenames which v5 schema trigger REJECTS — this is a latent bug for non-AE files passing through file-ingest path. So Topic 4 has a code defect intersecting with the design decision.

**Round 2 cross-examination needed**:
- Forward-compat: minimal-change calls it scope creep / reinvention failure; codex calls it cheap insurance against v1 API break. Which is right for v0.0.1?
- Latent bug: how does the ratify decision constrain the bug fix? (If AE-only is enforced, the rejection is correct behavior — but error message is wrong. If broader, the rejection becomes a real bug.)
- Challenger's "AE-only = extraction discipline" reframe: is there a separable proposal here (mengdie accepts any source IF it carries AE-style structured extraction provenance), or is this just rejecting the topic?

### Topic 5 — Loop-closure signal

| Verdict | Agents | Key reasoning |
|---|---|---|
| Per-search nonempty rate (F-002) + ae:retrospect qualitative + falsification rule | ai-engineer | computable from existing F-002 audit data, no new schema; concrete falsification rule (nonempty < 20% over 14d AND two "idk" retrospect verdicts → loop not delivering) |
| F-002 audit table + `mengdie audit-stats` CLI + ae:retrospect | minimal-change-engineer | empty-result rate / repeat-query density / zero-row days; BL-014 already filed; ACK feedback is scope creep against 028's locked "no ACK" |
| Two F-002-derived metrics: search-with-results-rate + synthesis-influencing-search rate | system-architect | surfaced via `mengdie stats` + inlined in `IngestOutput`; no separate event stream |
| Search utilization rate (F-002) + operator retro verdict | codex-proxy | two-signal minimum; medium-high confidence |
| Thumbs up/down on every result + forced weekly stats report | gemini-proxy | prevents invisible failure |
| Contradiction-detection trend + Round 0 citation rate (hard-to-game) | challenger | Goodhart's Law — search-count and synthesis-count are gameable proxies |

**Convergence**: 5 of 7 propose F-002-based quantitative + ae:retrospect / operator retro qualitative. Disagreement on which specific F-002 metric.

**Verification artifact** (archaeologist): F-002 audit table (`memory_search_audit` + `audit_returned_facts`, schema v6) records query/scope/took_ms/searched_at/returned-fact-IDs. NOT exposed via MCP tools (no `memory_audit_query`). CLI `stats` reads `metrics` table only. **No "was this fact cited?" signal exists in the schema.** No ae:analyze vs manual-search distinction.

**Round 2 cross-examination needed**:
- Which specific F-002-derived metric? nonempty rate (ai-engineer) vs synthesis-influencing-search rate (system-architect) vs empty-result rate (minimal-change). All three are computable from existing schema.
- Synthesis-influencing-search rate requires search-result-cited signal which archaeologist confirms is NOT in F-002 schema. Does adding it count as "minimum" signal or scope creep against 028's "no ACK feedback" lock?
- Gemini's thumbs up/down per-result is a per-result ACK signal — falls foul of 028's no-ACK lock per minimal-change. Round 2 should resolve.
- Challenger's Round 0 citation rate is structurally similar to system-architect's synthesis-influencing-search rate — both require AE-side hooks that don't exist.

## 4 Mandatory Synthesis Fields

### 1. Pruned

- **The framing's premise that "v0.x shipped a cron-based dreaming pass" is empirically falsified**. Archaeologist verified the plist is a template; "13 syntheses" was on-demand CLI. Pruned: any Round 2 reasoning that assumes cron is the established baseline. Replacement: cron is half-shipped (logic ships in dreaming.rs, trigger NOT wired). On-demand IS the established baseline.
- **No agent positions pruned yet** — Round 1 was independent research; cross-examination occurs in Round 2.
- **Convergence pruning candidates** (recorded but not yet pruned, pending Round 2 cross-exam):
  - Challenger's "pull-default" (T1) — 1 of 7; if cross-exam doesn't surface new evidence, prune
  - Challenger's "cross-project as default" (T3) — 1 of 7; same
  - Gemini's "thumbs up/down per result" (T5) — conflicts with 028's no-ACK lock per minimal-change; if confirmed, prune
- **No frame-challenges silently disappeared** (this is Round 1; no prior round to compare).

### 2. Of-framing disposition

Challenges raised against the framing during Round 1 + TL disposition:

| Challenge | Source | TL disposition |
|---|---|---|
| T1 framing implicitly favors push due to v0.x execution gap, not design merit | challenger | Integrate into Round 2 cross-exam — push of 4-of-7 is direction, not anchor; design-merit cross-exam is required |
| T2 cron is NOT actually running (framing claimed "shipped") | archaeologist (factual) | **Integrate immediately** — synthesis must reflect this. Updates baseline reasoning across all agents. |
| T3 §5 was a migration-cost deferral that disappears in rebuild | challenger | Integrate into Round 2 cross-exam — reframing has substance; needs evidence challenge |
| T4 "AE-only" should mean extraction discipline not physical files | challenger | Integrate into Round 2 — distinct from ratify-as-stated; needs explicit rejection or absorption |
| T4 latent bug: AE-only is policy not enforcement; non-AE files rejected by trigger | archaeologist (factual) | Integrate immediately — affects how Topic 4 ships; separate from ratify decision but constrains fix |
| T5 Goodhart's Law on count-based metrics | challenger | Integrate into Round 2 — applies to nonempty rate / synthesis count proposals; cross-exam needed |

### 3. Verification artifacts

Round 1 produced one major and one minor verification source:

- **archaeologist's round-01/archaeologist.md** is the verification artifact for code-level claims. Specific verified facts (with file:line citations in archaeologist's file):
  - `watcher.rs` has zero non-test call sites (verified)
  - `com.mengdie.dream.plist` is a template, not deployable (verified)
  - "13 syntheses" was on-demand CLI invocation (verified)
  - Synthesis rows stored with `embedding=None` (verified)
  - Per-project default is `mcp_tools.rs:192-195`, 1-line diff to flip (verified)
  - F-002 audit table schema v6 exists; not exposed via MCP (verified)
  - AE-only is policy not enforcement; non-AE files rejected by v5 trigger (verified — latent bug)
- **All other agents' Round 1 files** carry directional verdicts grounded in source-material reading + lens-specific reasoning. These are claims that survive only if Round 2 cross-examination doesn't falsify them.

Claims that lack verification artifact and remain UNVALIDATED:
- "swiftide / rig contributor counts" (analysis.md flagged this as pending; not addressed in Round 1)
- specific industry-pattern claims (codex's "mem0 v1.0 pattern", gemini's "Google Drive pattern") — citations exist in agents' files but not independently re-verified

### 4. Frame-challenge disappearance self-check

**N/A** — Round 1 is the first round; no prior round of-framing markers exist to compare against. Round 2 will run this check by comparing Round 1's of-framing-disposition table (above) against any silent disappearance in Round 2 outputs.

## Round 2 cross-examination targets

**Required reading for Round 2 (per agent: read other 6 round-01 files)**:
- `round-01/system-architect.md`
- `round-01/archaeologist.md`
- `round-01/ai-engineer.md`
- `round-01/minimal-change-engineer.md`
- `round-01/codex-proxy.md`
- `round-01/gemini-proxy.md`
- `round-01/challenger.md`

**Topic-by-topic cross-exam priorities**:

1. **T1**: hybrid (gemini) vs push-only (4 agents) vs pull (challenger). Resolve in Round 2.
2. **T2**: Given cron is NOT running, does ai-engineer's trait-based on-demand-default + cron-opt-in win over "cron + on-demand both v0.0.1 defaults"? Cross-exam needed; this is the topic with the strongest new fact.
3. **T3**: 5 vs 1 ratify; challenger's reframing deserves a cross-exam round to see if any of the 5 ratifiers updates.
4. **T4**: forward-compat (codex) vs no-forward-compat (minimal-change, system-arch). Plus latent-bug intersection. Plus challenger's extraction-discipline reframe.
5. **T5**: which specific F-002 metric (3-way split among 5 ratifiers + 2 dissenters). Plus 028's no-ACK lock impact on gemini's thumbs up/down.

**Format expectation for Round 2**: each agent writes `round-02/<name>.md` with `## Findings (with file:line evidence)` / `## Agreements (cite specific peer file + line)` / `## Disagreements (cite specific peer file + line)` / `## Open Questions`. Cite peer claims by file path AND line number — TL synthesis is orientation, not authority.
