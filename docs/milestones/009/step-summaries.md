# Plan 009 (BL-006 Embedding Clustering) — Step Summaries

## Step 1 — cluster_memories + pure unit tests (commit: c2160c9)
**Decisions**:
- Put Step 1 + Step 2 code into one file (`src/core/clustering.rs`) but split commits by test class: Step 1 = pure-logic tests only, Step 2 = DB-backed tests. Plan offered `tests/clustering_db.rs` as alternative; co-locating keeps the caller/test coupling visible.
- Explicit mixed-dimension guard in `cluster_embeddings` (skip, don't panic) + `debug_assert` on `centroid` — addresses Codex + Doodlestein finding that the pure contract was loose. DB path still enforces `embedding_dim = 384`; the guard covers callers bypassing the DB seam.
- Relaxed N=200 benchmark ceiling from 500ms (plan AC2 dev-hardware target) to 5000ms; emit measured ms via `eprintln!`. Codex P3: the 500ms assert is flaky under debug/CI/sanitizer load while AC2 explicitly says "not CI-enforced". Measurement still recorded, threshold just can't false-fail.

**Rejected**:
- Extracting clustering into its own crate (not necessary at this scale; matches existing `vector.rs`/`search.rs` convention of in-tree modules).
- Using a `HashMap<String, Vec<f32>>` for fast neighbor lookup (the plan's sort-by-id determinism requirement would be harder to guarantee; iteration order of HashMap is non-deterministic).

**Cross-step deps**:
- `src/core/clustering.rs`: public surface `cluster_memories`, `cluster_embeddings`, `Cluster`, `ClusteringResult`, `DEFAULT_THRESHOLD`, `DEFAULT_MIN_SIZE` — BL-007 dream synthesis will consume `ClusteringResult.clusters` and decide policy on `.residuals`.
- SQL filter in `load_embeddings` mirrors `vector.rs::search_vector` exactly — if vector filter ever changes (e.g. new soft-delete column), clustering must track it.

**Actual files**: `src/core/mod.rs`, `src/core/clustering.rs`

**Benchmark (AC2)**: N=200 synthetic completes in ~0ms (debug build, M-series dev hardware). Well under the 500ms plan target.
