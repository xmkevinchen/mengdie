//! Integration tests for the fastembed Embedder wrapper.
//!
//! F-006 Step 6 / C3 — fastembed unit-vector verification (Option B per
//! the F-006 plan recommendation, which was revised from the original
//! `debug_assert!` Option A by Codex cross-family plan-review feedback:
//! `debug_assert!` doesn't fire in release builds, so it's not a runtime
//! verification of the assumption baked into `vector.rs:137`'s
//! `score = 1.0 - distance / 2.0` formula).
//!
//! This test runs the actual fastembed model and asserts the output is
//! unit-normalized within `1.0 ± 1e-3`. If fastembed ever changes its
//! output normalization (or we accidentally swap to a non-normalizing
//! model), this test fails — flagging the assumption breakage at CI
//! time, not during a silent search-quality regression in production.
//!
//! Marked `#[ignore]` because the fastembed model downloads ~90MB on
//! first run, which is too heavy for default `cargo test` runs. Run
//! explicitly with `cargo test --test embeddings -- --ignored` when
//! verifying the assumption (e.g., before a release tag).

use mengdie::core::embeddings::Embedder;

/// AC2 — fastembed Embedder produces unit-normalized vectors (L2 norm ≈ 1.0
/// within 1e-3). The vector.rs distance-to-similarity formula
/// (`similarity = 1 - distance / 2`) assumes this; if the assumption
/// breaks, search rankings stay consistent (rank-based RRF in search.rs)
/// but absolute scores leave the [0, 1] band the rest of the code
/// expects.
#[test]
#[ignore = "Downloads ~90MB fastembed model on first run; opt-in via --ignored"]
fn test_fastembed_returns_unit_vector() {
    let mut embedder = Embedder::new().expect("Embedder::new should succeed (downloads model)");

    let embedding = embedder
        .embed_text("test query for unit-norm verification")
        .expect("embed_text should succeed");

    assert_eq!(
        embedding.len(),
        embedder.dimension(),
        "embedding length must match Embedder::dimension()"
    );
    assert_eq!(embedding.len(), 384, "fastembed AllMiniLML6V2 is 384-d");

    let norm_sq: f32 = embedding.iter().map(|x| x * x).sum();
    let norm = norm_sq.sqrt();

    assert!(
        (norm - 1.0).abs() < 1e-3,
        "fastembed output must be unit-normalized (norm ≈ 1.0); got norm = {norm} \
         (norm² = {norm_sq}). The vector.rs:137 similarity formula \
         `1.0 - distance / 2.0` assumes this; if this test fails, either \
         re-normalize at the embedder boundary or rewrite the similarity \
         formula to match the actual range fastembed produces."
    );
}
