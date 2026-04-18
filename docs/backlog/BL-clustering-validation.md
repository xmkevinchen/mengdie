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
