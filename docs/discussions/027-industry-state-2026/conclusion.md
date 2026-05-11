---
id: "027"
title: "v0.0.1 Step 0 — industry state of personal AI memory 2026 — Conclusion"
concluded: 2026-05-05
plan: ""
entities: [ingest, mechanism, ingest-mechanism, reflection, trigger, reflection-trigger-model, reflectiontrigger-trait, cross-project, retrieval, scope, cross-project-retrieval-scope, source, boundary, ingest-source-boundary, extraction-discipline, loop-closure, signal, loop-closure-signal, ae-retrospect, f-002-nonempty-rate, push-primary, watcher-opt-in, on-demand-trigger, ratify-section-five, ae-only-extraction]
spawned_bls: [BL-022, BL-023, BL-024, BL-025]
---

# v0.0.1 Step 0 — Conclusion

The 2026 industry survey (analysis.md + blueprint.md v0.2) produced 8 cross-source convergent patterns and identified mengdie's unfilled niche. Five blueprint §8 architectural design points have now been resolved through 2 rounds of 7-agent council discussion + 3 framing-review reruns + 2 user decisions on substantive splits. P1/P2 BL filing under v0.0.1 rebuild is unblocked.

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Ingest mechanism — delivery pattern from AE to mengdie | **Push-primary**: AE skill explicitly calls `memory_ingest` after each pipeline phase. `core/watcher.rs` kept as opt-in library code, NOT wired to mcp_server.rs/cli.rs daemon. Cold-start uses `cmd_import` (`cli.rs:361`, already shipped). | 7/7 convergence after challenger updated R2 on `resolves` atomicity as design-merit (not v0.x execution gap). `cmd_import` covers cold-start replay (archaeologist verified `cli.rs:361-424`). Pull-fallback for "AE not running" produces no inputs anyway. Industry pattern: mem0 v1.0 / LangMem ReflectionExecutor / Vector Stores API all push. | high (both push and pull infrastructure exist; switching is wiring not data migration) |
| 2 | Reflection trigger model — v0.0.1 default + `ReflectionTrigger` trait | **On-demand as v0.0.1 default trigger**. Introduce `ReflectionTrigger` trait (~50-80 LoC seam, mirrors `LlmProvider` pattern). Cron is a second impl plugged into the trait, deployed via operator launchd plist + 1-paragraph setup doc. Salience / composite / debounced are deferred BLs (BL-024) that slot into the same trait when filed. | User decision after 4-vs-2 team split. Trait FOR (ai-engineer originator + codex/gemini/challenger): cheap insurance, mirrors `LlmProvider` abstraction, future BLs plug in cleanly. Trait AGAINST (system-architect/minimal-change): premature abstraction. Both sides agreed on-demand IS v0.0.1 default — archaeologist verified `com.mengdie.dream.plist:74-77` is a template (placeholder path), the "13 syntheses" first run was on-demand CLI, not cron. Synthesis embedding=None bug (BL-022) gates which trigger can be default — on-demand works now; cron-default needs the embedding fix. | high (adding/removing trait abstraction is refactor; no data migration) |
| 3 | Cross-project default retrieval scope — ratify §5 with rationale refinement | **Ratify §5** per-project default unchanged at the storage/query level. AE skills should specify `scope` per-skill explicitly (default-when-omitted is fallback). Cross-project synthesis explicitly deferred to P2. | 7/7 convergence after challenger updated R2 with NEW rationale (recorded here as the authoritative reason): cross-project contamination risk — project-A decisions surfaced in project-B contexts; AI agents won't reliably filter by provenance — outweighs cross-project recall benefit at solo-operator scale (ai-engineer cluster contamination argument `round-01:476-494`). The original §5 "avoid migration cost" framing is superseded since storage is already global (archaeologist verified `mcp_tools.rs:192-195` 1-line diff). | high (per-project default = `mcp_tools.rs:192-195` 4-line conditional; storage already global; flipping default = 1-line change) |
| 4 | Ingest source boundary — extraction-discipline (not AE-files-only) | **Ratify the spirit of "AE-only"** — mengdie does not become a generic memex — interpreted as **extraction discipline**, NOT physical AE-files-only enforcement. Existing `source_type` enum is the typed marker. `memory_ingest` accepts text payloads from any caller IF the caller asserts AE-style structured extraction was applied. Latent bug fix: rename `source_type::unknown` → `direct` (archaeologist round-01:269-275 latent rejection bug). Ad-hoc facts (e.g., debug-session conclusions outside AE pipeline) are valid via the `direct` source_type. | User decision after 3-vs-3 team split. Extraction-discipline FOR (system-architect + challenger + ai-engineer R2): existing `source_type` enum sufficient as typed marker; AE-discipline is quality gate not physical-source restriction; archaeologist's latent-bug evidence (`memory_ingest` already accepts any text; non-AE rejected by trigger) confirms extraction-discipline is what the code already does. AE-files-only AGAINST (codex/gemini/minimal-change R2): tighten parser allowlist + clear MCP error; ~25 LoC bug fix. Both sides agreed: ratify spirit of AE-only; latent bug must be fixed. | medium (switching post-ship requires re-classification of memories ingested under one stance to match the other; storage shape identical but ingest-API behavior differs) |
| 5 | Loop-closure signal — F-002 nonempty rate + ae:retrospect | **Two-signal v0.0.1 minimum, both 028 no-ACK lock-compliant**: (1) Quantitative — per-search nonempty rate computed from F-002 `audit_returned_facts` table (server-side aggregation; no schema changes; no new ACK channel). (2) Qualitative — `ae:retrospect` hook with prompt "did mengdie short-circuit anything this week? y/n/idk" + falsification rule: nonempty rate <20% over 14 days AND two consecutive idk verdicts → loop NOT delivering value. Surfacing: `mengdie audit-stats` CLI subcommand (BL-014 already filed). Optional secondary signals (file as P1 BLs, gated on synthesis-embedding fix BL-022): synthesis-influencing-search rate, contradiction-trend, zero-row-days, repeat-query-density. | Convergence on substrate (F-002 audit table) and shape (one quantitative + one qualitative). 028 no-ACK lock verified verbatim (`docs/discussions/028-v0.0.1-architecture-design/conclusion.md:22-27`) — rules out gemini's R1 thumbs up/down (retracted R2), codex's R1 cited-rate (retracted R2), challenger's R1 R0-citation rate (retracted R2). Inverse-Goodhart property: nonempty rate gaming = correct mengdie usage. Falsification rule (Perplexity 77→95% recall by storing half as many memories — mengdie should fire a tripwire if going wrong direction). | high (signals are read-only computed views over existing F-002 audit data; adding/removing metrics is non-breaking; falsification thresholds are tunable post-ship) |

## Doodlestein Review

Post-conclusion review by 3 fresh Doodlestein agents in the existing 027-council team. All 3 findings are valid + integratable via direct edits to this conclusion (no team round, no decision reversal).

### doodlestein-strategic — single smartest improvement

**Finding**: T3 reopening trigger first arm (`≥10% scope: 'global' opt-in over 30 days`) is self-defeating — it measures *override usage* (operator already knows they need cross-project) rather than *outcome quality* (whether per-project default is failing). False confidence that the decision is being watched.

**Disposition**: INTEGRATED via edit. T3 reopening trigger first arm replaced with outcome-oriented variant referencing T5's nonempty-rate primary signal (see "Reopening Triggers" table below). Second arm (3 retrospect-reported incidents) retained as-is — well-formed.

### doodlestein-adversarial — first failure mode in real use

**Finding**: BL-025 ("AE plugin per-skill `memory_ingest` wiring + explicit `scope` parameter") trigger condition reads `"v0.0.1 ingest contract finalized AND ReflectionTrigger trait shipped"`. The `ReflectionTrigger` trait is Topic 2's output and architecturally independent of Topic 1's wiring work. Real-use risk: ReflectionTrigger trait is treated as low-priority boilerplate ("cheap insurance"); BL-025 never trips its trigger; AE skills never start calling `memory_ingest`; the loop never closes in practice; ae:retrospect produces consistent "idk" responses; T5 falsification rule fires for the wrong reason.

**Disposition**: INTEGRATED via edit. BL-025 trigger condition simplified to drop the false AND conjunction (`docs/backlog/unscheduled/BL-025-...md` updated).

### doodlestein-regret — most likely reversal in 6 months

**Finding**: T2 (on-demand default + `ReflectionTrigger` trait) is the most likely decision to be reversed. The 4-vs-2 user-decided split is the tell — the minority position (minimal-change + system-architect: premature abstraction) had substantive grounds. Specific 6-month observation that would force reversal: F-002 audit data shows synthesis rates lagging ingest rates by >2x sustained over 30+ days. Interpretation: on-demand default requires operator discipline that does not materialize. Reversal cost is LOW because the trait was designed for this — ~20-30 LoC swap to make cron the default impl, launchd plist already templated, no data migration, no BL-024 ripple (deferred trigger impls plug into same trait regardless).

**Disposition**: INTEGRATED as a new T2 reopening trigger (see "Reopening Triggers" table below). The prediction is plausible and provides an operational tripwire; recording it makes the regret prediction itself an instrument of recovery.

### Process

- 3 Doodlestein challenges raised; 3 integrated by direct edit; 0 reopened topics; 0 new rounds spawned.
- Conclusion remains converged on all 5 topics.
- T3 + T4 + T2 now all carry explicit reopening triggers (T1 + T5 are high-reversibility with self-evident operational signals already; no separate trigger needed).

## Spawned Backlog Items (Step 7 Sweep)

The discussion surfaced four follow-on engineering items recorded as new BLs in `docs/backlog/unscheduled/`:

| BL | Title | Origin | Trigger |
|---|---|---|---|
| BL-022 | Synthesis row re-embed pass | archaeologist round-01 verification (`dreaming.rs:569-570` + `clustering.rs:71-79`) | Cron-default trigger reconsideration OR T5 synthesis-influencing-search rate metric landing |
| BL-023 | `project_id` cwd-switch staleness fix | archaeologist round-01 verification (`mcp_server.rs:32-34`) | Operator runs MCP server across multiple project working directories within a single session |
| BL-024 | Reflection trigger feasibility — salience / composite / debounced | Topic 2 deferred candidates | Per-trigger conditions (corpus size for composite; structured importance signals for salience; daemon shape change for debounced) |
| BL-025 | AE plugin per-skill `memory_ingest` wiring + explicit `scope` parameter | Topic 1 push-primary + Topic 3 explicit-scope refinement | v0.0.1 ingest contract finalized AND `ReflectionTrigger` trait shipped |

Topic 5's secondary signals (synthesis-influencing-search rate, contradiction-trend, zero-row-days, repeat-query-density) are captured as deferred to follow-on BLs gated on observation of primary signal sufficiency + (where applicable) BL-022 synthesis-embedding fix.

## Reopening Triggers (recorded for ratify topics + Doodlestein-flagged decisions)

| Topic | Trigger that would reopen the decision |
|---|---|
| 2 (Reflection trigger) | F-002 audit data shows synthesis rates lagging ingest rates by >2x sustained over 30+ days. Interpretation: on-demand default requires operator discipline that doesn't materialize in practice. Reversal path: promote cron impl to default within the same `ReflectionTrigger` trait (~20-30 LoC swap; launchd plist already templated; no data migration). *(Recorded post-conclusion by doodlestein-regret.)* |
| 3 (Cross-project scope) | T5 nonempty rate (primary signal) stays below 20% for 30+ days despite operator-confirmed active mengdie usage *(outcome-oriented; replaces the self-defeating "≥10% global opt-in" measurement of override-usage flagged by doodlestein-strategic)*, OR 3 retrospect-reported incidents of "I knew this was decided in another project but mengdie didn't surface it" |
| 4 (Source boundary) | ≥3 high-value facts per quarter that cannot be retrofitted into AE pipeline AND cannot reasonably be wrapped with the `direct` source_type |

*T1 (push-primary) and T5 (nonempty rate + ae:retrospect) carry no separate reopening triggers — their reversal signals are self-evident operationally (T1 reversal would manifest as widespread ingest failures the operator sees directly; T5 IS the falsification surface for the loop itself).*

## Spawned Discussions

None. All 5 topics converged within 027.

## Deferred Resolutions

None. Step 7 Sweep produced zero deferred topics. The four spawned BLs (above) are independent engineering items, not deferred topic resolutions.

## Team Composition

| Agent | Role | Backend | Joined |
|---|---|---|---|
| host | TL (moderator) | Claude (Opus 4.7 1M) | Start |
| system-architect | system design / module boundaries / contract design | Claude (project agent: `engineering-software-architect`) | Start |
| archaeologist | code archaeology / fact-grounded baselines | Claude (built-in: `ae:research:archaeologist`) | Start |
| ai-engineer | reflection / synthesis ML expertise | Claude (project agent: `engineering-ai-engineer`) | Start |
| minimal-change-engineer | scope discipline / Karpathy load-bearing test | Claude (project agent: `engineering-minimal-change-engineer`) | Start |
| codex-proxy | OpenAI cross-family lens | Codex MCP | Start |
| gemini-proxy | Google cross-family lens | Gemini MCP | Start |
| challenger | groupthink prevention / contrarian counter-positions | Claude (built-in: `ae:workflow:challenger`) | Start |

Round 0 framing-review team (separate, torn down before Step 2): codex-proxy + gemini-proxy + doodlestein-strategic + doodlestein-adversarial + engineering-minimal-change-engineer. 3 reruns total (rerun #0 + rerun #1 + rerun #2 → APPROVED-via-override on convergent micro-edits).

## Process Metadata

- **Discussion rounds**: 2 (Round 1 independent research → Round 2 cross-examination + position updates)
- **Framing-review reruns**: 3 (Round 0 rerun #0/#1/#2 — converged from 4 REVISE → 3 REVISE → 2 REVISE on micro-edits → override)
- **Topics**: 5 total (5 converged; 0 spawned sub-discussions; 0 deferred)
- **Autonomous TL decisions**: 3 (T1, T3, T5 — converged with strong team evidence + reversibility-aware rationale)
- **User escalations**: 2 (T2 trait pattern, T4 source-boundary reading — both genuine 4v2/3v3 splits with substantive evidence on both sides)
- **Position updates during Round 2 cross-examination**: codex-proxy=3, gemini-proxy=4, challenger=4, system-architect=3, ai-engineer=2, minimal-change-engineer=2 (reinforcement). Cross-examination working as intended — agents updated based on archaeologist's verified facts.
- **Verification artifacts produced**: 7 file:line-cited facts from archaeologist's two rounds + 028 no-ACK lock language verbatim
- **Spawned BLs**: 4 (BL-022, BL-023, BL-024, BL-025)
- **Doodlestein post-conclusion challenges**: 3 raised (strategic / adversarial / regret); 3 integrated by direct edit; 0 reopened topics; 0 new rounds. Findings improved 2 reopening triggers (T2 + T3) and 1 BL trigger condition (BL-025).
- **Knowledge capture (Step 8.5)**: SKIPPED — `memory_search` / `memory_ingest` MCP tools not registered in this Claude Code session (consistent with what `analysis.md` already noted at L21-24).

## Next Steps

→ `/ae:plan` for v0.0.1 Phase 1 BL filing (per `docs/v0.0.1-rebuild-plan.md`). The 5 converged decisions feed directly into Phase 1 architectural BLs.
→ Co-commit BL-023 + BL-025 (project_id staleness fix lands as part of AE-plugin per-skill wiring).
→ BL-022 synthesis-embedding fix lands before any cron-default reconsideration or T5 synthesis-influencing-search metric work.
→ BL-024 stays in `unscheduled/` — file individual trigger BLs per the documented per-trigger fire conditions.
→ Update `docs/blueprint.md` §8 to mark all five questions resolved (link to this conclusion); consider promoting blueprint to v0.3.

---

## 2026-05-05 Post-Conclusion Note — Thesis Clarification

Several hours after this conclusion was written + Doodlestein-reviewed, the operator clarified the v0.0.1 thesis in chat:

> **v0.0.1 的目标就是要有个最小可能用的，但避免以后自己重复造轮子的 AE 大脑**
>
> *(v0.0.1 thesis: a minimum-viable AE-brain that avoids re-inventing wheels in future.)*

**What this thesis means in code-scope terms** (per `docs/discussions/026-rust-oss-survey/analysis.md` library scorecard, which had already settled most verdicts at analysis time):

- **Keep all in-house code that already works** — fastembed-rs (in use), FTS5, db.rs, schema.rs, ingest.rs, mcp_tools.rs, parser.rs, dreaming.rs, clustering.rs, synthesis.rs (main pipeline), contradiction.rs, llm.rs::ClaudeCliProvider, F-002 audit substrate. Karpathy "don't refactor things that aren't broken" applies.
- **Adopt OSS only where it prevents re-inventing wheels** — vector.rs (264 LoC) → **sqlite-vec** (qualified ADOPT, pending 15-min static-vs-dynamic-link spike); synthesis.rs JSON parser (~100 LoC brace-depth) → **rig::Extractor** (CONTINGENT-ADOPT, pending 50-line subprocess-streaming spike); optional **async-openai** as a second `LlmProvider` impl alongside `ClaudeCliProvider` for the local oMLX endpoint.
- **Rejected by 026 analysis**: swiftide (pre-1.0 churn + storage adapter mismatch), Qdrant (single-binary violation), candle / mistral.rs / ollama-rs (post-v0.0.1), arroy (build-once incompatible), duckdb-rs (wrong shape), community Anthropic clients (immature).
- **Deferred with trigger**: LanceDB (corpus >100k OR p95 vector latency >50ms), Tantivy (multilingual query F1 <0.7 on a measured test set OR corpus >5M tokens). Thresholds calibrated for personal-KB scale (current corpus = 214 memories).

**Cargo.toml net change**: 1–3 lines added (**contingent on BL-026 + BL-027 spike outcomes**; both spikes are still pending — see "Spike-pending caveat" below).
**src/ touched**: ~200–500 LoC under spike-PASS assumption.
**Not** a "rip out and replace" rebuild.

### Spike-pending caveat (added 2026-05-06 per /ae:code-review Track 4 finding)

The Cargo / LoC / "1-2 weeks to ship" estimates above assume both Phase 1 spikes PASS:

- **BL-026** sqlite-vec adoption — gated on the 15-min static-vs-dynamic-link spike (does the rusqlite extension load statically into the binary, or does it require a runtime `.dylib`?). FAIL → fall back to keeping `vector.rs` as-is and revisit when LanceDB triggers fire (corpus >100k OR p95 latency >50ms).
- **BL-027** rig::Extractor adoption — gated on the 50-line subprocess-streaming spike (does `rig::Extractor<SynthesisDraft>` parse `claude -p` subprocess output correctly?). FAIL → keep brace-depth JSON parser in `synthesis.rs`.

If both FAIL, v0.0.1 ships with zero OSS adoptions on the mengdie side — only the AE-plugin wiring (BL-008 + BL-025 + BL-023) makes the v0.0.1 deliverable. Estimate becomes "AE wiring + cleanup BLs only", ~1 week.

### What this means for the 5 decisions in this conclusion

**All five are fully valid under this thesis** — none of them depend on the specific implementation of vector.rs or the synthesis JSON parser:

| Decision | Status under thesis |
|---|---|
| **T1 Push-primary** | ✓ Valid. Push delivery mode is independent of which vector store backs retrieval. Watcher.rs as opt-in library remains correct. |
| **T2 On-demand default + `ReflectionTrigger` trait** | ✓ Valid. Reflection / synthesis stays in-house (dreaming.rs / clustering.rs / synthesis.rs — all KEEP per 026). The trait abstracts an in-house implementation that is being kept. |
| **T3 Ratify §5 per-project** | ✓ Valid. Per-project namespace decision is policy-level, independent of轮子. |
| **T4 Extraction-discipline** | ✓ Valid. `source_type` enum + the `unknown` → `direct` rename happens in the kept schema. |
| **T5 F-002 nonempty rate + ae:retrospect + falsification rule** | ✓ Valid. F-002 audit substrate stays. Nonempty rate is computed from kept `audit_returned_facts` table. |

**Reopening triggers** (T2/T3/T4) remain valid as written.

### Spawned BLs status

- **BL-022** (synthesis re-embed pass) — **active (open)**. synthesis.rs stays in-house; the embedding=None bug is real and gates synthesis-influencing-search rate metric.
- **BL-023** (project_id cwd-switch staleness) — **active (open)**. Independent of轮子.
- **BL-024** (reflection trigger feasibility — salience/composite/debounced) — **active (deferred-with-trigger)**. The `ReflectionTrigger` trait is being built; per-trigger feasibility studies fire on documented conditions.
- **BL-025** (AE plugin per-skill `memory_ingest` wiring) — **active (open)**. Per operator priority decision: AE-side integration work is sequenced AFTER mengdie-side OSS swaps (sqlite-vec + rig::Extractor spike), to keep the interface contract stable when AE plugin starts calling.

### New BLs to file (post-conclusion, per 026 analysis recommendations)

- **BL-026** (filed 2026-05-06) — sqlite-vec adoption: 15-min static-vs-dynamic-link spike + replace vector.rs (~50 LoC swap on PASS_STATIC). Cargo.toml +1 dep on PASS. See `docs/backlog/unscheduled/BL-026-sqlite-vec-adoption-replace-vector-rs.md`.
- **BL-027** (filed 2026-05-06) — rig::Extractor verification spike (50-line proof subprocess-streaming) + conditional adoption replacing synthesis.rs JSON parser. See `docs/backlog/unscheduled/BL-027-rig-extractor-synthesis-json-parser.md`.

Both are Phase 1 sprint candidates for the v0.0.1 sprint's next-wave commitment.
