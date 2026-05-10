---
id: BL-036
title: "vec0 search test — replace EQP-string smoke with behavioral KNN-correctness test"
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "(a) bundled SQLite version bump (per Cargo.lock diff on rusqlite or a `bundled` feature change), OR (b) sqlite-vec version bump (per Cargo.lock diff on sqlite-vec), OR (c) next touch of `Db::search_vector` SQL"
related: [F-006]
source: F-006 /ae:review (Codex P2 + architect Q2/C5 + performance Q1)
---

# BL-036: replace EQP-string smoke with behavioral KNN-correctness test

## What

`tests/vector.rs::test_vec_search_uses_vec0_match` (added in F-006 commit `01910cc`) asserts the SQLite `EXPLAIN QUERY PLAN` output for the per-project search query contains the substring `VIRTUAL TABLE INDEX`. Three F-006 reviewers independently flagged this as fragile:

- **Codex P2** — SQLite docs explicitly warn `EXPLAIN QUERY PLAN` output is debug-only and the format can change between minor releases.
- **architect Q2/C5** — the test body duplicates the live `search_vector` SQL inline. If the live SQL changes, the test still passes against its stale copy, giving a false green.
- **performance Q1** — `VIRTUAL TABLE INDEX` is necessary but NOT sufficient for KNN-via-vec0; vec0 fallback paths or future query-shape changes could fire the marker without using the actual KNN path.

Replace it with a behavioral test that exercises the KNN semantic directly — without depending on EQP text or duplicated SQL.

## Why deferred

The current EQP test passes today; it's not blocking. The fragility is latent: it only manifests when SQLite or sqlite-vec is bumped, OR when the search SQL is refactored. None of those are in the immediate v0.0.1 path. Filing as `defer-until-trigger` means the replacement happens at the time the fragility would actually fire.

## Trigger condition

Move this BL to a sprint when EITHER:

- `Cargo.lock` diff shows a rusqlite version bump (the bundled SQLite version moves with rusqlite), OR
- `Cargo.lock` diff shows a sqlite-vec version bump, OR
- Any future change touches `Db::search_vector` SQL (the test would silently keep passing on stale SQL).

## Hint at fix shape

Replace the EQP-text assertion with a behavioral test. Concrete shape:

```rust
#[test]
fn test_vec_search_returns_correct_nearest() {
    let db = test_db();
    // Insert exact-nearest target in project A.
    let target_emb = make_384d(&[1.0, 0.0, 0.0]);
    let (target, _) = mem_with_embedding("proj-a", "target", &target_emb);
    let target_id = db.insert_memory(target).unwrap();
    db.store_embedding(&target_id, &target_emb, 384).unwrap();

    // Insert far row in project A (orthogonal direction).
    let far_emb = make_384d(&[0.0, 1.0, 0.0]);
    let (far, _) = mem_with_embedding("proj-a", "far", &far_emb);
    let far_id = db.insert_memory(far).unwrap();
    db.store_embedding(&far_id, &far_emb, 384).unwrap();

    // Insert exact-match decoy in DIFFERENT project (proj-b) — must not be returned.
    let (decoy, _) = mem_with_embedding("proj-b", "decoy", &target_emb);
    let decoy_id = db.insert_memory(decoy).unwrap();
    db.store_embedding(&decoy_id, &target_emb, 384).unwrap();

    // Query in proj-a with the target embedding. Result must be target_id, not decoy.
    let results = db.search_vector(&target_emb, Some("proj-a"), 5).unwrap();
    assert_eq!(results.len(), 2, "should return both proj-a rows");
    assert_eq!(results[0].id, target_id, "exact match must rank first");
    assert!(
        !results.iter().any(|r| r.id == decoy_id),
        "proj-b decoy must not leak into proj-a results"
    );
    // Score sanity: exact match should be near 1.0 (cosine similarity 1.0 → distance 0 → score 1).
    assert!((results[0].score - 1.0).abs() < 1e-3);
}
```

This verifies three behaviors the EQP test only weakly implies:
1. KNN actually returns the nearest result (not a scan that happens to use vec0).
2. Per-project filter works (decoy in proj-b doesn't leak into proj-a results).
3. Score formula gives ~1.0 for exact match (cross-checks vec0 distance interpretation).

The existing `test_vector_search_returns_closest` covers behavior #1 partially; this new test adds behavior #2 (cross-project leakage) and #3 (score sanity) which are the actual KNN-correctness invariants.

After this test lands, delete `test_vec_search_uses_vec0_match` — the EQP smoke is redundant once the behavioral test covers the same ground.

## Out of scope

- Touching the live `search_vector` SQL itself.
- Changing the auxiliary-column-vs-IN-subquery pattern (separate concern; see performance reviewer Q2 for that scope).

## F-006 relationship

F-006 close-out treated the EQP test as "good enough" for v0.0.1 — the test does pass and does verify *something* meaningful. This BL captures the upgrade path for when fragility actually bites.
