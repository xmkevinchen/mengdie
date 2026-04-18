---
id: BL-clustering-validation
status: open
origin: BL-006 /ae:review (challenger + codex-proxy)
created: 2026-04-18
---

# BL-006 clustering — design bets pending BL-007 empirical validation

Three design decisions in `src/core/clustering.rs` cannot be validated
without real mengdie memory data being clustered and fed to an LLM. BL-007
(dream synthesis) is the first such consumer; these should be re-evaluated
when its output quality is measurable.

## 1. Seed-ordering strategy

The pure-algorithm seed is the lexicographically smallest unassigned
`memory_id`. With UUID-v4 IDs this is effectively random. If empirical
cluster quality from BL-007 is poor, swap to density-weighted seeding
(pick the seed with the most above-threshold neighbors first) before
reaching for DBSCAN / connected-component.

**Trigger**: BL-007 dream outputs show unclear / duplicated / mis-grouped
summaries that trace back to cluster composition rather than prompt
wording.

## 2. Default threshold `0.75`

Borrowed from Sentence-Transformers' `community_detection` default for
`all-MiniLM-L6-v2`. SBERT's default is tuned for general sentence-level
corpora. Mengdie's corpus (AE pipeline outputs: plans, reviews,
conclusions) is denser and more homogeneous — the inter-document cosine
baseline may be elevated, making 0.75 too loose and producing
over-clustered noise.

**Trigger**: First BL-007 run on real mengdie data. Sanity-check the
cluster-count and member lists; if "everything ends up in one giant
cluster" or "related decisions land in separate clusters", sweep
{0.80, 0.85, 0.90} on the current corpus and pick the elbow.

## 3. `ClusteringResult.residuals: Vec<String>` shape

Currently a bare list. Three downstream policies are plausible (skip,
summarize pairs differently, merge to "misc"). BL-007 may need richer
data — e.g., nearest-cluster distance per residual, or sub-threshold
pair groupings — in which case the struct needs a field addition.

**Trigger**: BL-007 design decides on residual policy. If it's "skip",
leave the type alone. If it's "summarize pairs" or "misc group",
augment `residuals` to carry the info BL-007 needs (probably
`Vec<ResidualMemory { id: String, nearest_cluster: Option<usize>, nearest_score: f32 }>`).

## Also note (from the same review round)

- **SQL filter duplication** — `load_embeddings` (clustering.rs:65) and
  `search_vector` (vector.rs:54) share a nearly identical WHERE clause.
  Extract into a shared helper when BL-007 adds the third consumer, not
  before.
- **`EMBEDDING_DIM = 384` inlined in clustering.rs** — dual source of
  truth with `embeddings.rs::Embedder::new`. Promote to
  `pub const EMBEDDING_DIM: usize = 384` in `embeddings.rs` when the
  third consumer lands; a silent-zero-rows bug on model swap is the
  failure mode to watch for.

## BL-007 empirical results (first real dream run, 2026-04-18)

First `mengdie dream --synthesize` on production DB (198 eligible
memories across all projects, global scope).

**Threshold bucket observed**: threshold=0.75 (default), min_size=3,
max_cluster_size=20. Produced **14 clusters + 133 residuals** (76% of
eligible memories fell into residuals). 13 syntheses created, 1 parse
error, 0 LLM-call errors. 26 memories truncated at the 4000-char cap.

**Cluster quality judgment**: **good**. 13 synthesis titles cover
clearly distinct topics with no obvious overlap:

- SmartPal Backend Reviews & Claude CLI Provider Lifecycle
- SmartPal iOS: history, textbook/mastery UI, question revision
- SmartPal Textbook Pipeline: Architecture Decisions & Milestones
- SmartPal Backlog Strategy and Milestone Decisions (v0.0.4–v0…)
- SmartPal Home Server 部署架构：Compose、OAuth 刷新与 CLI 认证
- SmartPal 跨里程碑工程决策汇总
- AE Framework Core Architecture: Output, Writeback, Roadmap
- AE Pipeline Retrospect Aggregate: v0.0→v0.4 Snapshot (4 Feat…)
- AE Plugin: Test Framework + Review Pipeline Maturation
- AE Plugin Core Architecture Decisions — Cross-Skill Synthesis
- AE Skill Governance — Pre-checks, Hooks, and Layer 1 Gates
- ae:roadmap, ae:dashboard, ae:next — Design and Phasing Decisions
- ae:roadmap v2 Phase A/B: migration gates, semantic validation

Topical cohesion looks strong — clusters split cleanly across projects
(SmartPal / mengdie / AE plugin) and within projects by sub-concern
(architecture / plugin / skill governance for AE). Manual spot-check of
the cluster on AE skill governance produced a synthesis that correctly
consolidated the pre-check / hook / layer gate invariants across
multiple plans. No obvious hallucinations in the 3–5 rows spot-checked.

**Triggers fired**:

1. **#3 (residuals policy)** — **TRIGGER ADDRESSED by plan 011**
   (2026-04-18). 133 / 198 ≈ 67% residuals at threshold 0.75 + min_size=3
   prompted a /ae:discuss → /ae:plan cycle (discussion 018, plan 011).
   Resolution: flip `DEFAULT_MIN_SIZE` 3→2 (recovers near-duplicate pairs)
   and bundle a null-escape-hatch (`{"skip": true, "reason": ...}`) so
   the ~30% topically-adjacent pair share cleanly gets LLM-rejected
   instead of synthesized as noise. Original option (c) taken (reduce
   min_size to 2); options (a) and (b) deferred. Next signal source:
   plan 011's AC5 post-ship audit writeback in this file's new
   `## BL-residuals-reduction empirical results` section.

   **Updated signal trigger (replaces `>50% residuals`)**: `>50%
   residuals AND synthesis_hit_rate < 10% = revisit-parameter signal`.
   Note: `synthesis_hit_rate` instrumentation is deferred (no search-log
   table exists yet); use residual-% only until search logging lands
   in a future plan. The AND-conjunction form is the forward-looking
   wording once instrumentation exists.
2. **#2 (threshold validation)** — **WEAK TRIGGER**. The 14 clusters
   that DID form look topically tight. Loosening threshold to 0.70 might
   pull more memories into clusters but risks over-clustering (Project A
   + Project B notes landing together). Not an urgent change; revisit
   only if residual-reduction plan from #3 above lands and still leaves
   too many singletons.
3. **#1 (seed-ordering quality)** — **NOT FIRING**. Manual review of 3
   syntheses did not surface any clusters that obviously split related
   decisions by lexicographic accident. If rerun quality degrades,
   re-inspect.

Additional observations (not in original backlog triggers):

- **Content truncation is hitting 26 / 198 ≈ 13% of memories**. The
  4000-char cap is active signal. Consider raising to 6000-8000 for
  content-rich memories (plans, reviews, conclusions) in a follow-up —
  Claude Sonnet 4.6 handles 20K-token prompts comfortably even with
  20-cluster max size. `memories_truncated` counter earning its keep.
- **1 parse error on 14 LLM calls** ≈ 7% error rate. Cluster failed
  silently; re-running would pick it up via content_hash dedup on the
  other 13 (their content is identical) and re-try the failing cluster.
  Systematic structural regression unlikely at 1 / 14; monitor across
  runs.
- **Promotion pass promoted 0 memories** — unrelated to BL-007, but
  worth flagging: the RRF normalization ceiling issue from
  `docs/discussions/013-what-next-after-pause` may still be in play.
  Orthogonal to this backlog item.

**What to do next**: draft a follow-up plan to address residuals-rate
(trigger #3 above). Provisional scope: second-pass clustering over
residuals at looser threshold, or reduced min_size, or both. Document
as a BL-residuals-reduction entry first, then plan when validated
against a second dream run.
