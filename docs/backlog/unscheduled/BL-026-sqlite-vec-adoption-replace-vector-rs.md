---
id: BL-026
title: "sqlite-vec adoption — replace vector.rs (264 LoC) with sqlite-vec virtual table after static-vs-dynamic-link spike PASS"
status: open
created: 2026-05-06
origin: "discussion 026 OSS-survey analysis (qualified ADOPT verdict) + 027 conclusion 2026-05-06 caveat (narrow OSS-replacement scope) + /ae:code-review Track 4 strategic finding (sprint candidate needs filed home)"
trigger: "v0.0.1 Phase 1 work — fires immediately upon sprint commitment; the static-vs-dynamic-link spike is the gate inside this BL itself, not a separate BL"
depends_on: []
size: S
v_target: "v0.0.1 — Phase 1 mengdie-side OSS swap (paired with BL-027)"
---

# BL-026 — sqlite-vec adoption (replace vector.rs)

## Origin

Discussion 026 OSS-survey analysis library scorecard (`docs/discussions/026-rust-oss-survey/analysis.md` L36 + L156) verdict:

> **sqlite-vec — Qualified ADOPT** — pending static-vs-dynamic distribution check.

Per 026 analysis L54-64 (sqlite-vec deep dive):

- SQLite extension (pure C, zero deps) providing `vec0` virtual tables for KNN over float32 / int8 / binary vectors
- Loads via `sqlite3_auto_extension` into existing rusqlite connection
- Distance functions: `vec_distance_L2`, `vec_distance_cosine`, `vec_distance_hamming`
- Replaces `vector.rs` (264 LoC) entirely — current full-table-scan + Rust-side cosine loop becomes one indexed SQL query
- ~50 lines net change in mengdie src/

The "qualified" caveat is challenger pushback at 026 analysis time:
> Does sqlite-vec ship as a Rust crate that statically links the C via a `cc` build script, OR as a shared library (`.dylib`/`.so`) that operators must install separately? The two have very different operator stories. Same-binary-no-runtime-extension is LOW; "operator installs `.dylib`" is real friction.

## Acceptance criteria (sequenced)

### Step 1 — Compatibility spike (15 min, embedded gate)

- [ ] Fresh workspace: `cargo new --lib bl026-spike && cd bl026-spike`
- [ ] `cargo add sqlite-vec`
- [ ] `cargo add rusqlite --features bundled,load_extension` (matching mengdie's `Cargo.toml`)
- [ ] Write a 30-line proof: open a SQLite connection, register sqlite-vec via `sqlite3_auto_extension`, create `vec_memories USING vec0(embedding float[384])`, insert 10 random vectors, query KNN top-3
- [ ] `cargo build --release` and confirm: does the resulting binary contain sqlite-vec statically (file size, `nm` symbol scan), OR does it require a runtime `.dylib`?
- [ ] Document outcome at `docs/spikes/sqlite-vec-distribution.md` (PASS_STATIC / PASS_DYNAMIC / FAIL)

### Step 2 — Adoption (PASS_STATIC only; ~50 LoC src change)

- [ ] Add `sqlite-vec` to mengdie root `Cargo.toml`
- [ ] Register extension in `src/core/db.rs::open` via `sqlite3_auto_extension`
- [ ] Add v7 schema migration: `CREATE VIRTUAL TABLE vec_memories USING vec0(embedding float[384])` + populate from existing `memory_entries.embedding` BLOB rows
- [ ] Replace `src/core/vector.rs::search_vector` body with `SELECT memory_id, distance FROM vec_memories WHERE embedding MATCH ? ORDER BY distance LIMIT ?`
- [ ] Convert `vec_distance_cosine` distance (0 = identical, 2 = opposite) to similarity for RRF merge: `score = 1.0 - distance / 2.0`
- [ ] Verify against fastembed-rs all-MiniLM-L6-v2 normalization (it normalizes to unit vectors by default; cosine distance maps cleanly)
- [ ] Existing search.rs RRF merge unchanged (consumes similarity scores)
- [ ] Run `cargo test` — all existing vector-search tests pass with new backend

### Step 2 alternative — Adoption (PASS_DYNAMIC; defer)

If the spike returns PASS_DYNAMIC (sqlite-vec works but requires operator-installed `.dylib`), file the operator-friction concern as a separate decision: keep current `vector.rs` and reopen when LanceDB trigger fires (corpus >100k OR p95 vector latency >50ms). Document the deferral rationale in `docs/spikes/sqlite-vec-distribution.md` outcome record.

### Step 2 alternative — Adoption (FAIL)

If sqlite-vec does not load against bundled rusqlite (FAIL), keep current `vector.rs`; document failure mode + revisit only if LanceDB DEFER trigger fires.

## Trigger

Fires immediately upon v0.0.1 sprint commitment (Phase 1 mengdie-side OSS swap). Co-commit candidate with BL-027 only if both Step 1 spikes PASS in the same sprint window.

## Reversibility

**HIGH**. If sqlite-vec adoption ships and later proves problematic (e.g., upstream API churn, runtime `.dylib` distribution discovered late), the existing `vector.rs` full-table-scan logic is still in git history; reverting requires removing the v7 migration + restoring the old vector.rs body. No data migration — `memory_entries.embedding` BLOB column is the source of truth in both designs; the `vec_memories` virtual table is a derived index.