---
id: "sqlite-vec-compat"
type: spike
status: accepted
date: 2026-04-28
spike_for: F-001  # / BL-007
plan: docs/plans/018-sqlite-vec-compat-spike.md
outcome: PASS_WITH_CONDITIONS
decision_drivers:
  - "BL-012 vector.rs adoption needs metric identity for score normalization design"
  - "BL-002 Reflection consolidation defers on this outcome"
  - "v0.0.1 sprint Wave 1 BL-007 verification gate"
environment:
  sqlite_vec_version: "=0.1.9"
  rusqlite_version: "0.39"
  sqlite_version: "3.51.3"  # via libsqlite3-sys 0.37 bundled
  rust_toolchain: "rustc 1.95.0 (59807616e 2026-04-14) (Homebrew)"
  target_triple: "aarch64-apple-darwin"
  os: "macOS (operator's primary dev machine)"
  invocation: "cargo run --example sqlite_vec_smoke"
caveats:
  - description: "vec0 default distance metric is L2, not cosine. Current code is unaffected — `src/core/search.rs` uses pure rank-based RRF (`1.0 / (k + rank + 1.0)`); no raw-distance score conversion exists in `src/` today. The metric matters at BL-012 adoption time: when sqlite-vec replaces brute-force `search_vector`, any future score-conversion layer assuming cosine semantics (range [0, 2]) would silently corrupt scores under L2 (range [0, ~28] on 384-d unit vectors). Mitigation: BL-012 MUST declare the virtual table with the column-declaration override `vec0(embedding float[384] distance_metric=cosine)` (verified working at runtime in =0.1.9 — see Evidence section). Failure to use the override is a correctness bug, not a style choice."
    severity: ACCEPTABLE
    trigger_fires: true
---

# F-001 / BL-007 — sqlite-vec compatibility verification spike

## Question

Does `sqlite-vec = "=0.1.9"` load cleanly into mengdie's `rusqlite { features = ["bundled", "load_extension"] }` runtime, and what default distance metric does the `vec0` virtual table's `MATCH` operator use on `float[N]` columns?

## Context

mengdie's v0.0.1 sprint Wave 1 (per `.ae/roadmaps/v0.0.1.md`) includes this verification spike as a gate for two downstream commitments:

1. **BL-002 Reflection consolidation** (`docs/backlog/unscheduled/BL-002-...`) — deferred from v0.0.1 with explicit trigger "after sqlite-vec spike outcome is recorded." This spike's recorded outcome IS that trigger.
2. **BL-012 vector.rs adoption** (to be filed if outcome ≠ FAIL) — must know the default metric to design score normalization correctly. `src/core/search.rs` currently uses rank-based RRF (per dependency-analyst review of plan 018), so adoption-time score conversion matters, not current-code corruption.

Per F-001 analyze phase findings:

- `Cargo.toml`: `rusqlite = "0.39"` already had `features = ["bundled", "load_extension"]` enabled but never called from `src/` — this spike is the first call site
- `src/core/embeddings.rs:96-98`: vector blob format `f.to_le_bytes()` matches sqlite-vec's MATCH bind-parameter byte format (zero conversion needed)
- SQLite bundled version 3.51.3 is well above sqlite-vec's 3.44.0 minimum requirement
- fastembed all-MiniLM-L6-v2 normalizes outputs to unit vectors (relevant because cosine and L2 ranking are equivalent for unit vectors, but raw distance values differ)

External evidence (from codex-proxy plan-review) suggested `vec0` defaults to L2 with `distance_metric=cosine` available as column override. This spike confirms or refutes that against the specific `=0.1.9` pin.

## Method

Plan: `docs/plans/018-sqlite-vec-compat-spike.md`. Executed in 5 steps on `spike/018-sqlite-vec` branch (squash-merged to base after this commit).

**Smoke binary**: `examples/sqlite_vec_smoke.rs` (throwaway; deleted in Step 4 before this record was committed). Source is **not preserved** — neither in this record's appendix nor in plan 018's body (only the step-level checklist + SQL snippets are recorded). BL-011 (Linux x86_64 verification) author reconstructs from plan 018 Steps 1–3 specifications + the SQL queries cited verbatim in this record's Evidence section. This is an accepted durability tradeoff for a single-platform smoke spike; if a stronger audit trail is needed at BL-011 time, the BL-011 plan should add a preserved `examples/` source under its own scope.

**Discriminating probe pair** (constructed at Step 2):

- `A = [1.0, 0.0, ..., 0.0]` (384-d, position 0 = 1.0)
- `B = [0.5, sqrt(0.75), 0.0, ..., 0.0]` (384-d, sqrt(0.75) ≈ 0.866025)
- Pre-INSERT verification (in test code): `dot(A, B) = 0.5` (within 1e-6), `||A|| = ||B|| = 1.0` (within 1e-6)

For unit vectors with `dot = 0.5`:
- cosine_distance = `1 - dot = 0.5`
- L2_distance = `sqrt(2 - 2·dot) = sqrt(1.0) = 1.0`

These two reference values are 0.5 apart — far outside the 1e-4 tolerance — so the returned `distance` from sqlite-vec's MATCH operator unambiguously identifies one metric or the other (or, in the indeterminate FAIL case, neither).

## Environment

```yaml
sqlite_vec_version: =0.1.9
rusqlite_version:   0.39
sqlite_version:     3.51.3 (via libsqlite3-sys 0.37 bundled)
rust_toolchain:     rustc 1.95.0 (59807616e 2026-04-14) (Homebrew)
target_triple:      aarch64-apple-darwin
os:                 macOS (operator's machine)
invocation:         cargo run --example sqlite_vec_smoke
```

Distribution model confirmed: sqlite-vec compiles its vendored `sqlite-vec.c` (337KB) statically via `cc` build script. **No runtime `.dylib`/`.so` dependency. No operator install step beyond `cargo build`.** Final `target/release/mengdie` and `target/release/mengdie-mcp` would NOT carry sqlite-vec after this spike's revert (verified — Cargo.lock has zero `sqlite-vec` entries post-Step 4).

## Evidence

Captured directly from the smoke binary's stdout (formatted):

```
[Step 1] vec_version() = v0.1.9
[Step 1] OK — extension registered + vec_version() responsive

[Step 2] CREATE VIRTUAL TABLE vec_test USING vec0(embedding float[384]) — OK
[Step 2] Probe pair: dot(A,B) = 0.500000 (expect 0.5),
         ||A|| = 1.000000 (expect 1.0), ||B|| = 1.000000 (expect 1.0)
[Step 2] Inserted 5 vectors (rowid 1..=5)
[Step 2] KNN with probe=A, LIMIT 3 → [(1, 0.0), (2, 1.0), (5, 1.4142135)]
[Step 2] Self-match (probe A vs itself): rowid=1, distance=0.000000

[Step 3] Returned distance(A,B) = 1.000000;
         cosine reference = 0.500000;
         L2 reference     = 1.000000
[Step 3] Identified metric: L2 (Case 2) → outcome candidate PASS_WITH_CONDITIONS

[Step 3 OVERRIDE TEST]
[Step 3] CREATE VIRTUAL TABLE ... distance_metric=cosine — accepted
[Step 3] Override path: distance(A,B) = 0.500000 (expect cosine = 0.500000)
[Step 3] Override WORKS — caveat ACCEPTABLE, trigger_fires=true
```

Key facts:

- Default metric on `vec0(embedding float[384])` is **L2**:
  - returned `distance(A, B) = 1.000000`
  - L2 reference for `dot=0.5` unit vectors = `sqrt(2 - 2·0.5) = sqrt(1.0) = 1.000000` ✓
  - cosine reference = `1 - 0.5 = 0.500000` ✗
  - `|returned − L2| = 0.0 < 1e-4` → match
  - `|returned − cosine| = 0.5 > 1e-4` → no match
- Self-match works: probing A against the vec_test table including A returns `(rowid=1, distance=0.0)` as the top result
- Orthogonal vectors (A vs E at position 3) return `distance = sqrt(2) ≈ 1.4142135` — consistent with L2 for two orthogonal unit vectors
- Override path works:
  - `CREATE VIRTUAL TABLE vec_test_cos USING vec0(embedding float[384] distance_metric=cosine)` — column declaration accepted by sqlite-vec parser
  - Inserting A, B and probing A on this table returns `distance(A, B) = 0.500000` (matches cosine reference)
  - Override is functional at runtime in `=0.1.9`

## Distance metric finding

**`vec0` MATCH default metric = L2** (case 2 of plan 018 AC3).

The default can be overridden at column declaration time:

```sql
CREATE VIRTUAL TABLE my_vecs USING vec0(embedding float[N] distance_metric=cosine)
```

This override is documented in sqlite-vec's reference and **verified to work in `=0.1.9`** by this spike. After override, `MATCH ... ORDER BY distance` returns cosine distance (range [0, 2] for unit vectors) instead of L2 distance.

**Note on L2 vs L2² disambiguation**: for the primary probe pair (A, B) with `dot=0.5`, both L2 and L2² return `1.0` — that single observation alone cannot distinguish them. The disambiguation comes from the orthogonal-pair evidence (Evidence section row 5: `distance ≈ 1.4142135` for the `dot=0` pair). For unit vectors with `dot=0`, L2 = `sqrt(2) ≈ 1.4142` while L2² = `2.0` — the observed value matches L2, not L2². Combined with the (A, B) measurement, the default metric is unambiguously L2 (not L2², not dot-product-distance for non-unit input). Caller-side data-path note: this analysis assumes fastembed-normalized unit vectors (current production path); for non-unit inputs the discrimination would need an additional probe.

## RRF compatibility analysis

**Important framing correction (per F-001 analyze + plan 018 review):**

mengdie's current `src/core/search.rs` uses **rank-based RRF** (`1.0 / (k + rank + 1.0)`), not raw-distance-based RRF. The current code consumes `VectorResult { score: f32 }` (similarity) but the merge ranks results, not their distance values. Therefore this spike's outcome **does not affect current code** (AC6 enforced no `src/` modification regardless).

The metric matters at **BL-012 adoption time**, when sqlite-vec replaces the brute-force `search_vector` path. BL-012 must:

1. Declare the `vec_memories` virtual table with explicit `distance_metric=cosine` override (recommended; simpler), OR
2. Refactor any score-conversion logic that assumes cosine distance to handle L2 distance (e.g., `score = 1.0 - distance/2.0` is wrong for L2; a correct L2-aware conversion would be `score = 1.0 / (1.0 + distance)` or similar)

**Recommended approach for BL-012**: option 1. The `distance_metric=cosine` override at table-creation time is documented, verified, and self-explanatory. mengdie's existing code patterns already favor cosine semantics (per `src/core/embeddings.rs::cosine_similarity`).

## Recommendation

**Adopt sqlite-vec at v0.0.1 Tier 1 (eventually) — proceed with BL-012 adoption planning.**

The compatibility verification PASSES with one ACCEPTABLE caveat: **BL-012 adoption MUST use the `distance_metric=cosine` column-declaration override** when creating the production `vec_memories` virtual table. If BL-012 fails to do so, the produced distance values (L2 [0, ~28]) will not match mengdie's existing cosine-similarity semantics, but rank-only consumers (like current RRF) would still produce correct ordering.

**Triggers fired by this spike:**

- **BL-002 Reflection consolidation** (`docs/backlog/unscheduled/BL-002-...`) — trigger condition met (sqlite-vec spike outcome recorded as PASS_WITH_CONDITIONS with `trigger_fires: true`). Operator may now schedule BL-002's plan to consolidate `clustering.rs` / `synthesis.rs` / `dreaming.rs`.

**Follow-up BLs to file (per plan 018 § Decisions not implemented):**

- **BL-011: Linux x86_64 CI verification of sqlite-vec spike** — same smoke binary, run on Forgejo runner (`runs-on: ubuntu-latest`). MUST be filed before BL-012 closes (not before BL-012 starts). Linux x86_64 PASS gate is required for adoption to reach DONE state.
- **BL-012: vector.rs sqlite-vec adoption** — replace brute-force `search_vector` impl with `vec0` virtual table query. Schema migration v5 → v6 adds `vec_memories` with `distance_metric=cosine` override. Bones-pattern adapter module (codex-proxy recommended) isolates the unsafe registration. Real integration tests in `tests/`.

**No follow-up needed for FAIL scenarios** — the spike outcome is PASS_WITH_CONDITIONS, not FAIL. The override path mitigates the L2 default cleanly.

## Consequences

**Enabling**:
- BL-002 (Reflection consolidation) trigger condition met — operator can now schedule its plan when ready (operator-scheduled, not auto-scheduled).
- BL-012 (vector.rs adoption) is now a viable v0.0.1+ work item; bones-pattern adapter design can begin.
- mengdie's storage Tier 1 (per blueprint §7) gains a concrete adoption candidate beyond hand-rolled brute-force.

**Risk-acceptance**:
- Pre-v1 sqlite-vec dependency: `=0.1.9` exact pin is locked in BL-012's adoption design. Future upgrades require deliberate spikes (matrix smoke + perf bench + issue audit) per gemini-proxy / codex-proxy pre-v1 pinning recommendation in F-001 analysis.
- L2-as-default is a documented quirk in `=0.1.9`; if sqlite-vec changes its default in v1.0+ the override-at-column-declaration path may behave differently. BL-012's plan should pin the version exactly and document this.

**Neutral / Captured baseline**:
- macOS arm64 verified (operator's primary dev platform).
- Linux x86_64 explicitly NOT verified — BL-011 captures this gap.
- Static-link via `cc` build script is the actual distribution model (no `.dylib`); single-binary deployment intent (per blueprint §7 Tier 1) is preserved by sqlite-vec adoption.
- vector blob byte format (`f.to_le_bytes()` LE-packed f32 from `embeddings.rs`) confirmed compatible with sqlite-vec MATCH bind input — zero serialization changes needed in BL-012.

## Spike artifact disposal

- `examples/sqlite_vec_smoke.rs` — deleted in plan 018 Step 4
- `Cargo.toml` `[dev-dependencies] sqlite-vec` line — reverted in plan 018 Step 4
- `Cargo.lock` regenerated — verified `grep -c 'name = "sqlite-vec"' Cargo.lock` returns 0
- This file (`docs/spikes/sqlite-vec-compat.md`) is the only artifact crossing the spike branch's merge boundary.
