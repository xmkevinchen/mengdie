---
id: "018"
title: "sqlite-vec compatibility verification spike (F-001 / BL-007)"
type: plan
created: 2026-04-28
reviewed: 2026-04-28
status: reviewed
discussion: "docs/discussions/028-v0.0.1-architecture-design/"
feature: ".ae/features/active/F-001-sqlite-vec-compatibility-verification-sp/"
backlog: BL-007
sprint: v0.0.1
wave: 1
size: S
size_caveat: "S+ contingency: AC3 indeterminate-result branch (Step 3) may add 2-4h investigation if returned distance matches neither cosine nor L2 within 1e-4."
spike_branch: "spike/018-sqlite-vec"
prerequisites:
  - "Rust toolchain ≥ 1.80 stable"
  - "Xcode Command Line Tools installed (macOS arm64; cc/clang for sqlite-vec C compilation)"
  - "Internet access to crates.io (sqlite-vec dep download)"
review_team: "F-001-plan-review (architect, dependency-analyst, codex-proxy, gemini-proxy); 7 Must Fix items addressed inline; review summary in plan body"
---

# Feature: sqlite-vec compatibility verification spike (F-001 / BL-007)

## Goal

Verify (a) `sqlite-vec` extension loads cleanly into mengdie's
existing `rusqlite { features = ["bundled", "load_extension"] }`
runtime AND (b) the `vec0` MATCH operator's default distance metric
identity (cosine vs L2 vs neither), producing a structured outcome
record at `docs/spikes/sqlite-vec-compat.md` so the future
`vector.rs` adoption BL (BL-012, post-spike) can correctly design
its score normalization and the deferred backlog item BL-002
(Reflection consolidation) can fire / gate on the spike's outcome.

**Important framing correction (per dependency-analyst review):
mengdie's current RRF in `src/core/search.rs` is rank-based**
(`1/(k+rank+1)`), not distance-based. The spike does NOT touch
production code (AC6 enforces this) and therefore CANNOT corrupt
the current RRF regardless of metric outcome. The metric identity
matters for **BL-012 adoption design** — when sqlite-vec actually
replaces the brute-force vector path, BL-012 must convert raw
`distance` to similarity correctly given the identified metric.
This spike provides BL-012 the input it needs.

## Prior Art from Project Knowledge Base

Prior context: unavailable (memory_search MCP tool not registered in this session).

Project-internal context (loaded as primary input alongside 028
conclusion + F-001 analysis):

- **`docs/discussions/028-v0.0.1-architecture-design/conclusion.md`** — architectural decisions feeding the spike
- **`docs/blueprint.md`** §10 verification spikes + §12 v0.0.1 decisions
- **`.ae/features/active/F-001-.../analysis.md`** — full 5-agent codebase research (rusqlite 0.39 features, vector blob byte-format match, SQLite 3.51.3 vs 3.44.0 minimum, fastembed unit normalization, registration timing, CI/dev platform mismatch, three production Rust+sqlite-vec users cited)
- **`docs/discussions/026-rust-oss-survey/analysis.md`** — sqlite-vec scorecard

External reference (from codex-proxy plan-review): current
sqlite-vec docs state `vec0` defaults to L2 with
`distance_metric=cosine` available as column-declaration override.
Spike will confirm or refute against `=0.1.9` specifically.

## Steps

### Step 1: Add dep + register extension + build smoke (AC1)
- [ ] Branch off current branch into `spike/018-sqlite-vec` (per dep-analyst review — operator runs all spike commits on this branch; merge semantics defined in Step 5)
- [ ] Add `sqlite-vec = "=0.1.9"` (exact pin) to `Cargo.toml` `[dev-dependencies]`
- [ ] Verify `rusqlite = "0.39"` already has `features = ["bundled", "load_extension"]` (per F-001 analysis)
- [ ] Create `examples/sqlite_vec_smoke.rs` (per codex-proxy review — Cargo target compatibility; `spike/` is NOT Cargo-discovered)
- [ ] In `examples/sqlite_vec_smoke.rs` `main()`: call `unsafe { rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ()))) }` BEFORE first `Connection::open_in_memory()`
- [ ] Open in-memory connection; run `SELECT vec_version()`; capture returned version string
- [ ] Confirm `cargo run --example sqlite_vec_smoke` builds + runs to completion without panic on macOS arm64

Expected files: `Cargo.toml` (transient — reverted in Step 4), `Cargo.lock` (transient), `examples/sqlite_vec_smoke.rs` (transient — deleted in Step 4)

### Step 2: vec0 + KNN + self-match smoke (AC2)
- [ ] In the smoke binary: `CREATE VIRTUAL TABLE vec_test USING vec0(embedding float[384])`
- [ ] Construct **explicit non-orthogonal unit-vector pair** (per gemini-proxy review — one-hot can't produce dot=0.5):
  - `A = [1.0, 0.0, 0.0, ..., 0.0]` (384-d, position 0 = 1.0)
  - `B = [0.5, sqrt(0.75), 0.0, ..., 0.0]` (384-d, sqrt(0.75) ≈ 0.8660254)
  - Verify (in test code, before INSERT): `dot(A, B) = 0.5` and `||A|| = ||B|| = 1.0` (within 1e-6 of 1.0 to confirm unit norms)
- [ ] Add 3 additional unit vectors at different positions (e.g., one-hot at positions 1, 2, 3 — orthogonal to A and B for sanity)
- [ ] INSERT all 5 vectors via parameterized SQL with blob bind (using `f.to_le_bytes()` flat-mapped per `embeddings.rs:96-98` — F-001 confirms byte-format match with sqlite-vec's MATCH bind input)
- [ ] Run KNN: `SELECT rowid, distance FROM vec_test WHERE embedding MATCH ? ORDER BY distance LIMIT 3` with A as the probe
- [ ] **Self-match assertion** (single ownership at Step 2 — per architect review; not duplicated at Step 3): query A against itself → top result rowid = A's rowid, distance < 1e-4
- [ ] Capture (rowid, distance) tuples for B and other inserted vectors for Step 3 analysis

Expected files: `examples/sqlite_vec_smoke.rs` (continued; same transient file)

### Step 3 [PRIMARY]: Distance metric identification (AC3)
- [ ] From the KNN results in Step 2: extract the (A, B) pair's returned `distance` value
- [ ] Compute reference values for unit vectors with `dot(A, B) = 0.5`:
  - cosine_distance = `1 - dot = 0.5`
  - L2_distance = `sqrt(2 - 2·dot) = sqrt(1.0) = 1.0`
- [ ] **Branch on result**:
  - Case 1 — `|returned_distance - 0.5| < 1e-4` → metric = **cosine** → outcome candidate **PASS**
  - Case 2 — `|returned_distance - 1.0| < 1e-4` → metric = **L2** → outcome candidate **PASS_WITH_CONDITIONS** (codex-proxy notes column-declaration override `distance_metric=cosine` is documented; spike additionally verifies whether this override is available at runtime)
  - Case 3 — neither matches within 1e-4 → metric = **indeterminate** → outcome **FAIL** with caveat reason "metric identity indeterminate — returned distance {X} matches neither cosine (0.5) nor L2 (1.0) for the probe pair (dot=0.5); investigation required (possible: L2-squared, dot-product distance, or sqlite-vec internal change)" (per dep-analyst review — this previously had no fallback branch)
- [ ] If Case 2: additionally test the override path — `CREATE VIRTUAL TABLE vec_test_cos USING vec0(embedding float[384] distance_metric=cosine)`, INSERT same vectors, query A against B; assert returned distance ≈ 0.5 (cosine) within 1e-4. If override works → caveat severity ACCEPTABLE, trigger_fires true. If override fails (column declaration syntax not accepted, or distance still returns L2) → caveat severity BLOCKER, trigger_fires false.
- [ ] Capture all observed values (returned distances, override test result if applicable, sqlite-vec version, rusqlite version, SQLite bundled version, OS/arch, rustc version, exact SQL queries used) for Step 5's outcome record

Expected files: `examples/sqlite_vec_smoke.rs`

### Step 4: Revert temp deps/files + final validation (AC5, AC6)
- [ ] Remove `sqlite-vec = "=0.1.9"` from `Cargo.toml` `[dev-dependencies]`
- [ ] Delete `examples/sqlite_vec_smoke.rs`
- [ ] Run `cargo build --release --locked` (per codex-proxy — `--locked` ensures Cargo.lock is internally consistent post-revert; per dep-analyst, unrelated transitive dep updates are acceptable, but `--locked` catches accidental state corruption)
- [ ] Run `cargo build --release` (without `--locked`) to regenerate Cargo.lock cleanly without sqlite-vec entry
- [ ] Confirm `cargo build --release` + `cargo test` exit 0
- [ ] Confirm `Cargo.lock` has no `sqlite-vec` entry: `grep -c "name = \"sqlite-vec\"" Cargo.lock` returns 0
- [ ] Run AC6 git-diff assertion (per architect — `merge-base` anchor, not `HEAD~N`):
  ```
  git diff --name-only $(git merge-base HEAD main)..HEAD -- src/
  ```
  Output MUST be empty. If output is non-empty → spike failed scope check; investigate.
- [ ] At this point working tree is clean except for evidence captured in Step 3 (held in operator's notes / shell scratchpad — not yet committed); next step writes the outcome record from this captured evidence.

Expected files: `Cargo.toml` (revert to pre-spike), `Cargo.lock` (revert — regenerated), `examples/sqlite_vec_smoke.rs` (deleted). NO file in `src/`.

### Step 5: Write outcome record + commit single final state (AC4, AC7)
*(Renumbered AC: AC4 = outcome-record content; AC7 = single-commit final state. Outcome record commit is **last**, per codex-proxy — record must describe final mergeable state, not an intermediate.)*

- [ ] Create `docs/spikes/sqlite-vec-compat.md` with frontmatter (per codex-proxy ADR/MADR enrichment + challenger Phase 2 P4 structured caveats):
  ```yaml
  ---
  id: "sqlite-vec-compat"
  type: spike
  status: accepted | failed | proposed
  date: 2026-04-28
  spike_for: F-001  # / BL-007
  outcome: PASS | PASS_WITH_CONDITIONS | FAIL
  decision_drivers:
    - "BL-012 vector.rs adoption needs metric identity for score normalization design"
    - "BL-002 Reflection consolidation defers on this outcome"
  environment:
    sqlite_vec_version: "=0.1.9"
    rusqlite_version: "0.39"
    sqlite_version: "3.51.3"
    rust_toolchain: "<rustc -V output verbatim>"
    target_triple: "aarch64-apple-darwin"
    os: "macOS <version>"
  caveats:
    - description: "<freeform>"
      severity: BLOCKER | ACCEPTABLE | INFORMATIONAL
      trigger_fires: true | false
  ---
  ```
- [ ] Body sections:
  - **Question** — one-line restatement of spike's purpose
  - **Context** — why this spike was needed (link to F-001 analysis + 028 conclusion)
  - **Method** — Steps 1–4 verbatim (or cite plan 018)
  - **Environment** — verbatim copy of frontmatter `environment:` block + exact `cargo run --example sqlite_vec_smoke` invocation
  - **Evidence** — captured tuples from Step 2 + Step 3 (returned distances for A vs B, self-match distance, all 5 inserted vectors' KNN positions, exact SQL queries used, override test result if applicable)
  - **Distance metric finding** (renamed from "Identified metric" per codex) — cosine | L2 | indeterminate, with discriminating evidence cited (returned `distance` vs computed cosine vs computed L2)
  - **RRF compatibility analysis** — clarify per dep-analyst: current `src/core/search.rs` RRF is **rank-based**, NOT distance-based, so spike outcome does not affect current code. The metric matters for BL-012 adoption design (when sqlite-vec actually replaces brute-force `search_vector`) — recommend either explicit `distance_metric=cosine` override in `CREATE VIRTUAL TABLE` (if Case 2) OR `1 - distance/2` score conversion in BL-012's score-normalization step (if Case 1)
  - **Recommendation** — adopt / adopt-with-caveats / defer; if PASS or PASS_WITH_CONDITIONS-trigger_fires-true: explicit pointer "BL-002 Reflection consolidation trigger fires; operator may schedule" + recommend filing BL-011 (Linux x86_64 CI verification) and BL-012 (vector.rs adoption with bones-pattern adapter)
  - **Consequences** — per ADR convention: enabling consequences (this unlocks X), risk consequences (this commits us to Y dep), neutral consequences (this captures Z baseline for future)
- [ ] Final state at this point: working tree clean except for `docs/spikes/sqlite-vec-compat.md` (new) — confirm via `git status`
- [ ] Make a **single commit** containing exactly this one file:
  ```
  git add docs/spikes/sqlite-vec-compat.md
  git commit -m "spike(F-001/BL-007): sqlite-vec compatibility outcome — <PASS|PASS_WITH_CONDITIONS|FAIL>"
  ```
  (Per architect — exactly one commit; per codex — outcome record is the only artifact crossing the spike branch's merge boundary)
- [ ] Re-run AC6 assertion to confirm `git diff --name-only $(git merge-base HEAD main)..HEAD -- src/` is still empty after the outcome-record commit

Expected files: `docs/spikes/sqlite-vec-compat.md` (new — committed; **only artifact surviving spike branch merge**)

### Spike branch merge (post-Step 5)
- [ ] Merge `spike/018-sqlite-vec` back to base branch via squash merge (so the dep-add + smoke + revert intermediate commits collapse to a single squashed commit on base, but only `docs/spikes/sqlite-vec-compat.md` shows in the diff because Steps 1-4 reverted everything else):
  ```
  git checkout <base>  # e.g., feature/v0.0.1-rebuild
  git merge --squash spike/018-sqlite-vec
  git commit -m "spike(F-001/BL-007): sqlite-vec compatibility — outcome record"
  ```
- [ ] Confirm base branch's `git status` shows only `docs/spikes/sqlite-vec-compat.md` added (no Cargo.toml, no Cargo.lock, no examples/, no src/ touched)
- [ ] Spike branch can be deleted post-merge

## Acceptance Criteria

### AC1: Build + auto-extension registration succeeds on macOS arm64
- `cargo build --example sqlite_vec_smoke` exits 0 on macOS arm64 with `sqlite-vec = "=0.1.9"` in `[dev-dependencies]`
- `cargo run --example sqlite_vec_smoke` runs to completion without panicking on the `unsafe { sqlite3_auto_extension(...) }` call or subsequent `Connection::open_in_memory()`
- `SELECT vec_version()` returns a non-empty version string matching `=0.1.9` (or `v0.1.9` with prefix)
- macOS arm64 only — Linux x86_64 CI verification is intentionally **deferred** to BL-011 follow-up (see § Decisions not implemented). Adoption BL (BL-012) does not reach DONE until both platform records exist.
- Prerequisites: Rust ≥ 1.80 stable, Xcode Command Line Tools (cc/clang for sqlite-vec.c compile), crates.io network access

### AC2: vec0 KNN + self-match returns expected rows
- `CREATE VIRTUAL TABLE vec_test USING vec0(embedding float[384])` succeeds
- 5 INSERT statements (A + B + 3 orthogonal one-hots) succeed
- KNN query returns exactly 3 rows ordered by distance ascending
- **Self-match probe**: A as both insert and probe → top result is A's rowid with distance < 1e-4
- All returned distances are non-negative finite floats

### AC3 [PRIMARY]: Distance metric identified unambiguously OR FAIL with reason
- Constructed pair `A=[1,0,...]`, `B=[0.5, sqrt(0.75), 0,...]` is verified to have `dot(A,B) = 0.5` and `||A||=||B||=1.0` (within 1e-6) before INSERT
- Returned `distance` for (A, B) probe falls into exactly one of three branches:
  - cosine: |distance − 0.5| < 1e-4
  - L2: |distance − 1.0| < 1e-4
  - indeterminate: neither — outcome = FAIL with caveat reason cited
- If Case 2 (L2): override test against `vec0(... distance_metric=cosine)` runs; assertion result recorded in outcome
- The matching metric (or indeterminate flag) is recorded in the outcome record's frontmatter `outcome` + `Distance metric finding` body section

### AC4: Outcome record at `docs/spikes/sqlite-vec-compat.md` complete + truthful
- File exists at `docs/spikes/sqlite-vec-compat.md` (new)
- Frontmatter has all required fields: `id`, `type`, `status`, `date`, `spike_for`, `outcome`, `decision_drivers`, `environment` (sqlite_vec_version, rusqlite_version, sqlite_version, rust_toolchain, target_triple, os), `caveats[]` (each caveat: description + severity + trigger_fires)
- `outcome` ∈ {PASS, PASS_WITH_CONDITIONS, FAIL}
- `caveats[]` non-empty if outcome = PASS_WITH_CONDITIONS or FAIL
- Body has all sections per codex-proxy ADR enrichment: Question, Context, Method, Environment, Evidence, Distance metric finding, RRF compatibility analysis, Recommendation, Consequences
- Outcome content matches actual test results (no aspirational PASS without the smoke run going green)
- Explicit BL-002 trigger-fire status is stated (fires iff outcome = PASS, OR PASS_WITH_CONDITIONS where every caveat has `trigger_fires: true`)
- BL-011 + BL-012 follow-up filings recommended in Recommendation section if outcome ≠ FAIL

### AC5: Cargo.toml + Cargo.lock reverted; cargo build --locked passes pre-final-commit
- Final `Cargo.toml` has no `sqlite-vec` entry (verified by absence of `sqlite-vec` line in `[dev-dependencies]`)
- `grep -c "name = \"sqlite-vec\"" Cargo.lock` returns 0
- `cargo build --release --locked` exits 0 (catches accidental Cargo.lock state corruption from revert)
- `cargo build --release` (regen) + `cargo test` exit 0
- Note (per dep-analyst): Cargo.lock may reflect updated transitive dep resolutions for crates unrelated to sqlite-vec (legitimate, not a violation)

### AC6: No src/ modified by this spike (audit)
- `git diff --name-only $(git merge-base HEAD main)..HEAD -- src/` returns empty output
- `git status` post-Step-5 shows clean working tree (or only `docs/spikes/sqlite-vec-compat.md` if mid-Step-5 between write and commit)

### AC7: Single final commit on spike branch contains only outcome record (per codex + architect)
- Spike branch's HEAD commit changes exactly 1 file: `docs/spikes/sqlite-vec-compat.md` (new)
- Squash merge to base branch produces a single commit also containing exactly that 1 file (Cargo.toml, Cargo.lock, examples/, src/ all unchanged on base)

## Decisions not implemented

These decisions surfaced during F-001 analyze + plan-review phases but are intentionally **NOT** in this spike's scope:

- **Linux x86_64 CI verification**: per challenger Phase 2 P3 + dep-analyst, this spike runs only on macOS arm64. **A follow-up BL (suggested ID: BL-011) MUST be filed before BL-012 (adoption) closes** — adoption BL does not reach DONE until both platform records exist.
- **`vector.rs` adoption / replacement**: this spike does NOT adopt sqlite-vec into `src/`. If outcome = PASS or PASS_WITH_CONDITIONS, a separate adoption BL (suggested ID: BL-012) is filed; the adoption BL writes real integration tests, schema v6 migration with `vec0` virtual table, `vector.rs` refactor (free-function form per 028 Topic 1 BL-010), and proper RRF score conversion given identified metric.
- **`search.rs` free-functions refactor (Wave 2 BL D+E, BL-010 in `docs/backlog/unscheduled/`)**: per 028 conclusion this is co-committed with mcp_tools defect fix. Coupling to sqlite-vec adoption is via BL-012 (the adoption BL builds atop the search.rs refactor). Not in this spike. (Architect review noted this missing item.)
- **Performance benchmarks**: per gemini-proxy reframing in F-001 analysis, perf measurement deferred — at v0.0.1 scale (~200 facts) any reasonable in-process solution adequate. Real perf testing triggers when corpus approaches 1k–5k. Not in this spike.
- **Adapter pattern (bones-style)**: codex-proxy recommended `bones-sqlite-vec` adapter pattern (isolate unsafe in one module + graceful fallback via env var). This is the right adoption pattern but is BL-012's job. Not in this spike.
- **RRF formula refactor for adoption-time metric**: if AC3 identifies L2 (Case 2), the spike's outcome record states the consequence; actual RRF score-conversion design lives in BL-012's adoption plan. Not in this spike.

## Plan-review summary (2026-04-28)

Reviewed by 4-agent team `F-001-plan-review`:

| Agent | Angle | Verdict |
|---|---|---|
| architect | Step decomposition + AC verification logic | CONDITIONAL APPROVE — 2 Must Fix + 3 Consider; addressed inline |
| dependency-analyst | Step ordering + parallel feasibility + lifecycle | 2 Must Fix + 1 fact correction; addressed inline |
| codex-proxy | Production spike pattern check | 2 Must Fix + 5 Consider; addressed inline |
| gemini-proxy | AC3 metric-identification rigor | 1 Must Fix (test-vector construction); addressed inline |

**7 Must Fix items applied:**

1. AC3 test-vector construction (gemini): one-hot replaced with explicit `A=[1,0,...]`, `B=[0.5, sqrt(0.75), 0,...]` for verified `dot=0.5`
2. Step 5 (now Step 5) commit granularity (architect): exactly one commit containing only outcome record
3. AC6 audit assertion (architect): replaced `HEAD~<N>` with `git merge-base HEAD main` anchor
4. AC3 indeterminate case (dep-analyst): Case 3 fallback explicit (FAIL with reason)
5. Spike branch lifecycle (dep-analyst): explicit `spike/018-sqlite-vec` + squash-merge semantics
6. Smoke binary placement (codex): `spike/` is not Cargo target; moved to `examples/sqlite_vec_smoke.rs` with `cargo run --example` invocation
7. Outcome record commit timing (codex): commit AFTER revert + `cargo build --locked`, not before

**1 fact correction (dep-analyst):** Goal + AC3 RRF compat section reframed — current `src/core/search.rs` RRF is rank-based, not distance-based; spike doesn't corrupt anything because AC6 keeps `src/` untouched. Metric identity matters for BL-012 adoption, not for current code.

**Key Consider items applied:** Xcode CLT prereq + Rust ≥ 1.80 added to AC1; ADR-style outcome record fields (status, decision_drivers, environment, consequences) added to AC4; "Identified metric" renamed to "Distance metric finding"; out-of-scope adds search.rs Wave 2 explicit exclusion; AC5 Cargo.lock unrelated-transitive-update note; self-match assertion ownership clarified as Step 2 only.

**Codex external-evidence note (informational):** sqlite-vec docs state `vec0` defaults to **L2** with `distance_metric=cosine` column-declaration override available. Spike is still valuable (verifies `=0.1.9` specifically) and the override path is now part of AC3 Case 2 verification. Expected outcome is therefore **PASS_WITH_CONDITIONS** with caveat severity=ACCEPTABLE, trigger_fires=true, plus a recommendation to BL-012 to use `distance_metric=cosine` in `CREATE VIRTUAL TABLE`.

## Next Steps

→ `/ae:work docs/plans/018-sqlite-vec-compat-spike.md` — plan is `status: reviewed`, ready for execution. Spike runs on `spike/018-sqlite-vec` branch, ~1 day (S; S+ if AC3 Case 3 fires).

→ Spike outcome handling:
   - **PASS** (cosine confirmed) → file BL-011 (Linux CI verification) + BL-012 (vector.rs adoption with bones-pattern adapter, distance/2 score conversion); BL-002 trigger fires.
   - **PASS_WITH_CONDITIONS** (L2 default but `distance_metric=cosine` column override works — codex-proxy expected case) → file BL-011 + BL-012 with override pattern as constraint in BL-012's `CREATE VIRTUAL TABLE` design.
   - **FAIL** (Case 3 indeterminate or registration crash or override unavailable when needed) → BL-002 stays deferred; BL-010 (search.rs free-fn refactor) proceeds against existing brute-force `search_vector` unchanged; mengdie postpones sqlite-vec until either v1.0 stable OR alternative path (LanceDB, etc.) opens.
