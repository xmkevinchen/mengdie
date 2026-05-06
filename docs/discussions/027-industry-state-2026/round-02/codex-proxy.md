---
round: 2
agent: codex-proxy
perspective: openai-family
date: 2026-05-05
---

# Round 2 — CODEX-PROXY cross-examination response

## Preamble

Round 2 cross-examines five targets against OpenAI-family patterns. Critical update from archaeologist verification: **cron is NOT actually running**. com.mengdie.dream.plist is a template (not wired); the 13 syntheses came from on-demand CLI invocation. This falsifies the "both cron and on-demand shipped" premise that 4 agents (including myself) inherited from the framing. Updated positions below.

---

## Cross-exam Target 1: T2 cron-not-shipped

**Restatement of my Round 1 position (codex-proxy.md Topic 2):**
> "Cron + on-demand (hybrid, operator-controlled) is the v0.0.1 default."
> Rationale: "cron is predictable... ChatGPT Memory's silence on triggers suggests no single trigger works... operator needs predictable cost."

**Archaeologist's verification (archaeologist.md:72-77)**:
- `com.mengdie.dream.plist:8-19` is a **template** with placeholder comment `<!-- Update this path to your built binary -->`.
- "Whether it is actually loaded in launchd on the operator's machine is not verifiable from code."
- The "13 syntheses" first run was manual on-demand CLI, not cron-fired.
- **Verdict: cron is half-shipped (logic exists, trigger NOT wired).**

**Impact on my Round 1 verdict:**

My recommendation was grounded in the assumption that both cron and on-demand were operational baselines. That assumption is **false for cron**. On-demand IS the operational baseline. Cron is a *proposed* trigger, not a proven one.

**Revised position for T2:**

**AI-engineer's trait-based proposal (ai-engineer.md:139-145)** is now the stronger architecture:
- Ship **on-demand as the actual v0.0.1 default** (it works today).
- Implement a `ReflectionTrigger` trait (ai-engineer's ~80 LoC proposal, paralleling `LlmProvider`).
- File salience/composite/debounced as post-v0.0.1 BLs.
- **Cron becomes an opt-in trigger** (via launchd plist) rather than a co-default.

This matches the **OpenAI Responses API pattern** (state moves to the server, developers stop rebuilding plumbing) — on-demand reflects operator agency; cron is background automation. For a developer tool at solo-operator scale, on-demand-as-default is the right inversion of the v0.x assumption.

**Key quote from ai-engineer.md:98-104:**
> "On-demand has a hidden virtue: it is the only candidate where the operator's engagement is the trigger... for a *solo-operator developer-tool*, on-demand is not a degenerate case — it is an operator-in-the-loop trigger that aligns reflection cost with operator attention."

**OpenAI ecosystem alignment:**
- Vector Stores API (push-driven, user-initiated) defaults to explicit trigger, not ambient background.
- ChatGPT Memory (opaque trigger, possibly debounced at session boundary) feels like on-demand-first to users — they explicitly save facts, not the other way.
- Responses API (state on server, developer manages orchestration) means reflection orchestration is the client's job, not the platform's. On-demand-as-default gives the operator explicit control.

**Falsification path I initially offered (Round 1):**
> "Reversibility: Very high. Cron runs once per day (24 invocations/month)..."
This reasoning was *assuming cron is wired*. Without that, the falsification path collapses.

**New falsification rule (from challenger.md:152-166):**
> "What metric would tell us cron is the wrong default? If we ship cron, the falsification signal is 'cron fired but produced ≤1 synthesis on N consecutive runs'."

**Agreement with ai-engineer on trait design**: The ReflectionTrigger trait coheres three truths:
1. On-demand is operationally proven (it worked for the 13 syntheses).
2. Cron *could* work, but is unproven in this deployment.
3. Salience/composite/debounced are post-v0.0.1 research territory.

The trait is the honest API: it says "pick your trigger" without claiming one is optimal.

---

## Cross-exam Target 2: T4 forward-compat split

**Restatement of my Round 1 position (codex-proxy.md Topic 4):**
> "Forward compatibility (v0.0.1 API design): Add a `source` enum in ingest schema: `{ source: "ae_plan" | "ae_review" | "ae_conclusion" | ... }`. Store source tag on every ingested memory."
> Rationale: "Source enum in schema already supports typed provenance ... When broader sources are added (commit messages, issue text), the schema already supports them."

**Minimal-change-engineer's challenge (minimal-change-engineer.md:375-386):**
> "Building flexibility for hypothetical future ingest sources is the exact failure mode the v0.0.1 rebuild is correcting against v0.x. The right move is: ratify, document the trigger, ship."
> "Typed source markers / per-source filters / generic ingest schema → API design work *now* for sources we are not adding *now*. YAGNI."

**System-architect's alignment (system-architect.md:456-501):**
> "Option A is the v0.0.1-correct choice... Topic 4 should ratify along the same axis: AE-only NOW, add typed-source-marker / per-source-schema when an actual second source is committed."
> "The 028 architecture conclusion already chose enum-shape decisions of this kind..."

**Archaeologist's discovery (archaeologist.md:269-275):**
> "`infer_source_type` returns `"unknown"` for non-matching filenames and `knowledge_type="factual"`. Files named `notes.md` or `BL-007.md` would be ingested as `source_type="unknown"`, `knowledge_type="factual"`. The schema's `ALLOWED_SOURCE_TYPES` trigger (v5 migration) would reject `"unknown"` — this is a **latent bug** in the file-ingest path for non-AE files."

**OpenAI evidence (re-examined)**:

OpenAI's **Vector Stores API does NOT ship typed source markers from day 1**. The API surface is:
```
POST /v1/vector_stores/{vector_store_id}/files
{ file_id, metadata: {...}, chunking_strategy: {...} }
```

Files can carry user-supplied `metadata` (free-form key/value), but there's no OpenAI-enforced "source type" enum. OpenAI's design is **accretive**: ship the minimal contract (file + metadata), let the caller decide what metadata means. Typed source markers come *later*, if a cross-file-type use case emerges.

**Implication**: OpenAI's pattern is the opposite of what I advocated. They start minimal (file + optional metadata), not with a forward-compat enum.

**Revised position on T4:**

**Minimal-change-engineer is correct: no forward-compat scaffolding in v0.0.1.**

Rationale:
1. **Matches OpenAI's pattern**: Ship minimal contract (AE-only ingest), add typed structure when a second source is committed.
2. **Avoids API design bias**: Any enum I design now (ae_plan, ae_review, ...) assumes the next source will fit those categories. Commit messages or debug-session facts may need different categorization entirely.
3. **The latent bug (archaeologist's finding) is orthogonal**: The `"unknown"` source_type → schema v5 rejection is a **code defect** in the file-ingest path, not a design question. Fixing it doesn't require forward-compat scaffolding (just be explicit: AE-only is policy; non-AE files are intentionally rejected).

**Codex-proxy correction**: My Round 1 position was overcautious about "v1 API break." OpenAI's own practices show that conservative forward-compat is not the default pattern. When a second source is genuinely added, the BL that adds it also extends the schema. This is not "breaking" — it's normal evolution.

**Agreement with challenger.md on one point (challenger.md:293-311):**
Challenger proposes: `"AE-only" should mean AE extraction discipline (quality gate), NOT physical AE files.` This is a reframe, not a rejection of ratifying AE-only. The resolution is:
- **v0.0.1 ingest**: only AE pipeline files (`conclusion.md`, `plan.md`, `review.md`, `retrospect.md`).
- **Manual `memory_ingest` CLI path**: operator can submit ad-hoc facts that passed AE-style extraction discipline *outside* the pipeline (e.g., distilled from a 90-minute debug session).
- **Future post-v0.0.1 sources**: any source that carries equivalent extraction discipline (LLM-mediated propositional facts, not raw content).

This is *not* forward-compat scaffolding in code; it's discipline documentation. The code path already supports it (MCP `memory_ingest` accepts any structured input).

---

## Cross-exam Target 3: T5 metric specifics

**Three proposals in Round 1:**
- **AI-engineer (ai-engineer.md:301-316)**: per-search nonempty rate (7d rolling) + qualitative retrospect verdict.
- **System-architect (system-architect.md:592-614)**: search-with-results-rate + synthesis-influencing-search rate.
- **My Round 1 (codex-proxy.md:Topic 5)**: search utilization rate + operator retro verdict.
- **Minimal-change (minimal-change.md:454-472)**: BL-014 `mengdie audit-stats` covering search-call rate, empty-result rate, repeat-query density, etc.

**Are these the same metric or different?**

Structural comparison:

| Metric | AI-eng | System-arch | Codex | Minimal-ch |
|---|---|---|---|---|
| Search-with-results rate (7d rolling) | ✓ | — | similar | empty-result rate (inverse) |
| Synthesis-influencing-search rate | — | ✓ | — | — |
| Search utilization rate (F-002) | ✓ | — | ✓ | ✓ |
| Qualitative retrospect | ✓ | — | ✓ | ✓ (implicit) |

**They are NOT the same metric.** The critical divergence is synthesis-influencing-search rate (system-architect's proposal), which requires:
- Tracking whether returned facts from `memory_search_audit` appear in later syntheses.
- Joinable via `audit_returned_facts → memory_entries.source_type = 'synthesis'`.

**But archaeologist confirms (archaeologist.md:379):**
> "Any future loop-closure signal that requires 'was this fact cited?' would need a new ingest event from the AE plugin side — mengdie has no way to observe what Claude does with search results after they are returned."

And archaeologist also notes (archaeologist.md:351-360):
> "The `metrics` table has `updated_at` but stores running totals, not per-event timestamps. Cannot compute per-day injection rates from it. No contradiction event table... No `source` column distinguishing ae:analyze injections from operator manual queries."

**Implication**: System-architect's synthesis-influencing-search rate is **not actually computable from F-002 audit data alone**. It requires:
1. A new mechanism to track "this synthesis used facts X, Y, Z from the audit table."
2. Or, reverse-engineering via embedding similarity (expensive, fuzzy).

System-architect claims `docs/discussions/029-f-002-audit-table-design/conclusion.md` and the index on `(searched_at, id)` support this. But archaeologist's verification shows no such tracking exists in schema v6.

**Revised position on T5:**

1. **AI-engineer's per-search nonempty rate + retrospect is the most defensible minimum.** Both are:
   - Computable from existing F-002 schema v6 (no new instrumentation).
   - Hard to misinterpret (nonempty = fact was returned, yes/no).
   - Forcing: operator sees the nonempty rate via `mengdie stats` or ingest response; retrospect prompts them weekly.

2. **System-architect's synthesis-influencing-search rate requires new infrastructure** that exceeds "minimal signal." Either:
   - Add a `synthesis_used_audit_ids: Vec<u64>` field to syntheses (new schema), or
   - Accept that "synthesis-influencing" is post-v0.0.1 (requires AE-side instrumentation to track "I used fact X in my output").

3. **Minimal-change's empty-result rate is the inverse of nonempty rate.** Both are saying the same thing: "did the corpus answer the query?" Nonempty is more actionable (positive signal); empty-result is more falsifiable (absence is clear).

4. **Gemini's thumbs up/down per result** (gemini-proxy.md:412-414) conflicts with **028's hard lock on ACK feedback** (minimal-change.md:429-432). Minimal-change is correct: "MCP `memory_search` ACK feedback: NOT in v0.0.1 contract." Gemini's proposal should be filed as a post-v0.0.1 BL.

**Convergence proposal for T5:**
- **Quantitative (computable from F-002)**: search-with-results-rate (nonempty rate, 7-day rolling). Falsification: <30% over 7 days.
- **Qualitative**: ae:retrospect prompt: "Did mengdie help avoid re-research this week?" Binary forced response (yes/no/unsure). Falsification: "no" twice in a row.
- **Surface**: Both inline in search output + `mengdie stats` weekly summary. No new event stream.
- **Post-v0.0.1**: synthesis-influencing-search rate, contradiction-trend, Round 0 citation rate (all require new instrumentation or AE-side hooks).

---

## Cross-exam Target 4: 028's no-ACK lock impact

**Minimal-change (minimal-change.md:429-432):**
> "028 conclusion locks: 'MCP `memory_search` ACK feedback: NOT in v0.0.1 contract. Triggers must be server-side observable.' This is a **hard constraint**."

**Gemini-proxy's T5 proposal (gemini-proxy.md:411-414):**
> "**Start with two forced signals (not optional, not deferred):**
> 1. **Thumbs up/down on every search result or synthesis.**"

**Conflict**: Thumbs up/down is per-result ACK feedback. It violates 028's lock.

**Precedent in OpenAI ecosystem**:
- ChatGPT Memory: no explicit thumbs up/down per fact (opaque auto-save).
- Vector Stores API: returns results, no feedback mechanism.
- Responses API: no built-in result-rating surface in the MCP tool contract.
- OpenAI's evals framework: gathering feedback requires a separate integration (user surveys, external annotation).

OpenAI's stance is **server-side observable, not caller-provided feedback**. This aligns with minimal-change's interpretation of 028.

**Codex-proxy verdict on gemini-proxy's proposal:**
- **Rejected for v0.0.1** (violates 028 lock).
- **File as post-v0.0.1 BL** with explicit reopening condition: "if quantitative metrics (nonempty rate + retrospect verdict) produce a verdict the operator disagrees with, add qualitative ACK signals."

---

## Cross-exam Target 5: T1 watcher.rs disposition

**Archaeologist (archaeologist.md:22-27):**
> "Zero call sites outside tests: `rg start_watcher` and `rg watch_loop` across the entire `src/` tree returns only hits inside `watcher.rs` itself... Neither `bin/mcp_server.rs` nor `bin/cli.rs` nor any other module imports or calls these functions."

**System-architect (system-architect.md:110-122):**
> "Push as v0.0.1 default. Watcher library kept as opt-in (no daemon shipped)... reversibility is high — the watcher library remains in `src/core/watcher.rs`; if a daemon is later wanted, ship one binary that wraps `start_watcher` + `watch_loop`."

**Minimal-change (minimal-change.md:72-99):**
> "Keep `core/watcher.rs` as a library with its existing tests, but do NOT wire it to a daemon in v0.0.1."

**Challenger (challenger.md:29-76):**
> "Pull (file-watcher daemon) is the architecturally sounder default for v0.0.1... 'never wired to a daemon' is a v0.x execution failure, not evidence against pull as a design."

**OpenAI pattern (re-examined)**:
- Vector Stores API: push (user submits files).
- Google Drive integration (my Round 1 research): hybrid (user picks files, system fetches).
- Responses API: no file watcher; state is caller-managed.

**Codex-proxy verdict on watcher.rs**:

Keep it as opt-in library, don't wire daemon. Reasoning:

1. **OpenAI's Vector Stores shows push is the primary pattern.** (My Round 1 codex-proxy.md was correct on this.)
2. **Challenger's architectural argument is sound** (pull is decoupled), but the *execution cost* (daemon supervision, restart-on-crash, heartbeat detection) is real. For v0.0.1, with on-demand-as-default (per revised T2), push is pragmatic.
3. **Cold-start is solved by CLI `mengdie import`** (system-architect verified this exists; minimal-change confirmed zero new code needed). The watcher's "natural replay" advantage is marginal.
4. **Delete vs keep**: I favor keeping as opt-in library. Reason: if a later v0.0.2 or post-v1 use case (e.g., a generic AI tool not tied to AE plugin) needs pull-daemon, the code already exists. Deletion is unnecessary. But document its opt-in status (e.g., `watcher.rs` top comment: "Library for file-watching daemon; not wired in v0.0.1; see BL-NNN for daemon supervisor integration").

---

## Revised v0.0.1 Verdicts (OpenAI-family perspective)

| Topic | Round 1 | Round 2 Revised | Rationale |
|---|---|---|---|
| **1: Ingest mechanism** | Push-primary | Push-primary, watcher as opt-in library | OpenAI Vector Stores = push pattern. Cold-start via CLI `mengdie import` is sufficient. |
| **2: Reflection trigger** | Cron + on-demand | On-demand as v0.0.1 default; cron opt-in (trait-based) | Cron NOT actually running. On-demand is operationally proven. Trait allows future triggers without rewrite. |
| **3: Cross-project scope** | Ratify per-project | Ratify per-project (unchanged) | OpenAI namespacing favors explicit per-namespace isolation. Challenger's cross-project reframe needs evidence, not redesign. |
| **4: Ingest source boundary** | Ratify AE-only + forward-compat | Ratify AE-only, NO forward-compat | OpenAI's Vector Stores shows accretive design (add structure when second consumer materializes). YAGNI applies. |
| **5: Loop-closure signal** | Two-signal minimum | Per-search nonempty rate (F-002) + ae:retrospect verdict | Synthesis-influencing-search requires new instrumentation (out of scope). Thumbs-up/down violates 028 lock. Minimal, computable, forcing. |

---

## Agreements (with specific peer citations)

1. **On-demand-as-default for T2**: Agree with ai-engineer.md:98-104 ("operator's engagement is the trigger"). Archaeologist's cron-not-shipped finding (archaeologist.md:72-77) makes this transition mandatory, not optional.

2. **No forward-compat scaffolding for T4**: Agree with minimal-change.md:375-386 (YAGNI) and system-architect.md:500 (128 architecture conclusion's approach). Archaeologist's latent-bug finding (archaeologist.md:269-275) is a code defect, not a design question.

3. **Per-search nonempty rate for T5**: Agree with ai-engineer.md:301-316 (computable from F-002, falsifiable, forcing). Archaeologist's verification (archaeologist.md:342-360) that synthesis-influencing-search requires new instrumentation rules out system-architect's proposal for v0.0.1.

4. **Reject thumbs up/down per result**: Agree with minimal-change.md:429-432 (028 lock) that gemini-proxy.md:412-414 violates the no-ACK constraint.

5. **Push + watcher-as-library**: Agree with minimal-change.md:72-99, system-architect.md:110-122 (push primary, watcher kept but not wired).

---

## Disagreements (with specific peer citations)

1. **Challenger on T3 (cross-project scope)**: Challenger.md:180-185 argues single-operator identity means cross-project should be DEFAULT. Codex disagrees: per-project default prevents silent cross-contamination. Operator can `--cross-project` when needed. Challenger's evidence would need to show the operator actually *uses* cross-project more often than not — F-002 audit data will tell.

2. **Challenger on T1 (pull vs push)**: Challenger.md:29-76 argues pull is "architecturally sounder." Codex disagrees: OpenAI's push pattern + v0.0.1 lacking a daemon supervisor + on-demand-as-default (T2 revised) means push is the pragmatic call. Challenger's counterargument is structurally sound but execution-cost argument outweighs it for this scope.

---

## Open Questions for Round 2 → Team consensus

1. **T2 trait design**: Should the `ReflectionTrigger` trait live in `src/core/trigger.rs` or be part of `dreaming.rs`? AI-engineer proposed the former; minimal-change's BL-014 suggestion implies it's a CLI concern. Need structural alignment.

2. **T5 baseline period**: AI-engineer and I both propose "7 days rolling window" for nonempty rate. Is 7 days meaningful on a 214-memory corpus with unknown search frequency? Should the window be configurable?

3. **028 re-open for T5**: If synthesis-influencing-search rate turns out to be post-v0.0.1 critical (after operator uses the loop for a month), does the 028 "no-ACK" lock need revision? This is not a Round 2 question; it's a precedent question for future discussions.

4. **T4 latent bug**: Should `infer_source_type` returning `"unknown"` be fixed as "reject non-AE files" or "accept any file as 'unknown' type"? The fix direction depends on whether we're closing the AE-only boundary or leaving it porous. This should be explicit in the v0.0.1 BL.

---

Sources (verified by archaeologist or re-examined this round):

- [archaeologist.md:72-77](file:line) — Cron plist is template, not wired
- [archaeologist.md:269-275](file:line) — Latent bug: AE-only is policy not enforcement
- [archaeologist.md:342-360](file:line) — F-002 does not track citation/synthesis-use
- [ai-engineer.md:98-104, 139-145](file:line) — On-demand as default + trait-based trigger
- [system-architect.md:110-122, 500](file:line) — Push primary + YAGNI on forward-compat
- [minimal-change.md:72-99, 375-386, 429-432](file:line) — Push-only + no forward-compat + 028 lock
- [gemini-proxy.md:412-414](file:line) — Thumbs-up/down (conflicts with 028)
- [challenger.md:29-76, 152-167, 180-185, 293-311](file:line) — Pull argument, cron validation, cross-project reframe, extraction discipline
