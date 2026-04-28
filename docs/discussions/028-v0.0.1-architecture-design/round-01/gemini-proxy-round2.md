---
id: "028"
round: 2
reviewer: gemini-proxy
role: Google ecosystem perspective (continued)
date: 2026-04-28
status: complete
---

# Round 2 — Gemini Proxy: Engagement with Peer Positions

## Engagement Point 1: Storage trait mechanism — defending conditional accept

**Peer positions (4-of-5 against):**
- architecture-reviewer.md:62-71: free functions over `&Db` are adequate; defer trait to Tier 2 trigger
- minimal-change-engineer.md:45-69: ≥2-impls rule (YAGNI); no second impl committed
- challenger.md:34-47: free functions sufficient; if Storage trait, must name second impl for same sprint
- codex-proxy.md:35-44: do not introduce public Storage trait without 2 credible impls

**Gemini position: HOLD CONDITIONAL ACCEPT. The peer positions misapply YAGNI to architectural boundaries.**

**Reasoning from Google ecosystem:**

The ≥2-impls heuristic is valid for *features* and *implementation details*, but **not for architectural boundaries**. Google's practice strongly differentiates:

1. **API-first architectural design (Google's standard):**
   - gRPC services define interfaces (proto definitions) *before* implementations
   - The interface's purpose is to establish a contract and boundary, not to enable immediate swappability
   - Multiple implementations validate the contract; single impl establishes the boundary

2. **When interfaces are introduced early (Google pattern):**
   - Foundational service contracts (required for team autonomy, modularity, long-term evolution)
   - Clear component boundaries (the trait's purpose: separating Storage from Retrieval)
   - Systems expecting multi-backend evolution (explicitly planned, like Mengdie's §7 ladder)

3. **Distinction YAGNI misses:**
   - **Feature YAGNI:** "Don't build search-split without knowing it's needed" ← sound
   - **Boundary YAGNI:** "Don't define Storage trait without 2 impls existing" ← misapplied
   - The trait's value is in establishing a *contract*, not in immediate swappability

**The specific context for Mengdie:**

- Search-split *is* in v0.0.1 scope (fixing two-ingest-paths defect touches `mcp_tools.rs` anyway)
- The refactor makes Retrieval a real boundary — defining the trait at that moment is *earned*
- Blueprint §7 explicitly plans Tier 2 (Kuzu) as a future implementation — the ladder exists
- Deferring the trait until Tier 2 fires means re-doing the boundary work twice: once as free functions, once as a trait

**Decision rule:** If the search-split is confirmed in-scope for v0.0.1, define `Storage` trait in the same change. The trait earns its keep by making the boundary explicit and supporting the planned evolution. Free functions are an intermediate step that postpones the inevitable refactor.

**Mechanism:** Rust trait (not struct, not free functions alone).

---

## Engagement Point 2: Bi-temporal `event_time` — picking a side

**Peer positions:**
- architecture-reviewer.md:74-115: REJECT permanently; propose optional `valid_from` override on `memory_ingest` for bulk import (the actual use case)
- minimal-change-engineer.md:73-112: REJECT permanently; "Dead schema is maintenance burden"
- challenger.md:60-94: Maintain reject unless operator provides 60-second counter-example
- codex-proxy.md:122-134: DEFER with trigger (first artifact with >60s gap)

**Gemini position: AGREE with arch-reviewer / minimal-change / challenger. REJECT PERMANENTLY.**

**Reasoning from Google temporal-database research:**

1. **Determinism vs. heuristic latency:**
   - In systems where artifacts are generated and ingested within seconds, `event_time` is *semantically identical* to `ingested_at`
   - Google distinguishes logical time (causality) from wall-clock time (physicality). If the gap is negligible, `event_time` adds no entropy
   - Introducing redundant temporal columns creates **Clock Skew Vulnerability**: if an operator manually overrides `event_time` later, it becomes a source of truth-conflict

2. **Ghost schema problem (Minimal-Change principle):**
   - Including `event_time` when identical to `ingested_at` violates schema parsimony
   - Future engineers will ask "What's the difference?" and the answer "Nothing in current use" is technical debt
   - Every downstream partition, index, and query path now carries this redundancy

3. **Falsifiability test (already failed):**
   - The test is exact: show 1 AE artifact where `event_time ≠ ingested_at` by >60 seconds
   - No evidence presented. The exception (post-hoc documentation) is not on the operator's roadmap (blueprint §9 locks it out)
   - Deferring "with trigger" on a workflow not planned is a trap — the trigger may never fire, leaving dead schema permanently

4. **Google practice: Evolutionary architecture with strict SSOT:**
   - Design for immutability and single source of truth
   - If current workload is machine-generated (real-time), `ingested_at` is ground truth
   - If future requires bi-temporal model, the schema can evolve via migration or sidecar metadata — no need to embed it now
   - **REJECT forces the requirement to be formalized; DEFER smuggles complexity into the baseline**

**Alternative for the legitimate use case (bulk import):**

Adopt architecture-reviewer.md:106-111 — add an optional `valid_from` parameter to `memory_ingest` MCP tool. This handles the only real scenario (importing historical artifacts) without schema churn:

```
memory_ingest(artifact, valid_from: Option<DateTime>) → Result
```

If caller omits `valid_from`, default to current time. If provided, use it. Cost: one parameter addition. Benefit: handles bulk import without schema speculation.

**Decision:** REJECT `event_time` column permanently. File the optional `valid_from` parameter as v0.0.1 scope. If post-hoc documentation workflow emerges later, re-open via a new discussion with measured evidence.

---

## Engagement Point 3: Reflector trait — UAG affirmation

**Peer consensus (4-of-5 explicit, 1 silent = gemini):**
- architecture-reviewer.md:119-180: Reflector trait NO (ANN swap ≠ 2nd strategy)
- minimal-change-engineer.md:142-157: Still no (swap point is one function; YAGNI)
- challenger.md:113-151: NO (demand runtime call site selecting strategies)
- codex-proxy.md:154-176: NO (only one strategy today)
- gemini-proxy.md Round 1: (silent)

**Gemini position: AFFIRM NO. Reflector trait deferred regardless of sqlite-vec outcome.**

**Reasoning from Google reflection/memory research:**

The distinction between *backend swap* and *algorithmic divergence* is critical:

1. **Current reflection algorithm:** cluster (cosine) → synthesize (LLM) → store
2. **If sqlite-vec succeeds:** cluster (ANN via sqlite-vec) → synthesize (LLM) → store
   - Only the "how we find neighbors" changes
   - The reflection algorithm's *identity* remains the same
   - This is a **computational optimization**, not a strategy divergence

3. **What would be a second strategy:**
   - (A) Cluster → Synthesize (existing)
   - (B) Temporal Decay (different algorithm, different purpose)
   - These are candidates. sqlite-vec ANN is not.

4. **YAGNI for traits applied correctly:**
   - A call site that *selects* strategies at runtime (e.g., CLI `--strategy` flag with ≥2 options) would justify the trait
   - Current architecture: all reflection passes run in sequence from one entry point (`dream` subcommand, launchd cron)
   - No caller selects strategies; they're composed, not polymorphic

5. **Google pattern:** Trait introduction requires either a real call site selecting implementations OR a semantic boundary that's actively burden-reducing. sqlite-vec introduces neither.

**Decision:** Defer Reflector trait. 
- If sqlite-vec succeeds: clustering is deleted (not merged); synthesis remains; Reflector still deferred
- If sqlite-vec defers: consolidation revisited in v0.0.2; Reflector still deferred
- Trigger for eventual Reflector trait: a second fundamentally different reflection strategy is implemented AND a call site exists that needs to select between them

---

## Engagement Point 4: A-MEM trigger measurement — simplest infrastructure-free option

**Peer positions (widely fragmented):**
- architecture-reviewer.md:199-221: 3-AND composite (top-3 score <0.35 + corpus >500 + cluster >5)
- minimal-change-engineer.md:159-205: 3-AND composite (corpus ≥1k + >=5 stale-retrieval + paper replication)
- challenger.md:186-205: Precision-based (ACK/returned <0.5) OR corpus-only if MCP ACK not in contract
- codex-proxy.md:252-266: 5-AND composite (corpus ≥10k + insufficient_context ≥15% + simpler tuning <5pp + ablation ≥8pp)

**Challenge:** None of these are "infrastructure-free." A-MEM trigger must be measurable from domain audit table + corpus size alone (no eval framework, no ablation code, no MCP protocol changes).

**Gemini position: Simplest measurable trigger is a HYBRID density-drift metric.**

**Proposed trigger (A-MEM Density-Drift):**

```
Trigger fires when ALL of:
1. Corpus >= 1,000 facts
2. Domain audit shows >= 5 "superseded-within-7-days" cases in rolling 30-day window
```

**Why this wins:**

1. **Infrastructure-free:**
   - "Superseded-within-7-days" already tracked by domain audit table (when a fact is invalidated shortly after being used)
   - Corpus size is standard metadata
   - No new code, no eval framework, no ablation runs needed

2. **Implicit precision signal:**
   - If system frequently overwrites its own knowledge or gets corrected within a week, retrieval is pulling stale/conflicting data
   - This is a proxy for "quality degraded" without requiring explicit feedback

3. **Statistical significance:**
   - Corpus ≥1k ensures superseded cases are not noise
   - 5 cases/30d is meaningful signal (one per week) without being hair-trigger

4. **Measurable in v0.0.1:**
   - One SQL query against existing audit table
   - One metadata check (corpus size)
   - No dependencies on MCP protocol changes, eval infrastructure, or external papers

**Why others fail the constraint:**

- **architecture-reviewer:** Requires "entity cluster size calculation" (NLP/clustering step not in basic audit table)
- **challenger:** Requires "ACK/returned ratio" (depends on MCP ACK feedback not yet in contract)
- **codex-proxy:** Requires "insufficient_context ≥15% on eval" (eval framework not in v0.0.1 scope)
- **minimal-change:** Requires "independent A-MEM paper replication" (external dependency, not measurable by operator)

**Decision:** Define A-MEM trigger as **corpus ≥1k AND ≥5 superseded-within-7-days facts in 30d window**. Measurable, infrastructure-free, aligns with Google's pattern of using operational signals (not external validation) to trigger upgrades.

---

## Summary: Round 2 positions

| Topic | Position | Reasoning |
|---|---|---|
| 1. Storage trait | CONDITIONAL ACCEPT | Google API-first design supports trait introduction when boundary is intentional (search-split in scope) |
| 2. Bi-temporal event_time | REJECT PERMANENTLY | Ghost schema with no current workflow; optional `valid_from` parameter covers bulk import case |
| 3. Reflector trait | DEFER (affirm NO) | sqlite-vec ANN is backend swap, not algorithmic divergence; no call site selects strategies |
| 4. A-MEM trigger | Density-drift composite (corpus ≥1k + ≥5 superseded/30d) | Infrastructure-free, measurable from audit table, statistically meaningful |

---

## Agreements with peer positions

All four engagement points converge or clarify toward decisions:

1. **Storage trait + search-split** are entangled (if search-split in v0.0.1, define trait; if not, defer both). TL decides scope.
2. **Bi-temporal column:** 4-of-5 reject (including gemini Round 2). Codex defers alone; not consensus.
3. **Reflector trait:** 5-of-5 defer / NO (gemini affirms Round 2). **Eligible for UAG.** Consolidation also 5-of-5 defer.
4. **A-MEM trigger:** Converging on corpus threshold + operational signal. Simplest infrastructure-free version: density-drift.

---

## Open question for TL

**Storage trait scope gate:** Is the search-split refactor confirmed in v0.0.1 scope, or deferred? This decision (TL call) determines whether Storage trait is v0.0.1 or Tier-2-trigger. If in scope, define trait. If deferred, use free functions.
