---
round: 1
agent: codex-proxy
lens: OpenAI ecosystem / industry production patterns
date: 2026-04-28
status: complete
---

# Round 1: codex-proxy research findings

## Topic 1: Storage abstraction — timing + mechanism

### Industry pattern for 10k LoC scale

**Verdict:** Do not introduce a public `Storage` trait in v0.0.1 unless mengdie already has ≥2 credible implementations with shared tests. Defer the trait.

**Mature library patterns (serde, tokio, sqlx, tantivy, lancedb, object_store):**

- **Serde** (the clean "trait is worth it" example): traits are core interoperability boundaries *between stable domains* (data structures vs serialization formats). Trait introduction is justified when it's the semantic boundary, not a speculative backend swap.
- **Tokio**: uses narrow core traits (`AsyncRead`/`AsyncWrite`) plus extension traits for convenience. Extension methods on generic types are correct, but they don't solve the "storage swappability" problem.
- **sqlx**: heavy trait use, but docs show the cost. `Executor` trait is not object-safe in all variants (e.g., `PgExecutor`); some impls were deleted because they didn't fit newer crate architecture without rewriting the trait.
- **Tantivy** (most relevant: search + storage): separates storage (`Directory` trait for WORM index) from search (flows through `IndexReader`, `Searcher`, `Query`, `Collector`). Search is not grafted onto a generic DB object.
- **LanceDB** (vector + AI memory): separates concepts (`connection/database/table/query`); search is a table query builder (`table.query().nearest_to(...)`), not `Db::search(...)`.
- **object_store**: mature storage abstraction, but *only* because storage interchange is the product. Default trait methods were moved to `ObjectStoreExt` because defaults were error-prone for implementors.

**The "search grafted onto Db" problem:**

No public Rust postmortem found saying "we put search extension methods on `Db` and it broke us," but the architectural pattern is well understood as harmful:

- `Db` becomes a service locator for storage, indexing, query planning, ranking, and schema concerns.
- Search API changes force `Db` churn.
- Storage "swappability" becomes fake because search depends on index-specific behavior.
- Testing degrades: mocking `Storage` is easy; search behavior actually depends on query/index semantics.
- Extension trait methods often aren't dyn-compatible.

**Decision rule for introducing `Storage` trait:**

Introduce now only if **all** are true:

1. Two *real* implementations exist (not test mocks).
2. The contract is narrow and semantic (WORM, key-value, object store, append log).
3. Search can be implemented without downcasts or `NotImplemented` branches.
4. A shared conformance test suite runs against every impl.
5. You'll support this as public API through early releases.

Otherwise, prefer modular API:

```rust
pub mod search {
    pub fn search(db: &Db, query: SearchQuery) -> Result<SearchResults> { ... }
}
// or
let searcher = db.searcher();
searcher.search(query)?;
```

Keep storage concrete or crate-private:

```rust
pub struct Db { storage: StorageEngine }
// or
pub(crate) trait StorageBackend { ... }
```

### Applied to mengdie

**For v0.0.1:**

1. Do **not** introduce a public `Storage` trait yet (only one impl: SQLite).
2. Split search functions to module-level API if they're already conceptually independent (good idea regardless).
3. Keep storage concrete internally; promote the trait later when Tier 2 (Kuzu) proves the contract.
4. A private `StorageBackend` trait is fine for internal refactoring.

**Cost/benefit:**

- Cost of deferring: the mcp_tools two-ingest-paths defect must be fixed anyway; fixing it touches callers; search-split would change callers again.
- Benefit of deferring: no public API churn if Tier 2 revises the boundary; no maintenance burden of a premature trait.

---

## Topic 2: Bi-temporal schema — event_time vs ingested_at

### When `event_time != ingested_at` matters

**Real production cases where the gap is material:**

1. **Backfill/import of historical records** (Graphiti explicitly documents this): old articles, chat/support interactions, bulk imports, structured product updates.
2. **Late or out-of-order events** (streaming systems): events can arrive late, be replayed, or arrive out of order. Flink, Google Dataflow, and streaming consensus all distinguish event time from processing time because the semantics differ.
3. **Corrections and retroactive facts** (XTDB, temporal DB literature): updates learned later, errors corrected retroactively, future-effective changes scheduled.
4. **Audit / decision replay** (regulated domains): "what did we know then?" (transaction time) vs "what was actually true then?" (valid time). Standard in financial, medical, compliance, and record-keeping systems.
5. **Temporal memory invalidation** (Graphiti/Zep): preserving changed facts instead of overwriting them requires storing when facts became/ceased-to-be-valid.

### Evidence on "regret"

No public postmortem found saying "we added bitemporal and later removed it." What's well documented is the operational burden:

- **SQL Server temporal tables**: can grow large, with significant storage cost and temporal-query performance tax. Complex constraints: no primary/foreign/check on history, limited cascade, no direct modification, blob implications, special partitioning/indexing needs.
- **General bitemporal modeling**: adds integrity complexity (overlap/gap checks, temporal referential integrity, harder maintenance).
- **Graphiti-specific cost**: bulk ingestion doesn't perform edge invalidation; custom entity types require re-ingesting into a new graph.

The "regret" pattern is usually not philosophical; it's schema/query/storage complexity without enough queries using the second time axis.

### AI/LLM workflow reality

For **native chat turns and immediate tool artifacts**, the gap is typically seconds. OpenAI's own conversation state and agent memory primitives are mostly about conversation/thread state, prior-response chaining, compaction, and retrieval—not general bi-temporal schema.

For **agent memory systems integrating external state** (old chats, uploaded docs, CRM/billing/support data, delayed analytics, user corrections, business facts with effective dates), the gap becomes real. Zep/Graphiti pitch exactly this enterprise-memory setting.

**Threshold for adoption:**

There is no universal corpus-size threshold. A 200-row compliance ledger with retroactive corrections may need bi-temporal history; a 10M-row append-only artifact log may not.

Use bi-temporal if you need queries like:
- "as of business time"
- "as known at decision time"
- "with effect from"
- "before correction"
- "after late import"

Avoid it for AE artifacts if `event_time` and `ingested_at` differ only by seconds and no product behavior depends on that distinction.

### Applied to mengdie

**For v0.0.1:**

1. Do **not** include `event_time` as a first-class schema column. The AE workflow produces artifacts in near-real-time (generation + ingestion within seconds).
2. Keep a single `created_at/ingested_at` timestamp.
3. Optional: add `occurred_at` or `source_timestamp` metadata for sources that can actually be delayed/backfilled, but not as part of core query semantics yet.
4. Add a concrete trigger to the backlog: `"Add bi-temporal schema when the operator ingests the first artifact whose creation time and underlying decision time differ by > 60 seconds in production, or when post-hoc documentation becomes a regular workflow."`

**Cost/benefit:**

- Cost of including it now: migration logic even at small scale; index complexity; query burden without queries.
- Benefit of deferring: when the gap becomes real and measurable, you'll have data to shape the schema instead of guessing.

---

## Topic 3: Reflection module consolidation + Reflector trait

### Pre-1.0 Rust consolidation patterns

**Verdict:** Defer consolidation until the sqlite-vec spike resolves. Do not introduce `Reflector` yet.

**When consolidation is worth doing now:**

Pre-1.0 Rust SemVer treats public item moves/removals as breaking (the `y` component in `0.y.z` acts as major version). Consolidation is justified when:

1. The boundary is actively misleading.
2. The modules are unlikely to be deleted.
3. You're committed to supporting the boundary through releases.

For mengdie: consolidation is not worth doing before the sqlite-vec result because clustering may be deleted, not moved. Consolidating a module that will disappear adds extra work.

**Readiness signals for a trait abstraction:**

A `Reflector` trait is ready when there are *two distinct reflection strategies*, not merely two implementations of one step.

Signals of readiness:

- Two strategies coexist (not one replacing the other).
- A real call site chooses between them by config, runtime condition, experiment, or fallback.
- Both have tests/evals and are expected to survive multiple sprints.
- The shared contract is stable enough to name.
- The trait boundary is at the right level.

sqlite-vec ANN does **not** satisfy this: it swaps the neighbor-finding primitive within the same `cluster → synthesize → store` strategy, not a distinct alternative path.

### Applied to mengdie

**For v0.0.1:**

1. Keep modules separate: `clustering.rs`, `synthesis.rs`, `dreaming.rs`.
2. Mark clustering `CONDITIONAL-DELETE`: if sqlite-vec ANN replaces hand-rolled neighbor finding, delete clustering; if deferred/rejected, revisit consolidation later.
3. Do **not** introduce a `Reflector` trait yet (only one strategy).
4. After sqlite-vec spike: if clustering survives, decide between collapse or `pub(crate)`. If sqlite-vec succeeds and another strategy emerges, then `Reflector` becomes legitimate.

**Local caveat:**

These modules are mechanically public today (exported in `src/core/mod.rs`; CLI imports defaults via library path). Before deferring consolidation, either:

- Make public status explicit as "unstable internal public" via docs, or
- Move constants behind `dreaming` module so public surface tightens.

**Cost/benefit:**

- Cost of early consolidation: rewrites internal code; loses the module boundary (useful during replacement spike).
- Benefit of deferring: isolates the deletion target; simplifies replacement experiments.

---

## Topic 4: A-MEM bidirectional update — concrete deferral trigger

### What production systems actually use

Graphiti/Zep and Mem0 expose policy knobs, but real triggers are measured production failures.

**Zep's concrete scaling trigger** (30x usage jump):

- Context retrieval P95: 200ms → >2s.
- Episode processing: 60s.
- LLM cost: 3-5x provisioned capacity.
- Rate limits caused customer failures.

Response: separate search infra, fewer LLM calls, entropy/TF-IDF/LSH dedup, normalized relevance scoring, temporal clarity.

**Mem0's triggers** (mostly policy/use-case):

- Use graph memory when conversations mix multiple actors/objects, compliance needs relationship auditability, or teams need shared context.
- Prune stale graph relationships to save latency.
- Update when user changes preference or clarifies a fact; pair with feedback signals for self-healing.

### Operationalizing "retrieval quality degraded"

Use a small eval set, not vague intuition.

**Recommended metrics at small corpus scale:**

1. `context_completeness_rate`: percent of eval questions whose retrieved facts are sufficient to answer (primary).
2. `insufficient_context_rate`: percent judged `INSUFFICIENT`.
3. `hit@k` / `recall@k`: only when expected memory IDs are known.
4. `MRR`: when the right memory is too low in rank.
5. `answer_accuracy`: secondary (model can fail despite good retrieval).
6. `tokens_per_answer` + `retrieval_latency_p95`: prevent "quality" improvements that just stuff more context.

At small corpus scale, use thresholds like "fails for 2 consecutive runs" or "at least N missed cases" rather than a single percentage on 20 examples.

### A-MEM adoption practice (unproven patterns)

A-MEM is promising but paper-first. The paper reports ablation gains and benchmark evidence, but no independent replication yet.

**Industry practice for unproven patterns:**

1. Require reproducibility on your workload, not just the paper benchmark.
2. Require equal-budget comparison: same context budget, latency, model, eval set.
3. Ship behind an offline job or feature flag first.
4. Adopt only if it beats simpler alternatives: better extraction, reranker, hybrid BM25+vector, metadata filters, dedup, temporal invalidation.
5. "Independent replication" is a confidence boost, not the only trigger.

### Corpus size guidance

`>1k facts` is too low as an automatic trigger. A-MEM's own scaling table reports tiny retrieval/storage overhead at 1k memories; the real cost is write-path LLM work (~1,200 tokens per operation, ~5.4s with GPT-4o-mini) and semantic drift.

Better guardrails:

- `<1k facts`: definitely premature unless eval failures are severe.
- `1k-10k facts`: tune retrieval, reranking, extraction, dedup, temporal policy first.
- `10k+ facts` or `1M+ stored source tokens`: start offline A-MEM experiments if multi-hop/temporal failures are visible.
- `50k+ facts` or high write rate: bidirectional update may become useful, but only if async/batched.

### Applied to mengdie

**Concrete deferral trigger for the backlog:**

Enable an A-MEM experiment (not production default) when **all** are true:

1. `memory_fact_count >= 10_000` OR `stored_source_tokens >= 1_000_000`
2. `insufficient_context_rate >= 15%` on a fixed 50+ question memory eval for 2 consecutive runs
3. Failures are concentrated in multi-hop, temporal, contradiction/update, or entity-linking questions
4. Simpler retrieval changes improve completeness by `<5 percentage points`
5. A-MEM offline ablation improves context completeness by `>=8 percentage points` at `<=1.5x` write cost and no worse P95 read latency

Production enablement gate:

> A-MEM runs async behind a feature flag and maintains `p95 retrieval latency`, `tokens/query`, `correction rate`, and `manual drift findings` within agreed limits for 2 weeks.

This balances evidence-driven iteration with optimism about a plausible pattern.

---

## Summary of agreements with team

- **Storage trait:** defer; split search to module API for v0.0.1 if needed for the two-ingest-paths fix; keep storage concrete.
- **Bi-temporal schema:** defer; keep single `ingested_at` for v0.0.1; define trigger for later adoption.
- **Reflection consolidation:** defer; keep modules separate; mark clustering `CONDITIONAL-DELETE`; do not introduce `Reflector` trait yet.
- **A-MEM trigger:** use measured signals (corpus size + eval quality degradation + feature-flag offline test) instead of optimism; concrete trigger defined for backlog entry.

All four decisions align with the analyze-phase outcomes and the industry pattern of "defer until you have measured pain or committed impls."

---

## Disagreements and open questions

No disagreements with prior findings. Three clarifications for Round 1 discussion:

1. **Storage trait and search-split coupling:** architecture-reviewer said trait is conditional on search-split refactor. Codex analysis agrees but adds: search-split is justified on its own merits (API clarity), independent of the trait. Recommend doing search-split for v0.0.1 (because mcp_tools two-ingest-paths must be fixed anyway) without coupling it to trait introduction. The trait decision becomes a separate follow-up.

2. **Reflector trait and sqlite-vec spike:** the spike may or may not produce a second reflection strategy. If it does (e.g., ANN-based clustering replaces hand-rolled), the trigger condition for `Reflector` is not "2 impls exist" alone, but "2 *distinct strategies* with a real selection point." ANN is a better algorithm for the same purpose; it's not an alternative strategy. If sqlite-vec forces a redesign and a genuinely different reflection approach emerges, that's when `Reflector` becomes legitimate.

3. **Public surface tightness (clustering/synthesis):** these modules are mechanically public via `src/core/mod.rs` exports, but docs don't mark them "unstable internal public." Before deferring consolidation, consider making the boundary explicit or moving constants behind `dreaming` to reduce accidental public API surface.

---

## Codex confidence level

Medium-to-high. Findings are grounded in released library source, docs, and well-established patterns in the Rust ecosystem. The main uncertainty is mengdie's specific future (how likely is sqlite-vec adoption to succeed, how confident are you in long-term A-MEM benefit), which is outside Codex's lens.
