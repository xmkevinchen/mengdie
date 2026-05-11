---
id: "027"
round: "02"
lens: "gemini-proxy"
created: 2026-05-05
cross_exam_targets: [T1, T2, T3, T4, T5]
status: "complete"
---

# Round 2: Gemini-proxy cross-examination + position updates

## Topic 1: Ingest mechanism — hybrid vs push-only vs pull

### Findings (Round 2 evidence)

**Archaeologist verified infrastructure state** (archaeologist.md:7-27):
- Push path: `mcp_tools.rs:278-370` wired, operational via MCP.
- Pull path: `watcher.rs:20-63` library complete, zero production call sites.
- CLI bulk-import: `cli.rs:361-420` exists as a cold-start mechanism.

**System-architect's contract analysis** (system-architect.md:40-108):
- Push binds caller errors to skill responsibility; pull defers to daemon logging.
- Cold-start replay: push requires explicit `mengdie import`; pull replays "naturally" but needs startup reconciliation anyway.
- Atomic resolves (`resolves: Vec<String>`) are push-only — a watcher cannot parse "this fact supersedes X, Y."

**Codex-proxy's ecosystem analysis** (codex-proxy.md:24-55):
- OpenAI Vector Stores API is push-primary with async server-side processing.
- mem0 v1.0, LangMem, Graphiti all converged on push-with-async-queuing (no pull daemon).

**Challenger's counter-position** (challenger.md:29-76):
- Pull decoupling is architecturally superior for observing AE pipeline output.
- "Never wired to a daemon" is v0.x execution gap, not design failure.
- Error visibility symmetry: push errors are silent if AE skill forgets to call `memory_ingest` (no upstream signal).
- Daemon surface argument overstated: mengdie already runs launchd plist for dreaming; one more plist is marginal.

**Minimal-change-engineer's YAGNI position** (minimal-change-engineer.md:23-129):
- Pull semantics wrong: daemon sees raw markdown, not the structured fact AE already produced.
- Either daemon re-derives extraction (duplicates AE) or reverts to naive ingest (exactly v0.x problem).
- Hybrid doubles maintenance surface without solving core coupling issue.

### Agreements (with peer findings)

- **All agents agree cold-start replay requires a bulk-import mechanism.** Push + CLI import (`mengdie import <dir>`) is already implemented and exercised; pull requires daemon-startup reconciliation (which is equivalent to bulk import, just automatic). (system-architect.md:96-108, minimal-change-engineer.md:72-88)
- **All agents agree event-driven (queue/bus) is post-v0.0.1.** Single-binary stdio MCP server constraint per blueprint §6. (system-architect.md:62-64, minimal-change-engineer.md:63, codex-proxy.md:54)
- **Atomic resolves (`resolves` parameter) is push-only.** No disagreement. (system-architect.md:23-27, minimal-change-engineer.md:44)

### Disagreements (and resolutions)

| Claim | Proponent | Counter | Gemini-proxy verdict |
|-------|-----------|---------|-----|
| "Pull provides resilience, error auditability, and historical data completeness that push alone cannot guarantee." | gemini-proxy (Round 1), challenger (R2:32-76) | minimal-change-engineer (R2:89-99): "redundant delivery, exactly v0.x-rebuild-against pattern" | **Challenger + gemini make a valid resilience argument.** However, minimal-change's YAGNI framing is defensible: "AE not running" for a personal operator is rare; when it happens, `mengdie import` re-syncs cold-start data. The *concrete failure mode* (pull prevents) is "operator forgot to call `mengdie import` after AE was down," which is also an operator-oversight failure. Push-only is not architecturally wrong for v0.0.1. |
| "Hybrid (push + pull fallback) is the Google Drive pattern and gains resilience." | gemini-proxy (Round 1) | system-architect (R2:57-61), minimal-change-engineer (R2:57-62): "doubles contract surface, doubles failure mode set" | **Retract hybrid position.** The double-surface argument is decisive for v0.0.1. Hybrid adds two logs to reconcile, two error paths, two cold-start policies. Minimal-change's load-bearing test applies: no line of hybrid is forced by the core promise ("AE produces fact → mengdie ingests"). Push + explicit import is the minimum. |
| "Watcher library has zero production miles, so pull is riskier." | system-architect (implied), minimal-change-engineer | challenger (R2:73-76): "v0.x execution gap is not design failure; library is feature-complete" | **Challenger is technically correct, but execution risk is real for v0.0.1.** A daemon requires supervision (restart-on-crash), log rotation, heartbeat detection. For a solo operator, these are UX frictions. Push is operationally simpler. |

### Updated verdict (Topic 1)

**Gemini-proxy revises from hybrid to: PUSH as v0.0.1 primary. Watcher library kept as opt-in, NOT wired to daemon.**

**Rationale:**
1. **Push aligns with OpenAI/Google ecosystem convergence** (Vector Stores, mem0, LangMem, Graphiti all push-primary).
2. **Hybrid's double-surface cost exceeds its resilience benefit** in a v0.0.1 that is solo-operator scoped.
3. **Cold-start replay via `mengdie import` is already implemented and tested.** This is not a gap; it is a working alternative to pull-automatic-replay.
4. **Atomic resolves are push-only by construction.** This is a semantic requirement, not a limitation.

**Reversibility:** High. Watcher library remains in-tree; if daemon is needed post-v0.0.1, wrap it with launchd plist + supervision. No data migration.

**Open question:** Should the AE plugin side wire explicit `memory_ingest` calls in each skill (plan, work, review, retrospect, analyze)? Or is a shared helper inside the AE plugin the minimum? (answer: per-skill direct calls are minimum; helper is nice-to-have, not v0.0.1) (system-architect.md:130-136)

---

## Topic 2: Reflection trigger — cron + on-demand vs on-demand default

### Findings (Round 2 evidence — critical fact-change)

**Archaeologist verified cron is NOT running** (archaeologist.md:72-87):
- `resources/com.mengdie.dream.plist` is a template (line 8: `<!-- Update this path -->`).
- Plist is NOT deployed to the operator's macOS launchd.
- The "13 syntheses" first real run was on-demand CLI invocation (`mengdie dream --synthesize`), not cron-driven.

**This invalidates a core assumption in Round 1.** The framing stated "cron-based synthesis is already running." This is empirically false. On-demand is the only trigger that has actually fired in production.

**AI-engineer's trait-based proposal** (ai-engineer.md:133-177):
- Frames v0.0.1 as: **cron-baseline + on-demand override** (both already exist).
- Introduces `ReflectionTrigger` trait (~80 LoC) for pluggable triggers.
- Defers salience/composite/debounced as post-v0.0.1 BLs with explicit triggers.
- Argues this is empirically defensible: "we picked cron because it works; we built the trigger seam so the choice is reversible."

**Minimal-change-engineer's baseline defense** (minimal-change-engineer.md:136-200):
- Cron produced 13 syntheses; on-demand already exists.
- Both are "zero-new-code" candidates.
- Salience/composite/debounced each require metrics mengdie doesn't compute.
- Recommendation: **cron + on-demand as defaults (both shipped); other three as deferred BLs with trigger conditions.**

**Challenger's burst-activity critique** (challenger.md:99-166):
- SCM paper motivates composite triggers with burst-activity observation: AE pipeline produces 3–8 facts in an hour, cron fires at 2am.
- Temporal mismatch is a structural problem: synthesis should fire on cluster density, not wall clock.
- 13 syntheses are an output *count*, not a quality measure. Cron has not been validated for usefulness.
- On-demand as default is more honest: operator's engagement is the trigger; validates reflection usefulness before committing to a model.

**Codex-proxy's OpenAI lens** (codex-proxy.md:62-95):
- ChatGPT Memory uses debounced-submit-dedupe with capacity gating (inferred from product behavior).
- Cron is predictable (operator can check results); debounced/salience triggers are silent and hard to debug.
- Recommendation: cron + on-demand hybrid; cron is primary operational baseline; on-demand costs nothing and gives operator agency.

### Agreements (with peer findings)

- **All agents agree salience/composite/debounced require metrics mengdie doesn't compute.** These are post-v0.0.1 territory. (ai-engineer.md:106-131, minimal-change-engineer.md:147-200, codex-proxy.md:83-90)
- **mem0's state-of-memory-2026 explicitly lists "reflection trigger that isn't cron or on-demand" as unsolved.** The industry has not solved this. (ai-engineer.md:99-104, minimal-change-engineer.md:161-165)
- **On-demand is already shipped and exercises the synthesis mechanism.** (archaeologist.md:83-87, minimal-change-engineer.md:139-141)

### Disagreements (and resolutions)

| Claim | Proponent | Counter | Gemini-proxy verdict |
|-------|-----------|---------|-----|
| "Cron is the running baseline, already producing 13 syntheses." | system-architect (R1:173-179), codex-proxy (R1 implicit), minimal-change-engineer (R1:136-141) | **archaeologist (R2 verified fact)** | **Concede.** Cron is not running in production. The premise of "cron + on-demand both already shipped" is false. Cron logic exists in dreaming.rs, but the trigger is not wired. This fact fundamentally shifts the framing. |
| "Cron baseline + on-demand escape hatch is the right two-layer model." | codex-proxy (R2:82-95), minimal-change-engineer (R2:169) | challenger (R2:112-142): "temporal mismatch on burst activity is a structural problem; on-demand should be default" | **Challenger's structural critique is valid, but operational reality favors cron as baseline for a rebuild.** The reason: cron is "hands-off" once configured; operator doesn't have to remember to synthesize. On-demand requires operator agency to be useful. For AE 的大脑 (operator-driven system), on-demand aligns better with the thesis. However, minimal-change's characterization ("both zero-new-code") matters for v0.0.1 sequencing. |
| "On-demand should be the v0.0.1 default; cron is opt-in." | challenger (R2:106-137), ai-engineer (R2:193-200 implicit), gemini-proxy (Round 1 was cron + on-demand, not default-specified) | minimal-change-engineer (R2:169): "cron is the documented default; on-demand is the escape hatch" | **Evidence favors on-demand as default.** (1) Cron is not running, so "documented baseline" is aspirational. (2) On-demand was the actual trigger that produced the 13 syntheses. (3) Operator control aligns with "AE 的大脑" thesis. (4) Trait-based seam (ai-engineer) costs ~80 LoC and future-proofs; not over-engineering. Recommend: **on-demand as v0.0.1 default; cron as documented opt-in via launchd + plist; trait seam if time permits (BL candidate).** |

### Updated verdict (Topic 2)

**Gemini-proxy revises from "cron + on-demand both viable" to: ON-DEMAND as v0.0.1 default. Cron as documented opt-in.**

**Rationale:**
1. **Archaeologist's verification fact-change:** Cron is not running. Crediting it as a baseline is false.
2. **On-demand was the actual trigger.** The 13 syntheses came from explicit operator invocation (`mengdie dream --synthesize`), validating on-demand works.
3. **On-demand aligns with "AE 的大脑" thesis.** Operator-driven trigger matches operator-driven AI system identity.
4. **Cron can be documented as opt-in for future use.** When/if the operator tires of manual synthesis, they configure launchd plist.
5. **AI-engineer's trait seam is worthwhile** (~80 LoC) if sequencing allows. Preserves optionality for salience/composite/debounced post-v0.0.1.

**Reversibility:** Very high. On-demand + cron (if implemented) is a stable local optimum. Upgrade trigger sophistication when corpus grows and metrics suggest need.

**Open question:** Should the trait-seam be in v0.0.1 scope, or deferred as a cleanup BL? (answer: trait-seam is clean enough for v0.0.1 if not blocking; deferred if timeline tight — does not change the on-demand-default verdict either way)

---

## Topic 3: Cross-project scope — per-project default ratify

### Findings (Round 2 evidence)

**Archaeologist verified architecture** (archaeologist.md:143-200):
- Per-project default is 4 lines in `mcp_tools.rs:189-195`: `scope: "global" → None, else → Some(pid)`.
- Storage is globally `project_id` tagged; changing default is 1-line diff.
- Changing cost: zero architectural; API already supports both directions.

**System-architect** (system-architect.md: Topic 3 not detailed in Round 1 excerpt, but synthesis shows system-architect ratifies).

**Minimal-change-engineer** (minimal-change-engineer.md: Topic 3 ratification implied).

**Codex-proxy's ratification with observability note** (codex-proxy.md:98-126):
- OpenAI's namespacing pattern is per-namespace isolation by default, cross-namespace opt-in.
- Cross-project retrievals should be observable in audit logs so operator can answer "do I use cross-project?" with data.

**Challenger's counter-position** (challenger.md:170-199):
- Single operator = unified identity across projects.
- Per-project default is artificial silo that fights the core use case.
- When asking "what did we decide about MCP transport?" the right answer draws from *all* projects where MCP was discussed.
- Contamination risk ("memory true in A, wrong in B") is weak for one operator with unified conventions.
- Argument: per-project default should be opt-in; cross-project should be the default.

**Gemini-proxy Round 1** (gemini-proxy.md Round 1, Topic 3):
- NotebookLM's notebook silos are documented user frustration.
- Per-project default is a valid, ratifiable pattern per Google's own products.
- Recommended: ratify §5 unchanged; add global search index; defer cross-project synthesis to P2.

### Agreements (with peer findings)

- **All agents agree storage is global; search default is a 1-line diff.** Changing direction is cheap and reversible. (archaeologist.md:174-182)
- **5 agents ratify §5 unchanged (per-project default).** Only challenger dissents. (synthesis.md:52-54)
- **All agents agree cross-project *retrieval* should be possible (via scope: "global" parameter).** The disagreement is on the default, not on capability. (codex-proxy.md:126, synthesis.md:59)

### Disagreements (and resolutions)

| Claim | Proponent | Counter | Gemini-proxy verdict |
|-------|-----------|---------|-----|
| "Per-project default prevents silos while avoiding cross-contamination; it is sound." | 5 agents (ratify §5) | challenger: "artificial silo that fights the use case for a single unified operator" | **Challenger makes a coherent reframing, but ratifiers' position is defensible for v0.0.1.** Argument: (1) Operating from within a project context is the common case (user opens a project dir in Claude Code). (2) Cross-project is one `scope: "global"` flag away. (3) Contamination risk is real even for one operator (project A's decision was "for that architecture"; project B's is different). Challenger's unified identity is true in principle but not universally applicable in practice. **Ratify §5 unchanged.** |
| "NotebookLM's silos are frustrating; suggests default should flip to cross-project." | challenger (R2 implicit) | gemini-proxy (Round 1): "per-project default + global search index balances focus with discoverability" | **NotebookLM frustration is about *synthesis* silos, not *retrieval* silos.** Users want to synthesize insights *across* notebooks; they also want per-notebook focus for active work. Mengdie can satisfy both: per-project search default (focus), cross-project synthesis opt-in (P2 feature). This is not an argument to flip the default; it is an argument to ship cross-project *synthesis* as a P2 follow-up. **Maintain per-project default; record trigger for cross-project synthesis as "revisit when operator works across ≥2 simultaneous active projects."** |

### Verdict (Topic 3)

**Gemini-proxy confirms ratification: CLAUDE.md §5 (per-project default search) ratified unchanged.**

**Rationale:**
1. **Google's own products (GCP, Drive, Workspace) use per-project isolation by default** with explicit cross-project opt-in.
2. **NotebookLM's frustration is about synthesis, not retrieval.** The answer is cross-project synthesis as P2, not flipping the default.
3. **Per-project focus aligns with operator's typical workflow** (user opens a project dir; searches default to that project).
4. **Global search index exists** (via scope: "global" parameter), preventing true silos.

**Reversibility:** High. Changing the default is a 1-line diff if evidence emerges.

**Open questions:**
- Should the re-opening trigger be "≥3 queries/month that would benefit from cross-project sources" or "operator works on ≥2 simultaneous active projects"? (Codex proposed 30% global opt-in; ai-engineer proposed 10% global queries; minimal-change proposed specific audit metrics.) Answer: use F-002 audit data post-v0.0.1 to make this empirical, not speculative.
- Does the per-project-at-startup model (project_id inferred from cwd at MCP-server-startup, archaeologist.md:184-193) need fixing if the operator switches projects without restarting Claude Code? (Answer: deferred as a separate bug; not a Topic 3 design issue.)

---

## Topic 4: Ingest source boundary — ratify AE-only with or without forward-compat

### Findings (Round 2 evidence)

**Archaeologist verified enforcement gap** (archaeologist.md: Round 1 implicit, synthesis.md:75):
- AE-only is a policy, not enforced. `memory_ingest` accepts any text.
- `infer_source_type` returns "unknown" for non-AE files; v5 schema trigger REJECTS them — **this is a latent bug for non-AE files passing through the file-ingest path.**

**System-architect** (synthesis.md:66-80, implicit ratification):
- Ratifies AE-only as v0.0.1 boundary.
- Minimal-change proposes NO forward-compat scaffolding (YAGNI).

**Minimal-change-engineer's YAGNI position** (synthesis.md:70):
- Ratify AE-only, NO forward-compat.
- Forward-compat is reinvention failure mode; reopening trigger only.

**Codex-proxy's forward-compat proposal** (synthesis.md:71):
- Ratify AE-only, WITH forward-compat (typed source markers, per-source filters).
- Bake in from v0.0.1 to avoid v1 API break.

**AI-engineer** (synthesis.md:72):
- Ratifies AE-only.
- Grounds ratification in Perplexity admission-filtering precedent.

**Challenger's reframe** (synthesis.md:73):
- "AE-only" should mean extraction discipline, not physical AE files.
- Ad-hoc debug-session facts are load-bearing edge case.

**Gemini-proxy Round 1** (gemini-proxy.md Round 1, Topic 4):
- NotebookLM's boundary expansion (PDF/Docs/URLs → Meet Transcripts) is strategic, reactive, and driven by use-case fit.
- Lessons: start strict; expand when use case is clear + technical fit is high.
- Recommended: ratify AE-only; forward-compat baked in for strategic expansion.

### Agreements (with peer findings)

- **All agents agree AE-only ratifies.** No dissent. (synthesis.md:68-72)
- **All agents agree broad sources that bypass AE-style extraction pollute the corpus** (CLAUDE.md principle "high signal-to-noise"). (synthesis.md:73, codex-proxy.md:156, minimal-change-engineer.md:50-56)
- **Latent enforcement bug exists:** AE-only is policy not enforcement; non-AE files are rejected by v5 trigger, but error message is wrong. (synthesis.md:75)

### Disagreements (and resolutions)

| Claim | Proponent | Counter | Gemini-proxy verdict |
|-------|-----------|---------|-----|
| "Forward-compat (typed source markers) should be baked into v0.0.1 to avoid v1 API break." | codex-proxy (R2:158-163) | minimal-change-engineer (YAGNI): "forward-compat is scope creep; reopening trigger only" | **Codex's "cheap insurance" framing is attractive, but minimal-change's YAGNI is correctly applied.** Typed source markers cost ~30 LoC (enum + serde rename_all) — this is not burdensome. However, "forward-compat for possible future expansion" is premature without a concrete next expansion. Gemini's evidence suggests NotebookLM's expansion was reactive (Meet Transcripts added after user demand, not pre-planned). Recommendation: **ratify AE-only without forward-compat scaffolding in v0.0.1; record trigger as "when a non-AE source (commit messages, issue text, chat summaries) is requested with a concrete use case, file a BL that includes schema migration to typed source markers if not already present."** This avoids pre-building for hypothetical futures while keeping the path clear when a real use case emerges. |
| "Latent enforcement bug: non-AE files rejected by v5 trigger, error message wrong." | archaeologist (synthesis.md:75) | (no explicit counter in Round 1) | **This is a separate concern from Topic 4 ratification.** The bug does not block the ratification decision (AE-only is still the right policy). However, the fix is constrained: if AE-only is enforced, the rejection is correct behavior and the error message should be clarified ("only AE pipeline artifacts are accepted in v0.0.1"). If broader sources are later permitted, the rejection becomes a real bug that needs removal. Recommendation: **file as a separate BL: "Clarify AE-only enforcement + error messaging for v0.0.1; triggers when broader-source-support BL files."** Do not block Topic 4 ratification. |
| "AE-only should mean extraction discipline, not physical AE files; ad-hoc debug facts are valuable." | challenger (synthesis.md:73) | (implicit ratifiers) | **Challenger's reframe is interesting but separable from Topic 4.** If the operator wants to ingest ad-hoc debug notes alongside AE pipeline artifacts, the path is: (1) extract them through AE-style LLM processing (challenger's "extraction discipline"), then (2) ingest via `memory_ingest` with a new `source_type: "debug_note"` (post-v0.0.1 when broader sources are explicitly permitted). This is not "reject AE-only," but "clarify that *all* ingested content must meet AE extraction discipline standards," which is implied by CLAUDE.md already. **Absorb challenger's clarification but ratify AE-only unchanged.** |

### Verdict (Topic 4)

**Gemini-proxy confirms ratification: AE-only as v0.0.1 boundary, NO forward-compat scaffolding in v0.0.1.**

**Rationale:**
1. **NotebookLM's expansion pattern is reactive, not pre-planned.** Meet Transcripts were added after user demand demonstrated value.
2. **Typed source markers cost ~30 LoC, but minimal-change's YAGNI applies:** no concrete next source in v0.0.1 scope.
3. **When the next source is requested, the migration is tractable:** add enum variant, backfill (or start new), ship migration. Deferring avoids speculative code.
4. **AE extraction discipline is already baked into CLAUDE.md principle.** Clarifying error messages is sufficient.

**Reversibility:** High. Adding typed sources post-v0.0.1 is a forward-compatible schema extension (new enum variant).

**Open question:** Should the enforcement clarification BL be filed now (as a separate concern from ratification) or deferred until a second source is requested? (Answer: file now as a code-quality BL; it's a one-line error message fix and clarifying to operators.)

---

## Topic 5: Loop-closure signal — reconciling 028's no-ACK lock

### Findings (Round 2 evidence — critical constraint)

**028 conclusion explicitly locked** (028:22-31):
> "MCP `memory_search` ACK feedback channel — NO in v0.0.1 contract. Caller acknowledgment is ambiguous — an AI that reads and discards facts by exclusion has still 'used' them. Contractual burden on every integrator is not worth a noisy precision estimate. **All Topic 4 triggers must be server-side observable from the persisted domain audit table.**"

This decision makes clear: **no per-result MCP ACK feedback from callers to mengdie.**

**Gemini-proxy Round 1 proposal** (gemini-proxy.md Round 1, Topic 5):
- Thumbs up/down on every search result.
- Forced weekly stats report.
- Operator-driven signals (explicit "useful/not useful" marks).

**AI-engineer's position** (ai-engineer.md:86-102):
- Proposes: per-search nonempty rate (F-002 audit data) + ae:retrospect qualitative + falsification rule.
- Justifies: "computable from existing F-002 data, no new schema; concrete falsification rule (nonempty < 20% over 14d AND two 'idk' retro verdicts → loop not delivering)."

**Minimal-change-engineer** (minimal-change-engineer.md:87, synthesis.md:87):
- Cites 028's no-ACK lock.
- Proposes: F-002 audit table + `mengdie audit-stats` CLI + ae:retrospect.
- **Explicitly flags: "ACK feedback is scope creep against 028's locked 'no ACK'."**

**System-architect** (synthesis.md:88):
- Proposes: search-with-results-rate + synthesis-influencing-search rate.
- Caveat: synthesis-influencing-search requires search-result-cited signal which archaeologist confirms is NOT in F-002 schema (archaeologist.md:95).

**Codex-proxy** (synthesis.md:89):
- Proposes: search utilization rate (F-002) + operator retro verdict.
- Avoids per-result ACK; focuses on "was this fact cited in downstream agent output."

**Challenger** (synthesis.md:91):
- Proposes: contradiction-detection trend + Round 0 citation rate.
- Frames as "Goodhart's Law pre-check" — count metrics are gameable; hard-to-game signals are better.

### Agreements (with peer findings)

- **All agents agree 028's no-ACK lock is real and binding.** No challenge. (synthesis.md:97-102)
- **All agents except gemini-proxy agree per-result thumbs up/down violates the lock.** (minimal-change-engineer.md:87 explicit; synthesis.md:100 flags as conflict)
- **F-002 audit table exists with query/scope/took_ms/returned-fact-IDs.** This is foundational for all proposals. (archaeologist.md:95)
- **Synthesis-influencing-search-rate requires new instrumentation** (search-result-cited signal) that archaeologist confirms is NOT in F-002 yet. (archaeologist.md:95, synthesis.md:98-99)

### Disagreements (and resolutions)

| Claim | Proponent | Counter | Gemini-proxy verdict |
|-------|-----------|---------|-----|
| "Per-result thumbs up/down is a non-ACK UX signal, not MCP-critical-path acknowledgment." | gemini-proxy (R1) | 028 lock + minimal-change-engineer (R2:87): "ACK feedback is scope creep; 028 forbids operator-facing signals that influence the core memory state" | **028's lock is stricter than I interpreted.** It says "no caller acknowledgment," period — not just "no critical-path acknowledgment." The distinction I drew (UX signal vs MCP ACK) may be clever, but it violates the spirit of the lock: 028 is saying "feedback loops that require operator action per-search are out of scope for v0.0.1." Thumbs up/down per result, even if aggregated offline, requires operator action per search result. **Concede: per-result thumbs violates 028's intent.** |
| "Forced weekly stats report is a valid non-ACK signal." | gemini-proxy (R1) | 028 framing: "all Topic 4 triggers must be server-side observable from persisted audit table" | **This one I can defend.** A weekly forced report is not per-search feedback; it is a post-hoc aggregation of server-side data (F-002 audit table, contradiction events, synthesis rows). The operator reads it passively; no per-result action required. This does NOT violate 028's lock. Minimal-change acknowledges audit-stats CLI exists (BL-014 already filed). **Defend: weekly forced stats report is valid, non-ACK signal.** |
| "Which specific F-002 metric? nonempty rate (ai-engineer) vs synthesis-influencing rate (system-architect) vs empty-result rate (minimal-change)?" | (3-way split, synthesis.md:98) | All three are computable; all require different interpretation of "value delivered" | **Three proposals map to different failure modes.** (1) **Nonempty rate** catches "searches return nothing; loop is broken." (2) **Synthesis-influencing-search rate** catches "searches return results but they don't feed forward synthesis" (requires schema change). (3) **Empty-result rate** is inverse of nonempty (same signal, inverted framing). Recommendation: **minimize: start with nonempty rate + ae:retrospect qualitative.** This is F-002-only, no new schema, and directly falsifiable: "if >20% of searches are empty over 14 days, investigate why." Synthesis-influencing rate can be P2 (requires adding search-result-cited to F-002 schema). |

### Updated verdict (Topic 5)

**Gemini-proxy revises from "thumbs up/down per result" to: F-002 nonempty rate + forced weekly stats + ae:retrospect qualitative.**

**Rationale:**
1. **028's no-ACK lock rules out per-result thumbs.** The lock is stricter than I initially interpreted; it forbids operator-feedback loops that require per-search action. Weekly aggregate reporting ≠ per-search feedback.
2. **Forced weekly stats report is valid.** It is server-side observable, offline aggregation, not a per-result ACK.
3. **Start with nonempty rate (F-002 only).** No new schema required. Directly falsifiable: nonempty rate < 20% over 14 days = loop not delivering.
4. **Add ae:retrospect qualitative layer.** After each `/ae:retrospect`, prompt operator: "Did mengdie help avoid re-discovering past decisions? (yes/uncertain/no)." This is lightweight, qualitative, and captures operator intent.
5. **Synthesis-influencing rate is P2.** Requires schema addition (search-result-cited signal in F-002). Defer until v0.0.2.

**Reversibility:** High. Metrics are all server-side; can add/remove without redeploying to operators.

**Falsification path:** If nonempty rate > 80% over 14 days AND ae:retrospect feedback is consistently positive ("yes, mengdie helped"), the loop is closing. If nonempty rate < 20% over 14 days OR ae:retrospect verdicts are consistently negative/uncertain, investigate (facts not useful, or synthesis not firing fast enough, or retrieval is broken).

**Open questions:**
- Should the per-operator retro verdict be stored as a dedicated table/row, or appended to ae:retrospect output? (Answer: store as a dedicated signal; simplifies metric aggregation.)
- Is the 14-day window the right baseline, or should it be operator-configurable? (Answer: hardcode for v0.0.1; make configurable post if evidence suggests shorter/longer windows are more useful.)

---

## Synthesis: Updated position summary

| Topic | Round 1 verdict | Round 2 update | Key change |
|---|---|---|---|
| **T1: Ingest mechanism** | Hybrid push-primary + pull-fallback | **Push-only; watcher library kept opt-in, not wired** | Retracted hybrid; double-surface cost exceeds v0.0.1 benefit |
| **T2: Reflection trigger** | Cron + on-demand both viable | **On-demand as default; cron as opt-in; trait seam if time permits** | Archaeologist fact-change (cron NOT running) invalidated baseline; on-demand was actual trigger |
| **T3: Cross-project scope** | Ratify per-project default | **Ratify per-project default unchanged; cross-project synthesis as P2** | Challenger's unified-identity reframe is valid but applies to *synthesis*, not *retrieval* |
| **T4: Ingest source boundary** | Ratify AE-only with forward-compat | **Ratify AE-only, NO forward-compat scaffolding in v0.0.1** | NotebookLM expansion pattern is reactive, not pre-planned; defer scaffolding until concrete next source |
| **T5: Loop-closure signal** | Thumbs up/down per result + weekly stats | **F-002 nonempty rate + forced weekly stats + ae:retrospect qualitative; no per-result ACK** | 028's no-ACK lock rules out per-result thumbs; defended weekly aggregation report as valid non-ACK |

---

## Cross-lens observations

1. **Google long-context pattern informs multiple topics:** Long-context (Gemini 1.5 Pro) reduces external pre-processing reflection but does not eliminate it. This validates ai-engineer's trait-seam approach: build the trigger interface now, add sophistication (salience, composite) when corpus grows.

2. **Archaeologist's fact-changes are load-bearing:** The discovery that cron is NOT running was not a nuance; it invalidated Round 1 assumptions for three agents (system-architect, minimal-change-engineer, codex-proxy). This is evidence that verification artifacts (code inspection, schema review) matter more than literature-based reasoning for v0.x state.

3. **028's decision boundaries propagate:** The no-ACK lock in 028 rippled through Topic 5 reasoning across all agents. This is healthy: prior-discussion locks prevent re-litigation of settled decisions.

4. **Forward-compat tension is healthy but time-bounded:** Codex's "cheap insurance" vs minimal-change's YAGNI will recur whenever new abstraction is proposed. For v0.0.1, YAGNI wins if no concrete second instance is in scope. Post-v0.0.1, the decision becomes context-dependent (how often do new sources get added? how expensive is migration?).

---

## Open questions for synthesis

- Should the per-skill AE plugin wiring (which skill calls `memory_ingest` after producing artifacts) be specified in the v0.0.1 blueprint, or deferred to an AE-plugin-side BL? (architectural vs operational scope boundary)
- Is the enforc ement clarification for Topic 4 (AE-only error messaging) a v0.0.1 BL or v0.0.2 cleanup? (I'd recommend v0.0.1; it's one-line code + clarifies intent.)
- Does gemini-proxy's conversion on T1, T2, T5 strengthen any other agent's position, or are these just local updates? (Answer: T2 on-demand-default strengthens ai-engineer + challenger; T1 push-only aligns with codex-proxy + minimal-change; T5 weekly-stats aligns with minimal-change + ai-engineer.)
