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

## Step 2 — DB-backed integration tests (commit: c60d129)
**Decisions**:
- Bundled plan-checkbox and step-summary writeback into the Step 2 commit rather than a separate meta commit — fewer commits, same information density, and the drift was pre-approved (plan + milestone meta files).
- Tightened `test_db_project_filter_excludes_other_projects` to assert "other"-project ids are absent from BOTH clusters AND residuals, not just clusters (Codex P2). A project-filter bug that leaked rows into residuals would otherwise pass silently.
- Deferred the `valid_until > ?1` vs `>=` concurrency concern (Doodlestein) to `docs/backlog/BL-valid-until-boundary.md` — same semantics as existing `search_vector`, not BL-006 scope.
- Skipped Codex's P3 test-tightening suggestion for `test_db_two_clusters_with_noise` (exact set-match on clusters+residuals). Current assertions are strong enough; further tightening is diminishing returns.

**Rejected**:
- Moving tests to `tests/clustering_db.rs` (external integration dir). Co-located `mod tests` block keeps implementation + tests visible to the same reviewer/reader.
- Adding a `content_hash` dedup collision test — out of scope; that's insert_memory's responsibility, not cluster_memories'.

**Cross-step deps**: none — BL-006 complete. BL-007 (dream synthesis) is the next consumer and will call `cluster_memories` from `mengdie dream`.

**Actual files**: `src/core/clustering.rs`, `docs/plans/009-embedding-clustering.md`, `docs/milestones/009/step-summaries.md`, `docs/backlog/BL-valid-until-boundary.md`
