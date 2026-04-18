---
id: "009"
title: "BL-006 — Embedding Clustering"
type: plan
created: 2026-04-18
status: reviewed
discussion: ""
---

# Feature: BL-006 — Embedding Clustering

## Goal

Add a pure-math `cluster_memories` function that groups memories by cosine similarity of their stored embeddings — the prerequisite for BL-007 dream synthesis (which will pass each cluster to the LLM for summary).

## Scope boundaries

- **In**: one new file `src/core/clustering.rs` exposing `cluster_memories(db, project_id, threshold, min_size) -> Vec<Cluster>`. Pure algorithm; reuses existing `cosine_similarity` + DB load pattern from `vector.rs`. Unit tests with synthetic embeddings.
- **Out**: LLM calls (that's BL-007), any schema change, any CLI wiring (that comes when BL-007 consumes it), storing cluster results anywhere (clusters are an in-memory transient), cross-project clustering (project-scoped only per plan 017 topic 02).
- **No callers yet**: just like plan 007 landed an unused `LlmProvider`, this plan lands `cluster_memories` that's not called by production code. BL-007 is the first caller.

## Algorithm

**Seed-neighborhood clustering**, greedy single pass on cosine similarity. This is NOT connected-component clustering — if A~B (similar) and B~C but A!~C, the seed-based pass does not chain them into one cluster. Intentional: for LLM summarization (BL-007), tighter topical groups produce better prompts than chain-linked sprawls.

```
inputs:  embeddings: Vec<(memory_id, Vec<f32>)>, threshold: f32, min_size: usize
outputs: ClusteringResult { clusters: Vec<Cluster>, residuals: Vec<String> }

1. Sort embeddings by memory_id ascending. Iteration order from the sorted slice is
   the ONLY source of determinism — the `assigned` set is for membership lookup only;
   its internal iteration order is never observed, so a HashSet is fine.
2. assigned: HashSet<memory_id> = {}
3. clusters: Vec<Vec<(memory_id, embedding)>> = []
4. for each (mid, emb) in embeddings (in sorted order):
     if mid in assigned: continue
     seed = (mid, emb)
     members = [seed]
     assigned.insert(mid)
     for each (other_mid, other_emb) in embeddings (in sorted order):
       if other_mid == mid: continue
       if other_mid in assigned: continue
       if cosine_similarity(seed.emb, other_emb) >= threshold:
         members.push((other_mid, other_emb))
         assigned.insert(other_mid)
     clusters.push(members)
5. Split clusters into two buckets:
   - kept (len >= min_size): build Cluster { memory_ids, centroid = mean(embeddings) }
   - dropped (len < min_size): collect all member memory_ids into residuals
6. return ClusteringResult { clusters: kept, residuals: dropped_member_ids }
```

Key properties:
- Each memory lands in AT MOST one group (assigned set).
- Seed = first memory (by sorted id) that hasn't been assigned yet.
- Member iteration within a cluster follows the input sort order → deterministic.
- `residuals` preserves memories that didn't reach min_size; BL-007 can choose to summarize pairs with a different prompt, skip them, or fold them into a "miscellaneous" group. This plan just exposes the data.

**Threshold justification**: 0.75 default. Sentence-Transformers' `community_detection` util uses 0.75 as its default for all-MiniLM-L6-v2, which is the model mengdie runs. At 0.75 the output is "tight topical community"; related-but-not-same sentences often score ~0.65-0.70 and are correctly excluded (see SBERT cross-encoder docs). Callers can pass a lower threshold for looser clustering. Citation in the module doc comment.

Complexity: O(N²) cosine comparisons + O(D·k) for centroid computation where D=384, k=cluster size. At N=1000 × 384 dim × f32, the cosine matrix is ~200ms (per memory: 1000 entries = ~0.2ms for one cosine; N² = ~200ms). Acceptable until we exceed ~10K memories, at which point we'd switch to sqlite-vec or ANN (separate future plan, not this one).

## Steps

### Step 1: `cluster_memories` + pure unit tests (AC1, AC2) — commit c2160c9

- [x] Add module: `pub mod clustering;` in `src/core/mod.rs` (keep alphabetical grouping consistent with existing layout)
- [x] Create `src/core/clustering.rs` with:
  - `pub struct Cluster { pub memory_ids: Vec<String>, pub centroid: Vec<f32> }` — `centroid` is the element-wise mean of member embeddings. BL-007 can use it to label the cluster in synthesis prompts or to rank clusters; cheap to compute (O(k·D) at cluster-build time) and avoids a second DB round-trip in BL-007.
  - `pub struct ClusteringResult { pub clusters: Vec<Cluster>, pub residuals: Vec<String> }` — `residuals` lists memory_ids that were evaluated but didn't land in a cluster ≥ min_size. BL-007 decides policy (summarize pairs differently, skip singletons, merge to "misc"). This plan does not drop residuals silently.
  - `pub fn cluster_memories(db: &Db, project_id: Option<&str>, threshold: f32, min_size: usize) -> anyhow::Result<ClusteringResult>` — loads embeddings from DB with the SAME filter as `search_vector`: `embedding IS NOT NULL` AND `embedding_dim = ?` AND `(valid_until IS NULL OR valid_until > now)` AND `project_id = ?`. The `embedding_dim` filter is required (copy from `vector.rs:56-72`) so mixed-dim DBs don't produce nonsense. Pass `384` as the constant today — if/when fastembed provides a const symbol, prefer it; otherwise inline the literal with a comment pointing at the fastembed init in `embeddings.rs::Embedder::new`.
  - Internal `fn cluster_embeddings(pairs: &[(String, Vec<f32>)], threshold: f32, min_size: usize) -> ClusteringResult` — pure, no DB access, the testable seam (matches the `drive_subprocess`/`classify_output` pattern from plan 007).
- [x] Default constants in the module: `pub const DEFAULT_THRESHOLD: f32 = 0.75; pub const DEFAULT_MIN_SIZE: usize = 3;` — justification in a doc comment: Sentence-Transformers' `community_detection` utility uses 0.75 as its default for all-MiniLM-L6-v2 (the model mengdie runs); tight topical community, excludes related-but-not-same scores (~0.65–0.70).
- [x] Use existing `cosine_similarity` from `crate::core::embeddings` — do not reimplement.
- [x] Use existing `blob_to_embedding` for reading embedding blobs from the DB.
- [x] Centroid computation: helper `fn centroid(embeddings: &[&[f32]]) -> Vec<f32>` that computes element-wise mean. Handle empty input defensively (shouldn't happen for kept clusters, but add a debug_assert).
- [x] Unit tests (all on `cluster_embeddings`, no DB):
  - Empty input → `ClusteringResult { clusters: [], residuals: [] }`
  - 3 near-identical embeddings, 2 orthogonal → 1 cluster of 3 + 2 residuals (the orthogonals)
  - 2 near-identical + 2 near-identical, orthogonal to each other, min_size=3 → 0 clusters + 4 residuals (neither pair reaches min_size; all 4 ids appear in `residuals`)
  - 5 near-identical embeddings, threshold 0.99 (strict), min_size=3 → 1 cluster of 5, no residuals
  - 5 near-identical embeddings, threshold 1.5 (unreachable), min_size=2 → 0 clusters + 5 residuals
  - Determinism: run same input twice, assert identical `ClusteringResult` (same clusters with same seed/member order, same residuals order). Explicitly calling this out because the algorithm's determinism is a DESIGN property (derives from sorted-slice iteration), not a happy accident — a future contributor must not introduce a HashSet-iteration-dependent ordering.
  - Seed selection: with sorted input, first cluster's seed is lowest memory_id. Input `[("c", v), ("b", v), ("a", v)]` all similar → one cluster with `memory_ids = ["a", "b", "c"]`.
  - Centroid math: 3 embeddings `[1,0,0], [1,0,0], [1,0,0]` → cluster centroid `[1,0,0]` (exact). 3 embeddings `[1,0,0], [0,1,0], [0,0,1]` — not a real cluster test, skipped for the 3-orthogonals case since they wouldn't cluster — but assert centroid helper directly: mean of `[[2.0, 0.0], [0.0, 2.0]]` = `[1.0, 1.0]`.
- [x] Verify: `cargo test --lib clustering::` passes (expected ≥7 tests). `cargo clippy --all-targets -- -D warnings` clean. `cargo fmt --all -- --check` clean.

Expected files: `src/core/mod.rs`, `src/core/clustering.rs`

### Step 2: DB-loading integration test (AC3)

- [x] Add a `#[test]` in `src/core/clustering.rs` (or `tests/clustering_db.rs` if it needs more setup) that:
  - Opens `Db::open_in_memory()`
  - Inserts 5 memories with controlled embeddings via `insert_memory` + `store_embedding`: 3 near-identical at `[1.0, 0.0, 0.0]` + tiny noise, 2 near-identical at `[0.0, 1.0, 0.0]` + tiny noise (2 distinct clusters)
  - Calls `cluster_memories(&db, Some("proj"), 0.9, 2)` — threshold 0.9 to catch the cluster even with noise, min_size 2 to catch both
  - Asserts: 2 clusters, one of size 3 and one of size 2, memory_ids assigned to the correct cluster based on embedding value
  - Second test: same setup but `min_size = 3` → 1 cluster (just the triplet)
  - Third test: project filter — insert two memories in project "other" with the same embedding as cluster A. Call `cluster_memories(&db, Some("proj"), 0.9, 2)` and verify the "other"-project memories are NOT included.
  - Fourth test: invalidated memory — invalidate one of the cluster-A memories via `db.invalidate_memory`. Call clustering. Assert it's excluded.

Expected files: `src/core/clustering.rs`

## Acceptance Criteria

### AC1: Algorithm correctness on pure inputs
All ≥8 pure-logic unit tests in Step 1 pass. Specifically:
- Empty input returns `ClusteringResult { clusters: [], residuals: [] }`.
- For the "3 similar + 2 orthogonal, threshold 0.75, min_size 3" case: result is exactly 1 `Cluster` with 3 members AND 2 entries in `residuals` (the 2 orthogonal memory_ids). No memory is silently dropped.
- Determinism: two back-to-back calls with the same `Vec<(id, embedding)>` input produce equal `ClusteringResult` (same cluster count, same ids per cluster, same member order, same residuals order).
- Seed selection: with sorted input, cluster seed = lowest memory_id. `cluster_embeddings([("c", v), ("b", v), ("a", v)], 0.9, 3)` produces `ClusteringResult { clusters: [Cluster { memory_ids: ["a", "b", "c"], centroid: ≈v }], residuals: [] }`.
- Centroid of 3 identical embeddings `[1,0,0]` is `[1,0,0]` (±1e-6).

### AC2: Complexity stays reasonable
- `cargo test --lib clustering::` completes in <1 second total for the full clustering test suite (including the 5-element integration test).
- A deliberate N=200 synthetic benchmark (100 near-identical + 100 random) in a `#[test]` with `std::time::Instant` completes in <500ms on this machine. Not a CI-enforced threshold — a local dev sanity check that O(N²) cosine is not pathologically slow. Record the measured number in the commit message.
- `cargo clippy --all-targets -- -D warnings` exits 0 — zero new `#[allow]` attributes added.

### AC3: DB-backed integration verifies scope + expiry filters work
- The 4 DB-integration tests in Step 2 all pass.
- `cluster_memories` correctly:
  - Respects `project_id = Some("proj")` — excludes memories from other projects.
  - Excludes memories with `valid_until` set (invalidated memories).
  - Excludes memories with `embedding IS NULL`.
  - Returns `Ok(Vec::new())` when no memories meet the criteria (not Err, not panic).

## Non-goals (explicit)

- No LLM calls. BL-007 will add `mengdie dream` as the first caller that takes these clusters and feeds them to a prompt.
- No schema change. Clusters are in-memory transients; if we ever store cluster assignments, that's a separate plan.
- No CLI wiring. `mengdie dream` command integration is BL-007's job.
- No cross-project clustering. Discussion 017 topic 02 confirmed project-scoped search/cluster is the default for solo-dev; cross-project is BL-010.
- No approximate nearest-neighbor (ANN) index. O(N²) is fine at current scale per memory `search_vector O(N) cosine loop is acceptable up to ~10K memories`. When we cross that threshold, sqlite-vec is the migration path — a separate plan.
- No tunable clustering algorithm. Greedy single-pass with a threshold is the spec. DBSCAN / HDBSCAN / hierarchical are NOT in scope — if the greedy results are poor in practice, we revisit in a new discussion.
- No density-based seed ordering. Codex review flagged that "best seed first" (sort by neighbor count above threshold) would produce more stable clusters when input order is adversarial. For MVP with sorted-by-id seeds, this is adequate; add if empirical clustering quality is poor once BL-007 lands.
- **Singleton/pair policy is exposed, not decided**: `ClusteringResult.residuals` lists everything that didn't cluster. BL-007 will decide whether to skip those, summarize pairs with a different prompt, or group them as "miscellaneous". This plan does not silently drop them.
